//! Contratos serializables del ciclo de vida de un run.

use batuta_contract::{RouteRef, TaskSpec};
use batuta_exec::InvocationFailure;
use serde::{Deserialize, Serialize};

use crate::{
    BudgetAmount, DiscardedRoute, ExecutionGrantV1, ExecutionPolicyV2, HandoffCheckpoint,
    RouteRequestEnvelopeV2, RunCandidateReceiptV2, RunConsumptionReceiptV2, RunDecisionReceiptV2,
    RunReceiptReferenceV2, RunReservationReceiptV2, RunResultReceiptV2,
};

/// Solicitud pública de una ejecución v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequestV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Identificador durable.
    pub id: String,
    /// Objetivo autocontenido; no contiene historial conversacional.
    pub objective: String,
    /// Contrato completo de la tarea.
    pub task: TaskSpec,
    /// Intención de routing.
    pub routing: RouteRequestEnvelopeV2,
    /// Grant ya creado.
    pub grant_id: String,
}

/// Fase durable observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhaseV2 {
    /// Selección durable, todavía sin reserva.
    Planned,
    /// Presupuesto durable confirmado.
    Reserved,
    /// La llamada pudo producir un efecto.
    InvocationStarted,
    /// Un intento terminó con fallo conocido y aún puede recuperarse.
    AttemptFailed,
    /// Espera durable de un retry explícito.
    WaitingRetry,
    /// Checkpoint durable listo para seleccionar fallback.
    HandoffReady,
    /// Resultado exitoso conocido.
    Completed,
    /// Fallo final conocido.
    Failed,
    /// Se desconoce si el proveedor produjo un efecto.
    OutcomeUnknown,
}

/// Tipo cerrado de evento del journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunJournalKindV2 {
    /// Selección validada.
    Planned,
    /// Reserva confirmada.
    Reserved,
    /// Marca sincronizada inmediatamente antes de llamar.
    InvocationStarted,
    /// Éxito conocido.
    InvocationSucceeded,
    /// Fallo conocido.
    InvocationFailed,
    /// Espera y siguiente intento quedaron reservados.
    RetryScheduled,
    /// Venció la espera durable.
    RetryElapsed,
    /// Checkpoint compacto creado.
    HandoffCreated,
    /// Resultado ambiguo recuperado.
    OutcomeUnknown,
}

/// Evento monotónico del journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunJournalEventV2 {
    /// Secuencia desde cero.
    pub sequence: u64,
    /// Instante Unix UTC en segundos.
    pub at: u64,
    /// Tipo de transición.
    pub kind: RunJournalKindV2,
    /// Ruta implicada.
    pub route: Option<RouteRef>,
}

/// Hechos durables de un intento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunAttemptV2 {
    /// Número desde uno.
    pub number: u32,
    /// Ruta exacta.
    pub route: RouteRef,
    /// Manifest de estado usado al seleccionar.
    pub state_manifest_hash: String,
    /// Candidato exacto seleccionado.
    pub candidate_hash: String,
    /// Manifest que el adaptador llegó a materializar.
    pub provider_manifest_hash: Option<String>,
    /// Reserva previa de esta llamada.
    pub reservation_id: String,
    /// Máximo reservado.
    pub reserved: BudgetAmount,
    /// Inicio durable en milisegundos Unix UTC.
    pub started_at_ms: Option<u64>,
    /// Fin conocido en milisegundos Unix UTC.
    pub finished_at_ms: Option<u64>,
    /// Fallo conocido.
    pub failure: Option<InvocationFailure>,
    /// No existe resultado fiable tras el inicio.
    pub outcome_unknown: bool,
    /// La observación de salud ya fue publicada.
    pub health_recorded: bool,
}

/// Acción durable que puede continuar `run resume`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunNextActionV2 {
    /// Repetir la misma ruta sólo por un `Retry-After` observado.
    RetrySameRoute {
        /// Ruta exacta.
        route: RouteRef,
        /// Instante mínimo Unix UTC en milisegundos.
        not_before_ms: u64,
        /// Duración reservada de la espera.
        wait_ms: u64,
        /// Reserva de pared de la espera.
        wait_reservation_id: String,
        /// Reserva del intento que seguirá.
        attempt_reservation_id: String,
        /// Número de ese intento.
        attempt: u32,
    },
}

/// Estado público recuperable de una ejecución.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStatusV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Run ID.
    pub id: String,
    /// Petición durable necesaria para reanudar.
    pub request: RunRequestV2,
    /// Grant usado.
    pub grant_id: String,
    /// Sello del grant al abrir el run.
    pub grant_hash: String,
    /// Creación Unix UTC en segundos.
    pub created_at: u64,
    /// Deadline conservador Unix UTC en milisegundos.
    pub deadline_at_ms: u64,
    /// Política cerrada que gobierna la recuperación.
    pub execution_policy: ExecutionPolicyV2,
    /// Fase durable.
    pub phase: RunPhaseV2,
    /// Consumo/carga actual únicamente de este run.
    pub consumed: BudgetAmount,
    /// Suma histórica de máximos reservados por este run.
    pub total_reserved: BudgetAmount,
    /// Ruta del intento vigente o final.
    pub route: Option<RouteRef>,
    /// Todos los intentos y sus manifests.
    pub attempts: Vec<RunAttemptV2>,
    /// Próxima acción durable.
    pub next_action: Option<RunNextActionV2>,
    /// Copia indexable de `not_before_ms`.
    pub next_action_at: Option<u64>,
    /// Relevos ya consumidos.
    pub handoffs: u32,
    /// Journal sincronizado.
    pub journal: Vec<RunJournalEventV2>,
    /// Último checkpoint compacto.
    pub checkpoint: Option<HandoffCheckpoint>,
    /// Todos los checkpoints usados por el recibo.
    pub checkpoints: Vec<HandoffCheckpoint>,
    /// Prohíbe retry o fallback.
    pub outcome_unknown: bool,
    /// Salida final conocida y acotada.
    pub output: Option<String>,
    /// Último fallo conocido.
    pub failure: Option<InvocationFailure>,
    /// Referencia al recibo final append-only.
    pub receipt: Option<RunReceiptReferenceV2>,
    /// Candidatos sellados acumulados.
    pub candidates: Vec<RunCandidateReceiptV2>,
    /// Descartes explicados acumulados.
    pub discards: Vec<DiscardedRoute>,
    /// Decisiones por intento.
    pub decisions: Vec<RunDecisionReceiptV2>,
    /// Reservas previas.
    pub reservations: Vec<RunReservationReceiptV2>,
    /// Consumos o cargas conservadoras.
    pub consumptions: Vec<RunConsumptionReceiptV2>,
    /// Resultados por intento.
    pub results: Vec<RunResultReceiptV2>,
}

impl RunStatusV2 {
    pub(super) fn empty(
        request: RunRequestV2,
        grant: &ExecutionGrantV1,
        policy: ExecutionPolicyV2,
        now_ms: u64,
    ) -> Self {
        Self {
            schema_version: 2,
            id: request.id.clone(),
            grant_id: request.grant_id.clone(),
            grant_hash: grant.grant_hash.clone(),
            request,
            created_at: now_ms / 1_000,
            deadline_at_ms: grant
                .expires_at
                .saturating_mul(1_000)
                .min(now_ms.saturating_add(grant.limits.wall_time_ms)),
            execution_policy: policy,
            phase: RunPhaseV2::Planned,
            consumed: BudgetAmount::default(),
            total_reserved: BudgetAmount::default(),
            route: None,
            attempts: Vec::new(),
            next_action: None,
            next_action_at: None,
            handoffs: 0,
            journal: Vec::new(),
            checkpoint: None,
            checkpoints: Vec::new(),
            outcome_unknown: false,
            output: None,
            failure: None,
            receipt: None,
            candidates: Vec::new(),
            discards: Vec::new(),
            decisions: Vec::new(),
            reservations: Vec::new(),
            consumptions: Vec::new(),
            results: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, now_ms: u64, kind: RunJournalKindV2, route: Option<RouteRef>) {
        self.journal.push(RunJournalEventV2 {
            sequence: u64::try_from(self.journal.len()).unwrap_or(u64::MAX),
            at: now_ms / 1_000,
            kind,
            route,
        });
    }
}
