//! Estado, preview y worker de la vista Execution.

use std::collections::BTreeSet;

use batuta_contract::RouteRef;
use batuta_routing::{
    ApplicationService, ExecutionGrantDraftV1, ExecutionGrantV1, ExecutionProfileProposalV1,
    GrantLimits, GrantOperation, GrantStatus, GrantStore, RunRequestV2, StateStore,
};
use serde::{Deserialize, Serialize};

use super::TuiApp;
use crate::{ApiErrorV2, Layout, OperationalApi};

mod presentation;
mod worker;

pub use worker::{TuiExecutionJob, TuiExecutionWorker};

/// Sección activa dentro de Execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiExecutionSection {
    /// Perfil operativo, staging y diff.
    Profile,
    /// Grants y revocaciones.
    Grants,
    /// Formulario, preview y corridas.
    Runs,
}

impl TuiExecutionSection {
    const ALL: [Self; 3] = [Self::Profile, Self::Grants, Self::Runs];

    const fn title(self) -> &'static str {
        match self {
            Self::Profile => "Perfil",
            Self::Grants => "Grants",
            Self::Runs => "Runs",
        }
    }
}

/// Preview puro de una corrida; `reserved` siempre es falso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunPreviewV2 {
    /// Versión del preview.
    pub schema_version: u16,
    /// ID que se debe volver a escribir.
    pub run_id: String,
    /// Manifest exacto usado para seleccionar.
    pub manifest_hash: String,
    /// Ruta exacta seleccionada.
    pub route: RouteRef,
    /// Grant revisado.
    pub grant_id: String,
    /// Sello del grant.
    pub grant_hash: String,
    /// Presupuesto máximo, todavía no reservado.
    pub budget: GrantLimits,
    /// Deadline durable que tendría la corrida.
    pub deadline_at_ms: u64,
    /// Prueba explícita de que preview no reserva.
    pub reserved: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ExecutionPanelState {
    section: usize,
    profile_proposal: Option<ExecutionProfileProposalV1>,
    run_preview: Option<RunPreviewV2>,
    run_request_json: Option<String>,
}

impl TuiApp {
    /// Sección operativa activa.
    pub const fn execution_section(&self) -> TuiExecutionSection {
        TuiExecutionSection::ALL[self.execution.section]
    }

    /// Avanza Perfil → Grants → Runs sin cambiar estado durable.
    pub fn next_execution_section(&mut self) {
        self.execution.section = (self.execution.section + 1) % TuiExecutionSection::ALL.len();
    }

    /// Propuesta visible que puede confirmarse desde el formulario interactivo.
    pub fn execution_profile_proposal(&self) -> Option<&ExecutionProfileProposalV1> {
        self.execution.profile_proposal.as_ref()
    }

    /// Preview visible que puede confirmarse desde el formulario interactivo.
    pub fn execution_run_preview(&self) -> Option<&RunPreviewV2> {
        self.execution.run_preview.as_ref()
    }

    /// Valida el formulario cerrado y crea sólo staging.
    ///
    /// # Errors
    ///
    /// Si el JSON o perfil no validan o staging no puede persistirse.
    pub fn stage_execution_profile_json(
        &mut self,
        layout: &Layout,
        input: &str,
        now_secs: u64,
    ) -> Result<ExecutionProfileProposalV1, ApiErrorV2> {
        let proposal = OperationalApi::new(layout, now_secs)
            .profile_import_json(input)?
            .data;
        self.status = format!(
            "perfil en staging: {}; escriba ese ID para aplicar",
            proposal.id
        );
        self.execution.profile_proposal = Some(proposal.clone());
        Ok(proposal)
    }

    /// Aplica sólo cuando el texto escrito coincide exactamente con el ID.
    ///
    /// # Errors
    ///
    /// Si la confirmación no coincide o la propuesta entra en conflicto.
    pub fn apply_execution_profile(
        &mut self,
        layout: &Layout,
        proposal_id: &str,
        typed_id: &str,
        expected_hash: &str,
        now_secs: u64,
    ) -> Result<(), ApiErrorV2> {
        if proposal_id != typed_id {
            return Err(ApiErrorV2::new(
                "confirmation_mismatch",
                "confirmation",
                "typed proposal ID does not match the staged proposal",
                serde_json::json!({"expected": proposal_id, "actual": typed_id}),
            ));
        }
        OperationalApi::new(layout, now_secs).profile_apply(proposal_id, expected_hash, true)?;
        self.status = format!("perfil activo: {proposal_id}");
        Ok(())
    }

    /// Crea un grant desde un borrador cerrado tras escribir su ID exacto.
    ///
    /// # Errors
    ///
    /// Si el borrador, manifest, rutas o confirmación no son válidos.
    pub fn create_execution_grant_json(
        &mut self,
        layout: &Layout,
        input: &str,
        typed_id: &str,
        now_secs: u64,
    ) -> Result<ExecutionGrantV1, ApiErrorV2> {
        let draft: ExecutionGrantDraftV1 = serde_json::from_str(input).map_err(|error| {
            ApiErrorV2::new(
                "invalid_json",
                "grant",
                error.to_string(),
                serde_json::json!({"document": "grant"}),
            )
        })?;
        require_typed_id(&draft.id, typed_id)?;
        let grant = OperationalApi::new(layout, now_secs)
            .grant_create_json(input, true)?
            .data;
        self.status = format!("grant creado: {}", grant.id);
        Ok(grant)
    }

    /// Consulta grant y revocación sin modificar estado.
    ///
    /// # Errors
    ///
    /// Si el grant no existe o su documento no valida.
    pub fn execution_grant_status(
        &self,
        layout: &Layout,
        id: &str,
        now_secs: u64,
    ) -> Result<GrantStatus, ApiErrorV2> {
        Ok(OperationalApi::new(layout, now_secs).grant_status(id)?.data)
    }

    /// Revoca append-only tras escribir el ID exacto del grant.
    ///
    /// # Errors
    ///
    /// Si la confirmación no coincide o la revocación no puede persistirse.
    pub fn revoke_execution_grant(
        &mut self,
        layout: &Layout,
        id: &str,
        typed_id: &str,
        now_secs: u64,
    ) -> Result<GrantStatus, ApiErrorV2> {
        require_typed_id(id, typed_id)?;
        let status = OperationalApi::new(layout, now_secs)
            .grant_revoke(id, true)?
            .data;
        self.status = format!("grant revocado: {id}; historia preservada");
        Ok(status)
    }

    /// Previsualiza el formulario tipado completo de `RunRequestV2`.
    ///
    /// # Errors
    ///
    /// Si el formulario no puede serializarse o no pasa grant y selección.
    pub fn preview_run(
        &mut self,
        layout: &Layout,
        request: &RunRequestV2,
        now_ms: u64,
    ) -> Result<RunPreviewV2, ApiErrorV2> {
        let json = serde_json::to_string(request).map_err(|error| {
            ApiErrorV2::new(
                "serialization_error",
                "request",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
        self.preview_run_json(layout, &json, now_ms)
    }

    /// Carga `RunRequestV2`, selecciona y calcula el deadline sin reservar.
    ///
    /// # Errors
    ///
    /// Si el JSON, grant, estado o selección no validan.
    pub fn preview_run_json(
        &mut self,
        layout: &Layout,
        input: &str,
        now_ms: u64,
    ) -> Result<RunPreviewV2, ApiErrorV2> {
        let request: RunRequestV2 = serde_json::from_str(input).map_err(|error| {
            ApiErrorV2::new(
                "invalid_json",
                "request",
                error.to_string(),
                serde_json::json!({"document": "run_request"}),
            )
        })?;
        let grant = active_grant(layout, &request, now_ms)?;
        let service = ApplicationService::from_state_store(
            &StateStore::open(layout.state()),
            now_ms / 1_000,
            false,
        )
        .map_err(|error| {
            ApiErrorV2::new(
                "routing_state_required",
                "state.manifest",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
        let decision = service
            .route_with_allowed_routes(request.routing.clone(), &grant.routes, &BTreeSet::new())
            .map_err(ApiErrorV2::from)?;
        let preview = RunPreviewV2 {
            schema_version: 2,
            run_id: request.id.clone(),
            manifest_hash: decision.manifest_hash,
            route: decision.route,
            grant_id: grant.id,
            grant_hash: grant.grant_hash,
            budget: grant.limits,
            deadline_at_ms: grant
                .expires_at
                .saturating_mul(1_000)
                .min(now_ms.saturating_add(grant.limits.wall_time_ms)),
            reserved: false,
        };
        self.status = format!(
            "preview {}: {} · sin reserva; escriba el run ID para ejecutar",
            preview.run_id, preview.route
        );
        self.execution.run_request_json = Some(input.to_string());
        self.execution.run_preview = Some(preview.clone());
        Ok(preview)
    }

    /// Encola el request previsualizado sólo tras confirmación por run ID.
    ///
    /// # Errors
    ///
    /// Si no hay preview, el ID escrito no coincide o el worker no admite el trabajo.
    pub fn queue_previewed_run(
        &mut self,
        worker: &TuiExecutionWorker,
        typed_id: &str,
    ) -> Result<(), ApiErrorV2> {
        let preview = self.execution.run_preview.as_ref().ok_or_else(|| {
            ApiErrorV2::new(
                "run_preview_required",
                "preview",
                "load and review a RunRequestV2 before execution",
                serde_json::Value::Null,
            )
        })?;
        if typed_id != preview.run_id {
            return Err(ApiErrorV2::new(
                "confirmation_mismatch",
                "confirmation",
                "typed run ID does not match the preview",
                serde_json::json!({"expected": preview.run_id, "actual": typed_id}),
            ));
        }
        let request_json = self.execution.run_request_json.clone().ok_or_else(|| {
            ApiErrorV2::new(
                "run_preview_required",
                "preview",
                "preview has no normalized request",
                serde_json::Value::Null,
            )
        })?;
        worker.submit(TuiExecutionJob::Run { request_json })?;
        self.status = format!("run {} enviado al worker", preview.run_id);
        Ok(())
    }
}

fn active_grant(
    layout: &Layout,
    request: &RunRequestV2,
    now_ms: u64,
) -> Result<ExecutionGrantV1, ApiErrorV2> {
    let grant = GrantStore::open(layout.grants())
        .authorize(&request.grant_id, now_ms / 1_000)
        .map_err(|error| {
            ApiErrorV2::new(
                "grant_error",
                "grant_id",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
    if !grant.operations.contains(&GrantOperation::Run)
        || !grant.actions.contains(&request.routing.request.action)
    {
        return Err(ApiErrorV2::new(
            "grant_scope_error",
            "grant_id",
            "grant does not permit the exact action and run operation",
            serde_json::Value::Null,
        ));
    }
    Ok(grant)
}

fn require_typed_id(expected: &str, actual: &str) -> Result<(), ApiErrorV2> {
    if expected == actual {
        Ok(())
    } else {
        Err(ApiErrorV2::new(
            "confirmation_mismatch",
            "confirmation",
            "typed ID does not match the reviewed document",
            serde_json::json!({"expected": expected, "actual": actual}),
        ))
    }
}
