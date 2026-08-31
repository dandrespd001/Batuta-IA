//! Entrada interactiva cerrada para la vista Execution.

use std::path::PathBuf;

use batuta_contract::TaskSpec;
use batuta_routing::{RouteRequestEnvelopeV2, RunRequestV2};
use serde::de::DeserializeOwned;

use super::{TuiApp, TuiExecutionJob, TuiExecutionWorker};
use crate::{ApiErrorV2, Layout, run_status};

/// Acción que puede iniciar el operador desde la vista Execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiInputAction {
    /// Completa los cuatro campos admitidos por el perfil operativo.
    ProfileForm,
    /// Pega un borrador JSON cerrado del perfil.
    ProfileJson,
    /// Confirma la propuesta visible escribiendo su ID.
    ProfileApply,
    /// Pega un borrador JSON cerrado y confirma su grant ID.
    GrantCreate,
    /// Consulta un grant por ID.
    GrantStatus,
    /// Revoca un grant escribiendo dos veces el mismo ID.
    GrantRevoke,
    /// Completa los campos superiores del run y los contratos tipados anidados.
    RunForm,
    /// Pega un `RunRequestV2` JSON completo.
    RunJson,
    /// Ejecuta el preview visible escribiendo su run ID.
    RunExecute,
    /// Consulta una corrida por ID.
    RunStatus,
    /// Reanuda una corrida por ID mediante el worker único.
    RunResume,
}

impl TuiInputAction {
    fn fields(self) -> &'static [&'static str] {
        match self {
            Self::ProfileForm => &[
                "workdir canónico",
                "max_stdout_bytes",
                "max_stderr_bytes",
                "termination_grace_ms",
            ],
            Self::ProfileJson => &["borrador ExecutionProfileV1 JSON"],
            Self::ProfileApply => &["confirmar proposal ID"],
            Self::GrantCreate => &["borrador ExecutionGrantV1 JSON", "confirmar grant ID"],
            Self::GrantStatus => &["grant ID"],
            Self::GrantRevoke => &["grant ID", "confirmar grant ID"],
            Self::RunForm => &[
                "run ID",
                "objective",
                "TaskSpec JSON",
                "RouteRequestEnvelopeV2 JSON",
                "grant ID",
            ],
            Self::RunJson => &["RunRequestV2 JSON"],
            Self::RunExecute => &["confirmar run ID"],
            Self::RunStatus | Self::RunResume => &["run ID"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputSession {
    action: TuiInputAction,
    field: usize,
    values: Vec<String>,
    buffer: String,
}

impl InputSession {
    fn new(action: TuiInputAction) -> Self {
        Self {
            action,
            field: 0,
            values: Vec::new(),
            buffer: String::new(),
        }
    }

    fn prompt(&self) -> &'static str {
        self.action.fields()[self.field]
    }

    fn is_last(&self) -> bool {
        self.field + 1 == self.action.fields().len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct TuiInteractionState {
    session: Option<InputSession>,
}

impl TuiApp {
    /// Inicia una acción cerrada y descarta cualquier entrada anterior sin persistirla.
    ///
    /// # Errors
    ///
    /// Si la acción requiere una propuesta o preview que todavía no existe.
    pub fn begin_execution_input(&mut self, action: TuiInputAction) -> Result<(), ApiErrorV2> {
        if action == TuiInputAction::ProfileApply && self.execution_profile_proposal().is_none() {
            return Err(required_error(
                "profile_proposal_required",
                "profile_proposal",
                "stage and review an execution profile before applying it",
            ));
        }
        if action == TuiInputAction::RunExecute && self.execution_run_preview().is_none() {
            return Err(required_error(
                "run_preview_required",
                "preview",
                "load and review a RunRequestV2 before execution",
            ));
        }
        self.interaction.session = Some(InputSession::new(action));
        self.status = format!(
            "entrada activa: {}",
            self.execution_input_prompt().unwrap_or("?")
        );
        Ok(())
    }

    /// Etiqueta del campo interactivo actual.
    pub fn execution_input_prompt(&self) -> Option<&str> {
        self.interaction.session.as_ref().map(InputSession::prompt)
    }

    /// Indica si las teclas deben editar el formulario en vez de navegar.
    pub fn execution_input_active(&self) -> bool {
        self.interaction.session.is_some()
    }

    /// Sustituye el campo actual; se usa también para pegar JSON en pruebas y adaptadores.
    pub fn replace_execution_input(&mut self, value: &str) {
        if let Some(session) = self.interaction.session.as_mut() {
            session.buffer.clear();
            session.buffer.push_str(value);
        }
    }

    pub(super) fn append_execution_input(&mut self, value: &str) {
        if let Some(session) = self.interaction.session.as_mut() {
            session.buffer.push_str(value);
        }
    }

    pub(super) fn pop_execution_input(&mut self) {
        if let Some(session) = self.interaction.session.as_mut() {
            session.buffer.pop();
        }
    }

    pub(super) fn cancel_execution_input(&mut self) {
        self.interaction.session = None;
        self.status = "entrada cancelada; no se guardó nada".to_string();
    }

    /// Avanza un campo o ejecuta la acción validada cuando el formulario termina.
    ///
    /// # Errors
    ///
    /// Si el campo está vacío, un contrato no valida o falla su operación durable.
    pub fn submit_execution_input(
        &mut self,
        layout: &Layout,
        worker: &TuiExecutionWorker,
        now_ms: u64,
    ) -> Result<(), ApiErrorV2> {
        let session = self.interaction.session.as_ref().ok_or_else(|| {
            required_error(
                "input_not_active",
                "input",
                "start an Execution action before submitting input",
            )
        })?;
        if session.buffer.trim().is_empty() {
            return Err(required_error(
                "input_required",
                session.prompt(),
                "the current field cannot be empty",
            ));
        }
        if !session.is_last() {
            let Some(session) = self.interaction.session.as_mut() else {
                return Err(required_error(
                    "input_not_active",
                    "input",
                    "start an Execution action before submitting input",
                ));
            };
            session.values.push(std::mem::take(&mut session.buffer));
            session.field += 1;
            self.status = format!("entrada activa: {}", session.prompt());
            return Ok(());
        }

        let action = session.action;
        let mut values = session.values.clone();
        values.push(session.buffer.clone());
        dispatch(self, layout, worker, now_ms, action, &values)?;
        self.interaction.session = None;
        Ok(())
    }

    pub(super) fn execution_input_snapshot(&self) -> String {
        let Some(session) = self.interaction.session.as_ref() else {
            return String::new();
        };
        let visible = tail_chars(&session.buffer, 160);
        format!("\nEntrada — {}\n> {visible}", session.prompt())
    }
}

fn dispatch(
    app: &mut TuiApp,
    layout: &Layout,
    worker: &TuiExecutionWorker,
    now_ms: u64,
    action: TuiInputAction,
    values: &[String],
) -> Result<(), ApiErrorV2> {
    match action {
        TuiInputAction::ProfileForm => stage_profile_form(app, layout, now_ms, values),
        TuiInputAction::ProfileJson => app
            .stage_execution_profile_json(layout, &values[0], now_ms / 1_000)
            .map(|_| ()),
        TuiInputAction::ProfileApply => {
            let proposal = app.execution_profile_proposal().cloned().ok_or_else(|| {
                required_error(
                    "profile_proposal_required",
                    "profile_proposal",
                    "stage and review an execution profile before applying it",
                )
            })?;
            app.apply_execution_profile(
                layout,
                &proposal.id,
                &values[0],
                &proposal.expected_active_hash,
                now_ms / 1_000,
            )
        }
        TuiInputAction::GrantCreate => app
            .create_execution_grant_json(layout, &values[0], &values[1], now_ms / 1_000)
            .map(|_| ()),
        TuiInputAction::GrantStatus => {
            let status = app.execution_grant_status(layout, &values[0], now_ms / 1_000)?;
            app.status = format!("grant status: {}", to_json(&status, "grant")?);
            Ok(())
        }
        TuiInputAction::GrantRevoke => app
            .revoke_execution_grant(layout, &values[0], &values[1], now_ms / 1_000)
            .map(|_| ()),
        TuiInputAction::RunForm => preview_run_form(app, layout, now_ms, values),
        TuiInputAction::RunJson => app.preview_run_json(layout, &values[0], now_ms).map(|_| ()),
        TuiInputAction::RunExecute => app.queue_previewed_run(worker, &values[0]),
        TuiInputAction::RunStatus => {
            let status = run_status(layout, &values[0])?.data;
            app.status = format!("run status: {}", to_json(&status, "run")?);
            Ok(())
        }
        TuiInputAction::RunResume => {
            worker.submit(TuiExecutionJob::Resume {
                id: values[0].clone(),
            })?;
            app.status = format!("run {} enviado al worker para resume", values[0]);
            Ok(())
        }
    }
}

fn stage_profile_form(
    app: &mut TuiApp,
    layout: &Layout,
    now_ms: u64,
    values: &[String],
) -> Result<(), ApiErrorV2> {
    let draft = serde_json::json!({
        "schema_version": 1,
        "workdir": PathBuf::from(&values[0]),
        "max_stdout_bytes": parse_u64("max_stdout_bytes", &values[1])?,
        "max_stderr_bytes": parse_u64("max_stderr_bytes", &values[2])?,
        "termination_grace_ms": parse_u64("termination_grace_ms", &values[3])?,
    });
    app.stage_execution_profile_json(layout, &draft.to_string(), now_ms / 1_000)
        .map(|_| ())
}

fn preview_run_form(
    app: &mut TuiApp,
    layout: &Layout,
    now_ms: u64,
    values: &[String],
) -> Result<(), ApiErrorV2> {
    let request = RunRequestV2 {
        schema_version: 2,
        id: values[0].clone(),
        objective: values[1].clone(),
        task: parse_json::<TaskSpec>("task", &values[2])?,
        routing: parse_json::<RouteRequestEnvelopeV2>("routing", &values[3])?,
        grant_id: values[4].clone(),
    };
    app.preview_run(layout, &request, now_ms).map(|_| ())
}

fn parse_json<T: DeserializeOwned>(field: &'static str, value: &str) -> Result<T, ApiErrorV2> {
    serde_json::from_str(value).map_err(|error| {
        ApiErrorV2::new(
            "invalid_json",
            field,
            error.to_string(),
            serde_json::json!({"document": field}),
        )
    })
}

fn parse_u64(field: &'static str, value: &str) -> Result<u64, ApiErrorV2> {
    value.parse::<u64>().map_err(|error| {
        ApiErrorV2::new(
            "invalid_integer",
            field,
            error.to_string(),
            serde_json::json!({"value": value}),
        )
    })
}

fn to_json(value: &impl serde::Serialize, field: &'static str) -> Result<String, ApiErrorV2> {
    serde_json::to_string(value).map_err(|error| {
        ApiErrorV2::new(
            "serialization_error",
            field,
            error.to_string(),
            serde_json::Value::Null,
        )
    })
}

fn required_error(code: &'static str, field: &str, message: &'static str) -> ApiErrorV2 {
    ApiErrorV2::new(code, field, message, serde_json::Value::Null)
}

fn tail_chars(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    if count <= limit {
        return value.to_string();
    }
    format!("…{}", value.chars().skip(count - limit).collect::<String>())
}
