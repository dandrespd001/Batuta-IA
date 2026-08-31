//! Ejecutor cuya configuración completa procede de manifests ya verificados.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use batuta_manifest::ProviderManifest;
use serde::Deserialize;

use crate::executor::truncate_utf8;
use crate::{
    ExecutionProfileV1, HarnessExecutor, InvocationFailure, InvocationRequestV2,
    NormalizedInvocationResult, RunContext, TokenUsage, build_env, materialize, read_stderr,
    resolve_argv, run,
};

#[derive(Debug)]
struct ResolvedManifest {
    manifest: ProviderManifest,
    program: PathBuf,
}

/// Adaptador local precargado y verificado antes de admitir runs.
#[derive(Debug)]
pub struct ManifestHarnessExecutor {
    profile: ExecutionProfileV1,
    run_root: PathBuf,
    manifests: Vec<ResolvedManifest>,
}

impl ManifestHarnessExecutor {
    /// Carga todos los manifests, comprueba hash y pin de versión y deja una
    /// configuración inmutable para las llamadas posteriores.
    ///
    /// # Errors
    ///
    /// Si el perfil está obsoleto, un manifest no tiene hash, el ejecutable no
    /// coincide o la sonda no demuestra el pin declarado.
    pub fn open(
        manifest_dir: &Path,
        profile: ExecutionProfileV1,
        run_root: PathBuf,
    ) -> Result<Self, ManifestExecutorError> {
        profile
            .validate()
            .map_err(|error| ManifestExecutorError::Profile(error.to_string()))?;
        let loaded = ProviderManifest::load_dir(manifest_dir)
            .map_err(|error| ManifestExecutorError::Manifest(error.to_string()))?;
        if loaded.is_empty() {
            return Err(ManifestExecutorError::Manifest(
                "trusted manifest directory is empty".to_string(),
            ));
        }
        let mut manifests = Vec::with_capacity(loaded.len());
        for manifest in loaded {
            if manifest.executable().sha256().is_none() {
                return Err(ManifestExecutorError::Manifest(format!(
                    "manifest '{}' must pin executable.sha256",
                    manifest.origin().display()
                )));
            }
            let program = manifest
                .verify_executable()
                .map_err(|error| ManifestExecutorError::Manifest(error.to_string()))?;
            verify_version(&manifest, &program, &profile)?;
            manifests.push(ResolvedManifest { manifest, program });
        }
        Ok(Self {
            profile,
            run_root,
            manifests,
        })
    }

    fn resolve(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<(&ProviderManifest, &Path, &batuta_manifest::ModelEntry), ManifestExecutorError>
    {
        for resolved in &self.manifests {
            let manifest = &resolved.manifest;
            if manifest.id().as_str() != request.route.harness() {
                continue;
            }
            for model in manifest.models() {
                let provider = model.route_provider().unwrap_or(manifest.id().as_str());
                let model_matches = model.id().as_str() == request.route.model()
                    || model.route_model().as_str() == request.route.model();
                let revision_matches = request
                    .route
                    .revision()
                    .is_none_or(|revision| revision == manifest.executable().version_pin());
                if provider == request.route.provider() && model_matches && revision_matches {
                    return Ok((manifest, resolved.program.as_path(), model));
                }
            }
        }
        Err(ManifestExecutorError::RouteNotFound(
            request.route.to_string(),
        ))
    }
}

impl HarnessExecutor for ManifestHarnessExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, crate::ExecutorError> {
        request.validate()?;
        self.profile
            .validate()
            .map_err(|error| crate::ExecutorError::Configuration(error.to_string()))?;
        if request.max_output_bytes > self.profile.max_stdout_bytes() {
            return Err(crate::ExecutorError::Configuration(
                "request max_output_bytes exceeds the active execution profile".to_string(),
            ));
        }
        let (manifest, program, model) = self
            .resolve(request)
            .map_err(|error| crate::ExecutorError::Configuration(error.to_string()))?;
        let run_dir = self.run_root.join(&request.run_id);
        std::fs::create_dir_all(&run_dir).map_err(|error| {
            crate::ExecutorError::Configuration(format!(
                "cannot create run directory '{}': {error}",
                run_dir.display()
            ))
        })?;
        let context = RunContext {
            model: model.id().clone(),
            route_model: model.route_model().clone(),
            route_provider: model.route_provider().map(str::to_string),
            workdir: self.profile.workdir().to_path_buf(),
            run_dir,
            prompt: request.objective.clone(),
            token: request.run_id.clone(),
            write_mode: request.task.write_mode(),
            reasoning_effort: request.task.reasoning_effort(),
        };
        materialize(manifest, &context).map_err(crate::ExecutorError::Exec)?;
        let argv = resolve_argv(manifest, &context).map_err(crate::ExecutorError::Exec)?;
        let cwd = match manifest.invoke().workdir() {
            "worktree" => self.profile.workdir().to_path_buf(),
            "run_dir" => context.run_dir.clone(),
            other => {
                return Err(crate::ExecutorError::Configuration(format!(
                    "unsupported trusted invoke.workdir '{other}'"
                )));
            }
        };
        let outcome = run(
            program,
            &argv,
            &build_env(manifest.env()),
            &cwd,
            Duration::from_millis(request.timeout_ms),
        )
        .map_err(crate::ExecutorError::Exec)?;
        let bounded_stderr = truncate_utf8(outcome.stderr, self.profile.max_stderr_bytes())?;
        let observed = observed_result(&bounded_stderr);
        let failure = if outcome.timed_out {
            Some(InvocationFailure::Timeout)
        } else if let Some(observed) = &observed {
            observed.failure.map(Into::into)
        } else if outcome.exit_code == Some(0) {
            None
        } else {
            Some(InvocationFailure::Permanent)
        };
        let provenance = observed
            .as_ref()
            .and_then(|item| item.provenance.clone())
            .or_else(|| {
                manifest
                    .provenance_pattern()
                    .and_then(|pattern| read_stderr(&bounded_stderr, pattern).ok())
                    .map(|observed| format!("{}/{}", observed.provider(), observed.model()))
            });
        let usage = observed
            .as_ref()
            .map_or_else(TokenUsage::default, |item| TokenUsage {
                input_tokens: item.input_tokens,
                output_tokens: item.output_tokens,
            });
        Ok(NormalizedInvocationResult {
            output: truncate_utf8(outcome.stdout, request.max_output_bytes)?,
            usage,
            latency_ms: u64::try_from(outcome.duration.as_millis()).unwrap_or(u64::MAX),
            provenance,
            manifest_hash: Some(format!("sha256:{}", manifest.source_sha256())),
            failure,
        })
    }
}

fn verify_version(
    manifest: &ProviderManifest,
    program: &Path,
    profile: &ExecutionProfileV1,
) -> Result<(), ManifestExecutorError> {
    let outcome = run(
        program,
        manifest.executable().version_probe(),
        &build_env(manifest.env()),
        profile.workdir(),
        Duration::from_millis(profile.termination_grace_ms()),
    )
    .map_err(|error| ManifestExecutorError::Version(error.to_string()))?;
    let pin = manifest.executable().version_pin();
    if outcome.timed_out
        || outcome.exit_code != Some(0)
        || (!outcome.stdout.contains(pin) && !outcome.stderr.contains(pin))
    {
        return Err(ManifestExecutorError::Version(format!(
            "executable '{}' did not prove version pin '{pin}'",
            program.display()
        )));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedResultV1 {
    failure: Option<ObservedFailureV1>,
    input_tokens: u64,
    output_tokens: u64,
    provenance: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case", deny_unknown_fields)]
enum ObservedFailureV1 {
    RateLimited { retry_after_ms: Option<u64> },
    Quota,
    Authentication,
    Balance,
    Transient,
    Timeout,
    Permanent,
}

impl From<ObservedFailureV1> for InvocationFailure {
    fn from(value: ObservedFailureV1) -> Self {
        match value {
            ObservedFailureV1::RateLimited { retry_after_ms } => {
                Self::RateLimited { retry_after_ms }
            }
            ObservedFailureV1::Quota => Self::Quota,
            ObservedFailureV1::Authentication => Self::Authentication,
            ObservedFailureV1::Balance => Self::Balance,
            ObservedFailureV1::Transient => Self::Transient,
            ObservedFailureV1::Timeout => Self::Timeout,
            ObservedFailureV1::Permanent => Self::Permanent,
        }
    }
}

fn observed_result(stderr: &str) -> Option<ObservedResultV1> {
    const PREFIX: &str = "BATUTA_RESULT_V1:";
    let encoded = stderr
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(PREFIX))?;
    serde_json::from_str(encoded).ok()
}

/// Error de precarga, antes de exponer el ejecutor al coordinador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestExecutorError {
    /// Perfil inactivo u obsoleto.
    Profile(String),
    /// Manifest o hash inválido.
    Manifest(String),
    /// La sonda no demostró el pin.
    Version(String),
    /// La ruta no existe exactamente en los manifests cargados.
    RouteNotFound(String),
}

impl fmt::Display for ManifestExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profile(message) | Self::Manifest(message) | Self::Version(message) => {
                f.write_str(message)
            }
            Self::RouteNotFound(route) => {
                write!(f, "exact route '{route}' is absent from trusted manifests")
            }
        }
    }
}

impl std::error::Error for ManifestExecutorError {}
