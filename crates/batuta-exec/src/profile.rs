//! Perfil operativo mínimo; nunca contiene cómo invocar un proveedor.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Documento de entrada que un operador puede proponer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfileDraftV1 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Directorio de trabajo que se canonizará antes de sellar.
    pub workdir: PathBuf,
    /// Máximo absoluto de stdout conservado.
    pub max_stdout_bytes: u64,
    /// Máximo absoluto de stderr conservado.
    pub max_stderr_bytes: u64,
    /// Gracia máxima de terminación antes de matar el árbol.
    pub termination_grace_ms: u64,
}

/// Perfil activo, canónico, cerrado y sellado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfileV1 {
    schema_version: u16,
    workdir: PathBuf,
    max_stdout_bytes: u64,
    max_stderr_bytes: u64,
    termination_grace_ms: u64,
    profile_hash: String,
}

#[derive(Serialize)]
struct ProfileBody<'a> {
    schema_version: u16,
    workdir: &'a Path,
    max_stdout_bytes: u64,
    max_stderr_bytes: u64,
    termination_grace_ms: u64,
}

impl ExecutionProfileV1 {
    /// Canoniza, valida y sella un borrador.
    ///
    /// # Errors
    ///
    /// Si la versión o los límites son inválidos, o `workdir` no es un
    /// directorio existente distinto de la raíz.
    pub fn seal(draft: ExecutionProfileDraftV1) -> Result<Self, ExecutionProfileError> {
        validate_limits(&draft)?;
        let ExecutionProfileDraftV1 {
            schema_version,
            workdir,
            max_stdout_bytes,
            max_stderr_bytes,
            termination_grace_ms,
        } = draft;
        let workdir = canonical_workdir(&workdir)?;
        let mut profile = Self {
            schema_version,
            workdir,
            max_stdout_bytes,
            max_stderr_bytes,
            termination_grace_ms,
            profile_hash: String::new(),
        };
        profile.profile_hash = profile.calculate_hash()?;
        Ok(profile)
    }

    /// Revalida el directorio vigente y el sello.
    ///
    /// # Errors
    ///
    /// Si cambió el documento, el directorio desapareció o dejó de resolver al
    /// camino canónico sellado.
    pub fn validate(&self) -> Result<(), ExecutionProfileError> {
        validate_limits(&ExecutionProfileDraftV1 {
            schema_version: self.schema_version,
            workdir: self.workdir.clone(),
            max_stdout_bytes: self.max_stdout_bytes,
            max_stderr_bytes: self.max_stderr_bytes,
            termination_grace_ms: self.termination_grace_ms,
        })?;
        let canonical = canonical_workdir(&self.workdir)?;
        if canonical != self.workdir {
            return Err(ExecutionProfileError::Invalid(
                "workdir no longer resolves to the sealed canonical path".to_string(),
            ));
        }
        if self.calculate_hash()? != self.profile_hash {
            return Err(ExecutionProfileError::HashMismatch);
        }
        Ok(())
    }

    /// Directorio canónico.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Límite de stdout.
    pub const fn max_stdout_bytes(&self) -> u64 {
        self.max_stdout_bytes
    }

    /// Límite de stderr.
    pub const fn max_stderr_bytes(&self) -> u64 {
        self.max_stderr_bytes
    }

    /// Gracia de terminación.
    pub const fn termination_grace_ms(&self) -> u64 {
        self.termination_grace_ms
    }

    /// Sello canónico.
    pub fn profile_hash(&self) -> &str {
        &self.profile_hash
    }

    fn calculate_hash(&self) -> Result<String, ExecutionProfileError> {
        let bytes = serde_json::to_vec(&ProfileBody {
            schema_version: self.schema_version,
            workdir: &self.workdir,
            max_stdout_bytes: self.max_stdout_bytes,
            max_stderr_bytes: self.max_stderr_bytes,
            termination_grace_ms: self.termination_grace_ms,
        })
        .map_err(ExecutionProfileError::Json)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn validate_limits(draft: &ExecutionProfileDraftV1) -> Result<(), ExecutionProfileError> {
    if draft.schema_version != 1 {
        return Err(ExecutionProfileError::Invalid(
            "execution profile schema_version must be 1".to_string(),
        ));
    }
    if draft.max_stdout_bytes == 0 || draft.max_stderr_bytes == 0 || draft.termination_grace_ms == 0
    {
        return Err(ExecutionProfileError::Invalid(
            "execution profile limits must be strictly positive".to_string(),
        ));
    }
    Ok(())
}

fn canonical_workdir(path: &Path) -> Result<PathBuf, ExecutionProfileError> {
    let canonical = std::fs::canonicalize(path).map_err(ExecutionProfileError::Io)?;
    if !canonical.is_dir() {
        return Err(ExecutionProfileError::Invalid(
            "workdir must be an existing directory".to_string(),
        ));
    }
    if canonical.parent().is_none() {
        return Err(ExecutionProfileError::Invalid(
            "workdir cannot be the filesystem root".to_string(),
        ));
    }
    Ok(canonical)
}

/// Error al sellar o revalidar un perfil.
#[derive(Debug)]
pub enum ExecutionProfileError {
    /// Contrato inválido.
    Invalid(String),
    /// El sello no coincide.
    HashMismatch,
    /// El directorio no se puede resolver.
    Io(std::io::Error),
    /// No se pudo construir el documento canónico.
    Json(serde_json::Error),
}

impl fmt::Display for ExecutionProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::HashMismatch => f.write_str("execution_profile_hash_mismatch"),
            Self::Io(error) => write!(f, "execution profile workdir failed: {error}"),
            Self::Json(error) => write!(f, "execution profile JSON failed: {error}"),
        }
    }
}

impl std::error::Error for ExecutionProfileError {}
