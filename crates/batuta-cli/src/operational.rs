//! API operativa K4 compartida por CLI y TUI.

use std::collections::BTreeSet;

use batuta_exec::{
    ExecutionProfileDraftV1, ExecutorError, HarnessExecutor, InvocationRequestV2,
    NormalizedInvocationResult,
};
use batuta_routing::{
    ExecutionGrantDraftV1, ExecutionGrantV1, ExecutionProfileProposalV1, ExecutionProfileStatusV1,
    ExecutionProfileStore, GrantStatus, GrantStore, RunCoordinator, RunCoordinatorError,
    RunRequestV2, RunStatusV2, StateStore,
};
use serde::de::DeserializeOwned;

use crate::{ApiErrorV2, ApiResponseV2, Layout};

/// Operaciones que no requieren abrir un harness.
#[derive(Debug, Clone, Copy)]
pub struct OperationalApi<'a> {
    layout: &'a Layout,
    now_secs: u64,
}

impl<'a> OperationalApi<'a> {
    /// Fija una disposición y un instante para toda la operación.
    pub const fn new(layout: &'a Layout, now_secs: u64) -> Self {
        Self { layout, now_secs }
    }

    /// Valida manifest, rutas y confirmación antes de sellar un grant.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, el borrador no valida, su manifest no es el
    /// activo, contiene rutas futuras o no puede persistirse.
    pub fn grant_create_json(
        &self,
        input: &str,
        confirmed: bool,
    ) -> Result<ApiResponseV2<ExecutionGrantV1>, ApiErrorV2> {
        require_confirmation(confirmed)?;
        let draft: ExecutionGrantDraftV1 = parse_json(input, "grant")?;
        let snapshot = StateStore::open(self.layout.state())
            .load()
            .map_err(state_error)?;
        let active_hash = snapshot.manifest.manifest_hash().map_err(state_error)?;
        if draft.manifest_hash != active_hash {
            return Err(ApiErrorV2::new(
                "stale_manifest",
                "manifest_hash",
                "grant draft was not based on the active StateManifestV2",
                serde_json::json!({
                    "expected": active_hash,
                    "actual": draft.manifest_hash,
                }),
            ));
        }
        let current_routes = snapshot
            .components
            .catalog
            .routes
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let missing = draft
            .routes
            .difference(&current_routes)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(ApiErrorV2::new(
                "grant_route_not_current",
                "routes",
                "a grant can only contain exact routes present in its base manifest",
                serde_json::json!({"routes": missing}),
            ));
        }
        let grant = draft.seal().map_err(|error| {
            ApiErrorV2::new(
                "invalid_grant",
                "grant",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
        grant.validate_at(self.now_secs).map_err(|error| {
            ApiErrorV2::new(
                "grant_not_active",
                "issued_at",
                error.to_string(),
                serde_json::json!({"now": self.now_secs}),
            )
        })?;
        GrantStore::open(self.layout.grants())
            .append(&grant)
            .map_err(grant_error)?;
        Ok(response(grant))
    }

    /// Consulta el documento inmutable y su primera revocación.
    ///
    /// # Errors
    ///
    /// Si el ID o el documento persistido son inválidos o no existen.
    pub fn grant_status(&self, id: &str) -> Result<ApiResponseV2<GrantStatus>, ApiErrorV2> {
        GrantStore::open(self.layout.grants())
            .status(id)
            .map(response)
            .map_err(grant_error)
    }

    /// Revoca append-only con actor fijo de la superficie local.
    ///
    /// # Errors
    ///
    /// Si falta confirmación o no puede validarse o persistirse la revocación.
    pub fn grant_revoke(
        &self,
        id: &str,
        confirmed: bool,
    ) -> Result<ApiResponseV2<GrantStatus>, ApiErrorV2> {
        require_confirmation(confirmed)?;
        let store = GrantStore::open(self.layout.grants());
        store
            .revoke(id, self.now_secs, "operator")
            .map_err(grant_error)?;
        store.status(id).map(response).map_err(grant_error)
    }

    /// Sella un borrador y crea staging con ID generado por contenido.
    ///
    /// # Errors
    ///
    /// Si el JSON o perfil son inválidos, o staging no puede persistirse.
    pub fn profile_import_json(
        &self,
        input: &str,
    ) -> Result<ApiResponseV2<ExecutionProfileProposalV1>, ApiErrorV2> {
        let draft: ExecutionProfileDraftV1 = parse_json(input, "profile")?;
        let profile = batuta_exec::ExecutionProfileV1::seal(draft).map_err(|error| {
            ApiErrorV2::new(
                "invalid_execution_profile",
                "profile",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
        let suffix = profile
            .profile_hash()
            .strip_prefix("sha256:")
            .unwrap_or(profile.profile_hash())
            .chars()
            .take(12)
            .collect::<String>();
        let id = format!("profile-{}-{suffix}", self.now_secs);
        self.profile_store()
            .stage(&id, self.now_secs, profile)
            .map(response)
            .map_err(profile_error)
    }

    /// Lee perfil activo y propuestas sin activar nada.
    ///
    /// # Errors
    ///
    /// Si un perfil o propuesta no valida o no puede leerse.
    pub fn profile_status(&self) -> Result<ApiResponseV2<ExecutionProfileStatusV1>, ApiErrorV2> {
        self.profile_store()
            .status()
            .map(response)
            .map_err(profile_error)
    }

    /// Publica el perfil sólo si ID, base y confirmación coinciden.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, existe conflicto CAS o falla la publicación.
    pub fn profile_apply(
        &self,
        proposal: &str,
        expected_hash: &str,
        confirmed: bool,
    ) -> Result<ApiResponseV2<batuta_exec::ExecutionProfileV1>, ApiErrorV2> {
        if !confirmed {
            return Err(confirmation_error());
        }
        self.profile_store()
            .apply(proposal, expected_hash, true)
            .map(response)
            .map_err(profile_error)
    }

    fn profile_store(&self) -> ExecutionProfileStore {
        ExecutionProfileStore::open(self.layout.execution_profile(), self.layout.leases())
    }
}

/// Inicia una corrida con el ejecutor ya resuelto por una frontera confiable.
///
/// # Errors
///
/// Si la petición, manifest, grant, presupuesto, selección o persistencia fallan.
pub fn run_json<E: HarnessExecutor + ?Sized>(
    layout: &Layout,
    executor: &E,
    input: &str,
) -> Result<ApiResponseV2<RunStatusV2>, ApiErrorV2> {
    let request: RunRequestV2 = parse_json(input, "request")?;
    RunCoordinator::open(layout.root().to_path_buf(), executor)
        .execute(request)
        .map(response)
        .map_err(run_error)
}

/// Continúa una corrida desde su journal y checkpoint durable.
///
/// # Errors
///
/// Si el estado no valida, la sonda aún no vence o la continuación falla.
pub fn run_resume<E: HarnessExecutor + ?Sized>(
    layout: &Layout,
    executor: &E,
    id: &str,
) -> Result<ApiResponseV2<RunStatusV2>, ApiErrorV2> {
    RunCoordinator::open(layout.root().to_path_buf(), executor)
        .resume(id)
        .map(response)
        .map_err(run_error)
}

/// Lee una corrida sin exigir perfil, manifest de harness ni ejecutable activo.
///
/// # Errors
///
/// Si el ID o el estado durable no existen o no validan.
pub fn run_status(layout: &Layout, id: &str) -> Result<ApiResponseV2<RunStatusV2>, ApiErrorV2> {
    RunCoordinator::open(layout.root().to_path_buf(), &StatusOnlyExecutor)
        .status(id)
        .map(response)
        .map_err(run_error)
}

fn response<T>(data: T) -> ApiResponseV2<T> {
    ApiResponseV2 {
        schema_version: 2,
        data,
    }
}

fn parse_json<T: DeserializeOwned>(input: &str, document: &str) -> Result<T, ApiErrorV2> {
    serde_json::from_str(input).map_err(|error| {
        ApiErrorV2::new(
            "invalid_json",
            document,
            error.to_string(),
            serde_json::json!({"document": document}),
        )
    })
}

fn require_confirmation(confirmed: bool) -> Result<(), ApiErrorV2> {
    if confirmed {
        Ok(())
    } else {
        Err(confirmation_error())
    }
}

fn confirmation_error() -> ApiErrorV2 {
    ApiErrorV2::new(
        "confirmation_required",
        "confirm",
        "the operation requires explicit --confirm",
        serde_json::Value::Null,
    )
}

fn state_error(error: impl std::fmt::Display) -> ApiErrorV2 {
    ApiErrorV2::new(
        "routing_state_required",
        "state.manifest",
        error.to_string(),
        serde_json::Value::Null,
    )
}

fn grant_error(error: impl std::fmt::Display) -> ApiErrorV2 {
    ApiErrorV2::new(
        "grant_error",
        "grant",
        error.to_string(),
        serde_json::Value::Null,
    )
}

fn profile_error(error: impl std::fmt::Display) -> ApiErrorV2 {
    ApiErrorV2::new(
        "execution_profile_error",
        "profile",
        error.to_string(),
        serde_json::Value::Null,
    )
}

fn run_error(error: RunCoordinatorError) -> ApiErrorV2 {
    let message = error.to_string();
    let (code, field, details) = match error {
        RunCoordinatorError::Invalid(_) => ("invalid_request", "request", serde_json::Value::Null),
        RunCoordinatorError::AlreadyExists(id) => {
            ("run_already_exists", "id", serde_json::json!({"id": id}))
        }
        RunCoordinatorError::Grant(_) => ("grant_error", "grant_id", serde_json::Value::Null),
        RunCoordinatorError::Route(_) => ("route_unavailable", "routing", serde_json::Value::Null),
        RunCoordinatorError::Budget(_) => {
            ("budget_exceeded", "grant.limits", serde_json::Value::Null)
        }
        RunCoordinatorError::State(_) => (
            "routing_state_required",
            "state.manifest",
            serde_json::Value::Null,
        ),
        RunCoordinatorError::Receipt(_) => ("receipt_error", "receipt", serde_json::Value::Null),
        RunCoordinatorError::ProbeNotDue { next_action_at } => (
            "probe_not_due",
            "next_action_at",
            serde_json::json!({"next_action_at": next_action_at}),
        ),
        RunCoordinatorError::Lease(_) => ("run_busy", "id", serde_json::Value::Null),
        RunCoordinatorError::Io(_) => ("io_error", "state", serde_json::Value::Null),
        RunCoordinatorError::Json(_) => ("invalid_state", "state", serde_json::Value::Null),
    };
    ApiErrorV2::new(code, field, message, details)
}

struct StatusOnlyExecutor;

impl HarnessExecutor for StatusOnlyExecutor {
    fn invoke(
        &self,
        _request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        Err(ExecutorError::InvalidRequest)
    }
}
