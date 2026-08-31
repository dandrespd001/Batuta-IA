//! Validación, hashes y aritmética pura del coordinador.

use std::fmt;

use batuta_contract::RouteRef;
use batuta_exec::InvocationFailure;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use super::model::{RunPhaseV2, RunRequestV2, RunStatusV2};
use crate::{
    BudgetAmount, ExecutionGrantV1, FailureCategory, HandoffCheckpoint, HandoffDraft, LedgerStatus,
    RouteDecision, RunConsumptionReceiptV2,
};

pub(super) fn reservation_for(request: &RunRequestV2) -> Result<BudgetAmount, RunCoordinatorError> {
    let tokens = request.routing.request.predicted_tokens.max(1);
    let wall_time_ms = u64::from(request.task.timeout_seconds())
        .checked_mul(1_000)
        .ok_or_else(|| RunCoordinatorError::Invalid("timeout overflow".to_string()))?;
    Ok(BudgetAmount {
        requests: 1,
        input_tokens: tokens,
        output_tokens: tokens,
        wall_time_ms,
    })
}

pub(super) fn reservation_id(run_id: &str, kind: char, attempt: u32) -> String {
    let digest = format!("{:x}", Sha256::digest(run_id.as_bytes()));
    format!("r-{}-{kind}{attempt}", &digest[..32])
}

pub(super) fn candidate_hash(
    decision: &RouteDecision,
    action: &str,
) -> Result<String, RunCoordinatorError> {
    #[derive(Serialize)]
    struct CandidateSeal<'a> {
        action: &'a str,
        route: &'a RouteRef,
        manifest_hash: &'a str,
        catalog_hash: &'a str,
        policy_hash: &'a str,
        evidence_hash: &'a str,
        health_hash: &'a str,
        capabilities_hash: &'a str,
        effective_score: f64,
        expected_cost: f64,
    }
    let bytes = serde_json::to_vec(&CandidateSeal {
        action,
        route: &decision.route,
        manifest_hash: &decision.manifest_hash,
        catalog_hash: &decision.catalog_hash,
        policy_hash: &decision.policy_hash,
        evidence_hash: &decision.evidence_hash,
        health_hash: &decision.health_hash,
        capabilities_hash: &decision.capabilities_hash,
        effective_score: decision.effective_score,
        expected_cost: decision.expected_cost,
    })
    .map_err(RunCoordinatorError::Json)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub(super) fn invocation_objective(status: &RunStatusV2) -> Result<String, RunCoordinatorError> {
    match &status.checkpoint {
        Some(checkpoint) => serde_json::to_string(checkpoint).map_err(RunCoordinatorError::Json),
        None => Ok(status.request.objective.clone()),
    }
}

pub(super) fn checkpoint_for(
    status: &RunStatusV2,
    grant: &ExecutionGrantV1,
    next_route: &RouteRef,
) -> Result<HandoffCheckpoint, RunCoordinatorError> {
    let failure = status.failure.ok_or_else(|| {
        RunCoordinatorError::Invalid("handoff requires a known failure".to_string())
    })?;
    let remaining_input = grant
        .limits
        .input_tokens
        .saturating_sub(status.consumed.input_tokens);
    let remaining_output = grant
        .limits
        .output_tokens
        .saturating_sub(status.consumed.output_tokens);
    HandoffCheckpoint::try_from(HandoffDraft {
        schema_version: 1,
        objective: status.request.objective.clone(),
        constraints: vec![
            "do not replay conversation history".to_string(),
            format!("grant={}", grant.id),
        ],
        decisions: status
            .attempts
            .iter()
            .map(|attempt| format!("attempt {} used {}", attempt.number, attempt.route))
            .collect(),
        files: Vec::new(),
        diff_summary: "no previous model output forwarded".to_string(),
        tests: Vec::new(),
        failure: failure_category(failure),
        failure_message: format!("observed harness result: {failure:?}"),
        next_step: format!("continue the objective on exact route {next_route}"),
        remaining_tokens: remaining_input.saturating_add(remaining_output),
        remaining_wall_seconds: grant
            .limits
            .wall_time_ms
            .saturating_sub(status.consumed.wall_time_ms)
            / 1_000,
    })
    .map_err(|error| RunCoordinatorError::Invalid(error.to_string()))
}

const fn failure_category(failure: InvocationFailure) -> FailureCategory {
    match failure {
        InvocationFailure::RateLimited {
            retry_after_ms: Some(millis),
        } => FailureCategory::RateLimited {
            retry_after_seconds: millis.div_ceil(1_000),
        },
        InvocationFailure::RateLimited {
            retry_after_ms: None,
        } => FailureCategory::RateLimitedUnknown,
        InvocationFailure::Quota => FailureCategory::QuotaExhausted,
        InvocationFailure::Authentication => FailureCategory::Authentication,
        InvocationFailure::Balance => FailureCategory::Balance,
        InvocationFailure::Transient => FailureCategory::Transient,
        InvocationFailure::Timeout => FailureCategory::Timeout,
        InvocationFailure::Permanent => FailureCategory::Permanent,
    }
}

pub(super) fn upsert_consumption(
    status: &mut RunStatusV2,
    consumption: RunConsumptionReceiptV2,
) -> Result<(), RunCoordinatorError> {
    if let Some(existing) = status
        .consumptions
        .iter()
        .find(|existing| existing.id == consumption.id)
    {
        if existing != &consumption {
            return Err(RunCoordinatorError::Invalid(
                "durable consumption changed".to_string(),
            ));
        }
    } else {
        status.consumptions.push(consumption);
    }
    Ok(())
}

pub(super) fn refresh_budget(
    status: &mut RunStatusV2,
    ledger: &LedgerStatus,
) -> Result<(), RunCoordinatorError> {
    let mut total_reserved = BudgetAmount::default();
    let mut consumed = BudgetAmount::default();
    for record in &status.reservations {
        total_reserved = checked_add(total_reserved, record.amount)?;
        let reservation = ledger.reservations.get(&record.id).ok_or_else(|| {
            RunCoordinatorError::Budget(format!("ledger has no reservation '{}'", record.id))
        })?;
        let charged = if reservation.outcome_unknown {
            reservation.reserved
        } else {
            reservation.confirmed.unwrap_or(reservation.reserved)
        };
        consumed = checked_add(consumed, charged)?;
    }
    status.total_reserved = total_reserved;
    status.consumed = consumed;
    Ok(())
}

fn checked_add(
    left: BudgetAmount,
    right: BudgetAmount,
) -> Result<BudgetAmount, RunCoordinatorError> {
    Ok(BudgetAmount {
        requests: left
            .requests
            .checked_add(right.requests)
            .ok_or_else(|| RunCoordinatorError::Budget("budget overflow".to_string()))?,
        input_tokens: left
            .input_tokens
            .checked_add(right.input_tokens)
            .ok_or_else(|| RunCoordinatorError::Budget("budget overflow".to_string()))?,
        output_tokens: left
            .output_tokens
            .checked_add(right.output_tokens)
            .ok_or_else(|| RunCoordinatorError::Budget("budget overflow".to_string()))?,
        wall_time_ms: left
            .wall_time_ms
            .checked_add(right.wall_time_ms)
            .ok_or_else(|| RunCoordinatorError::Budget("budget overflow".to_string()))?,
    })
}

pub(super) fn validate_request(request: &RunRequestV2) -> Result<(), RunCoordinatorError> {
    if request.schema_version != 2 {
        return Err(RunCoordinatorError::Invalid(
            "run request schema_version must be 2".to_string(),
        ));
    }
    validate_id(&request.id)?;
    validate_id(&request.grant_id)?;
    if request.objective.trim().is_empty() {
        return Err(RunCoordinatorError::Invalid(
            "run objective cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_status(status: &RunStatusV2, id: &str) -> Result<(), RunCoordinatorError> {
    if status.schema_version != 2
        || status.id != id
        || status.request.id != id
        || status.request.grant_id != status.grant_id
        || status.journal.is_empty()
        || status.attempts.is_empty()
    {
        return Err(RunCoordinatorError::Invalid(
            "run status identity, version, attempts or journal is invalid".to_string(),
        ));
    }
    let mut previous_at = status.created_at;
    for (index, event) in status.journal.iter().enumerate() {
        if event.sequence != u64::try_from(index).unwrap_or(u64::MAX) || event.at < previous_at {
            return Err(RunCoordinatorError::Invalid(
                "run journal is not contiguous and monotonic".to_string(),
            ));
        }
        previous_at = event.at;
    }
    for (index, attempt) in status.attempts.iter().enumerate() {
        if attempt.number != u32::try_from(index + 1).unwrap_or(u32::MAX) {
            return Err(RunCoordinatorError::Invalid(
                "run attempt sequence is not contiguous".to_string(),
            ));
        }
    }
    if status.outcome_unknown != (status.phase == RunPhaseV2::OutcomeUnknown) {
        return Err(RunCoordinatorError::Invalid(
            "outcome_unknown flag and phase disagree".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_id(id: &str) -> Result<(), RunCoordinatorError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RunCoordinatorError::Invalid(format!(
            "invalid identifier '{id}'"
        )));
    }
    Ok(())
}

/// Error tipado del coordinador.
#[derive(Debug)]
pub enum RunCoordinatorError {
    /// Entrada o estado inválido.
    Invalid(String),
    /// Run ID ya usado.
    AlreadyExists(String),
    /// Grant ausente, vencido o revocado.
    Grant(String),
    /// Selección imposible.
    Route(String),
    /// Presupuesto agotado o incoherente.
    Budget(String),
    /// Manifest o generación de estado inválidos.
    State(String),
    /// Recibo inválido o no persistible.
    Receipt(String),
    /// Aún no venció una espera durable.
    ProbeNotDue {
        /// Instante mínimo Unix UTC en milisegundos.
        next_action_at: u64,
    },
    /// Exclusión interproceso.
    Lease(String),
    /// E/S.
    Io(std::io::Error),
    /// JSON.
    Json(serde_json::Error),
}

impl fmt::Display for RunCoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message)
            | Self::Grant(message)
            | Self::Route(message)
            | Self::Budget(message)
            | Self::State(message)
            | Self::Receipt(message)
            | Self::Lease(message) => formatter.write_str(message),
            Self::AlreadyExists(id) => write!(formatter, "run already exists: {id}"),
            Self::ProbeNotDue { next_action_at } => {
                write!(formatter, "probe_not_due: next action at {next_action_at}")
            }
            Self::Io(error) => write!(formatter, "run I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "run JSON failed: {error}"),
        }
    }
}

impl std::error::Error for RunCoordinatorError {}
