//! Recibo exhaustivo, sellado y append-only de una ejecución K4.

#![allow(clippy::missing_errors_doc)]

use std::fmt;

use batuta_contract::RouteRef;
use batuta_exec::{InvocationFailure, TokenUsage};
use serde::{Deserialize, Serialize};

use crate::{
    BudgetAmount, DiscardedRoute, ExecutionGrantV1, HandoffCheckpoint, RunJournalEventV2,
    RunPhaseV2, RunRequestV2,
};

mod store;
mod validation;

pub use store::RunReceiptStoreV2;
use validation::{calculate_hash, validate_draft};

/// Candidato evaluado por una generación concreta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCandidateReceiptV2 {
    /// Ruta exacta.
    pub route: RouteRef,
    /// Acción evaluada.
    pub action: String,
    /// Hash canónico del candidato materializado.
    pub candidate_hash: String,
}

/// Selección efectuada antes de un intento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunDecisionReceiptV2 {
    /// Número de intento, desde uno.
    pub attempt: u64,
    /// Manifest de estado abierto para esta selección.
    pub manifest_hash: String,
    /// Ruta seleccionada.
    pub route: RouteRef,
    /// Hash del candidato seleccionado.
    pub candidate_hash: String,
}

/// Naturaleza de una reserva previa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunReservationKindV2 {
    /// Espera de un retry explícito.
    Wait,
    /// Una única invocación.
    Attempt,
}

/// Reserva durable efectuada antes del efecto correspondiente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReservationReceiptV2 {
    /// ID único en el grant.
    pub id: String,
    /// Tipo de efecto reservado.
    pub kind: RunReservationKindV2,
    /// Máximo cargado antes del efecto.
    pub amount: BudgetAmount,
}

/// Consumo conocido o carga conservadora de una reserva.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConsumptionReceiptV2 {
    /// Reserva a la que corresponde.
    pub id: String,
    /// Cantidad finalmente cargada.
    pub amount: BudgetAmount,
    /// `false` conserva la reserva completa por resultado ambiguo.
    pub known: bool,
}

/// Resultado de una única llamada, sin historial implícito.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunResultReceiptV2 {
    /// Número de intento.
    pub attempt: u64,
    /// Ruta invocada.
    pub route: RouteRef,
    /// Salida acotada cuando es conocida.
    pub output: Option<String>,
    /// Uso observado.
    pub usage: TokenUsage,
    /// Latencia observada.
    pub latency_ms: u64,
    /// Procedencia comunicada por el harness.
    pub provenance: Option<String>,
    /// Manifest de proveedor realmente materializado.
    pub provider_manifest_hash: Option<String>,
    /// Fallo observado, si existe.
    pub failure: Option<InvocationFailure>,
    /// El inicio quedó durable pero no existe resultado fiable.
    pub outcome_unknown: bool,
}

/// Forma sin sello de un recibo final.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceiptDraftV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Run ID.
    pub id: String,
    /// Instante de creación Unix UTC.
    pub created_at: u64,
    /// Petición pública completa.
    pub request: RunRequestV2,
    /// Grant histórico completo.
    pub grant: ExecutionGrantV1,
    /// Copia explícita del sello del grant.
    pub grant_hash: String,
    /// Candidatos materializados y sellados.
    pub candidates: Vec<RunCandidateReceiptV2>,
    /// Descartes explicados.
    pub discards: Vec<DiscardedRoute>,
    /// Selecciones por intento.
    pub decisions: Vec<RunDecisionReceiptV2>,
    /// Reservas previas.
    pub reservations: Vec<RunReservationReceiptV2>,
    /// Consumos conocidos o conservadores.
    pub consumptions: Vec<RunConsumptionReceiptV2>,
    /// Journal durable completo.
    pub transitions: Vec<RunJournalEventV2>,
    /// Resultados observados.
    pub results: Vec<RunResultReceiptV2>,
    /// Relevos compactos; nunca contienen historial conversacional.
    pub checkpoints: Vec<HandoffCheckpoint>,
    /// Estado terminal.
    pub final_phase: RunPhaseV2,
}

/// Recibo final cuyo hash cubre todos los hechos anteriores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceiptV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Run ID.
    pub id: String,
    /// Instante de creación Unix UTC.
    pub created_at: u64,
    /// Petición pública completa.
    pub request: RunRequestV2,
    /// Grant histórico completo.
    pub grant: ExecutionGrantV1,
    /// Copia explícita del sello del grant.
    pub grant_hash: String,
    /// Candidatos materializados y sellados.
    pub candidates: Vec<RunCandidateReceiptV2>,
    /// Descartes explicados.
    pub discards: Vec<DiscardedRoute>,
    /// Selecciones por intento.
    pub decisions: Vec<RunDecisionReceiptV2>,
    /// Reservas previas.
    pub reservations: Vec<RunReservationReceiptV2>,
    /// Consumos conocidos o conservadores.
    pub consumptions: Vec<RunConsumptionReceiptV2>,
    /// Journal durable completo.
    pub transitions: Vec<RunJournalEventV2>,
    /// Resultados observados.
    pub results: Vec<RunResultReceiptV2>,
    /// Relevos compactos.
    pub checkpoints: Vec<HandoffCheckpoint>,
    /// Estado terminal.
    pub final_phase: RunPhaseV2,
    /// Hash SHA-256 del borrador canónico.
    pub receipt_hash: String,
}

impl RunReceiptV2 {
    /// Valida todos los vínculos y sella el documento canónico.
    pub fn seal(draft: RunReceiptDraftV2) -> Result<Self, RunReceiptError> {
        validate_draft(&draft)?;
        let receipt_hash = calculate_hash(&draft)?;
        Ok(Self {
            schema_version: draft.schema_version,
            id: draft.id,
            created_at: draft.created_at,
            request: draft.request,
            grant: draft.grant,
            grant_hash: draft.grant_hash,
            candidates: draft.candidates,
            discards: draft.discards,
            decisions: draft.decisions,
            reservations: draft.reservations,
            consumptions: draft.consumptions,
            transitions: draft.transitions,
            results: draft.results,
            checkpoints: draft.checkpoints,
            final_phase: draft.final_phase,
            receipt_hash,
        })
    }

    /// Revalida vínculos y sello después de deserializar.
    pub fn validate(&self) -> Result<(), RunReceiptError> {
        let draft = self.clone().into_draft();
        validate_draft(&draft)?;
        if calculate_hash(&draft)? != self.receipt_hash {
            return Err(RunReceiptError::HashMismatch);
        }
        Ok(())
    }

    /// Elimina exclusivamente el sello derivado.
    pub fn into_draft(self) -> RunReceiptDraftV2 {
        RunReceiptDraftV2 {
            schema_version: self.schema_version,
            id: self.id,
            created_at: self.created_at,
            request: self.request,
            grant: self.grant,
            grant_hash: self.grant_hash,
            candidates: self.candidates,
            discards: self.discards,
            decisions: self.decisions,
            reservations: self.reservations,
            consumptions: self.consumptions,
            transitions: self.transitions,
            results: self.results,
            checkpoints: self.checkpoints,
            final_phase: self.final_phase,
        }
    }
}

/// Referencia estable publicada en el estado del run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceiptReferenceV2 {
    /// ID del recibo, igual al run ID.
    pub id: String,
    /// Sello final.
    pub receipt_hash: String,
}

/// Fallo al validar o persistir un recibo.
#[derive(Debug)]
pub enum RunReceiptError {
    /// Contrato incoherente.
    Invalid(String),
    /// El sello no corresponde al contenido.
    HashMismatch,
    /// El ID ya posee un recibo inmutable.
    AlreadyExists(String),
    /// Fallo de almacenamiento.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
}

impl fmt::Display for RunReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid run receipt: {message}"),
            Self::HashMismatch => formatter.write_str("run receipt hash mismatch"),
            Self::AlreadyExists(id) => write!(formatter, "run receipt '{id}' already exists"),
            Self::Io(error) => write!(formatter, "run receipt I/O error: {error}"),
            Self::Json(error) => write!(formatter, "run receipt JSON error: {error}"),
        }
    }
}

impl std::error::Error for RunReceiptError {}
