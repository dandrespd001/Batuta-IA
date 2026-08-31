//! Validación de vínculos y cálculo del sello canónico.

use std::collections::{BTreeMap, BTreeSet};

use batuta_exec::TokenUsage;
use sha2::{Digest as _, Sha256};

use super::{RunReceiptDraftV2, RunReceiptError, RunReservationKindV2};
use crate::{BudgetAmount, RunJournalKindV2, RunPhaseV2};

pub(super) fn validate_draft(draft: &RunReceiptDraftV2) -> Result<(), RunReceiptError> {
    if draft.schema_version != 2 {
        return invalid("schema_version must be 2");
    }
    validate_id(&draft.id)?;
    if draft.request.schema_version != 2
        || draft.request.id != draft.id
        || draft.request.grant_id != draft.grant.id
    {
        return invalid("request, receipt and grant identifiers must agree");
    }
    draft
        .grant
        .validate_seal()
        .map_err(|error| RunReceiptError::Invalid(error.to_string()))?;
    if draft.grant_hash != draft.grant.grant_hash {
        return invalid("grant_hash does not match the embedded grant");
    }
    validate_hash(&draft.grant_hash)?;
    if !matches!(
        draft.final_phase,
        RunPhaseV2::Completed | RunPhaseV2::Failed | RunPhaseV2::OutcomeUnknown
    ) {
        return invalid("a receipt can only be sealed in a terminal phase");
    }
    validate_candidates_and_decisions(draft)?;
    validate_reservations(draft)?;
    validate_transitions(draft)?;
    validate_results(draft)
}

fn validate_candidates_and_decisions(draft: &RunReceiptDraftV2) -> Result<(), RunReceiptError> {
    let mut candidates = BTreeSet::new();
    for candidate in &draft.candidates {
        if candidate.action.trim().is_empty() {
            return invalid("candidate action cannot be empty");
        }
        validate_hash(&candidate.candidate_hash)?;
        if !candidates.insert((candidate.route.clone(), candidate.candidate_hash.clone())) {
            return invalid("candidate route/hash pairs must be unique");
        }
    }
    let mut attempts = BTreeSet::new();
    for decision in &draft.decisions {
        if decision.attempt == 0 || !attempts.insert(decision.attempt) {
            return invalid("decision attempts must be positive and unique");
        }
        validate_hash(&decision.manifest_hash)?;
        validate_hash(&decision.candidate_hash)?;
        if !candidates.contains(&(decision.route.clone(), decision.candidate_hash.clone())) {
            return invalid("every decision must reference a materialized candidate");
        }
    }
    Ok(())
}

fn validate_reservations(draft: &RunReceiptDraftV2) -> Result<(), RunReceiptError> {
    let mut reservations = BTreeMap::new();
    for reservation in &draft.reservations {
        validate_id(&reservation.id)?;
        if reservation.amount == BudgetAmount::default()
            || reservations
                .insert(reservation.id.as_str(), reservation.amount)
                .is_some()
        {
            return invalid("reservations must be non-zero and uniquely identified");
        }
        if matches!(reservation.kind, RunReservationKindV2::Attempt)
            && reservation.amount.requests != 1
        {
            return invalid("an attempt reserves exactly one request");
        }
        if matches!(reservation.kind, RunReservationKindV2::Wait)
            && (reservation.amount.requests != 0
                || reservation.amount.input_tokens != 0
                || reservation.amount.output_tokens != 0
                || reservation.amount.wall_time_ms == 0)
        {
            return invalid("a wait reserves wall time only");
        }
    }
    validate_consumptions(draft, &reservations)
}

fn validate_consumptions(
    draft: &RunReceiptDraftV2,
    reservations: &BTreeMap<&str, BudgetAmount>,
) -> Result<(), RunReceiptError> {
    let mut consumed = BTreeSet::new();
    for consumption in &draft.consumptions {
        validate_id(&consumption.id)?;
        if !consumed.insert(consumption.id.as_str()) {
            return invalid("a reservation can have at most one final consumption");
        }
        let Some(reserved) = reservations.get(consumption.id.as_str()) else {
            return invalid("every consumption must reference a reservation");
        };
        if !componentwise_lte(consumption.amount, *reserved) {
            return invalid("consumption cannot exceed its reservation");
        }
        if !consumption.known && consumption.amount != *reserved {
            return invalid("ambiguous consumption must retain the full reservation");
        }
    }
    Ok(())
}

fn validate_transitions(draft: &RunReceiptDraftV2) -> Result<(), RunReceiptError> {
    if draft.transitions.is_empty() {
        return invalid("receipt transitions cannot be empty");
    }
    let mut previous_at = draft.created_at;
    for (index, transition) in draft.transitions.iter().enumerate() {
        if transition.sequence != u64::try_from(index).unwrap_or(u64::MAX)
            || transition.at < previous_at
        {
            return invalid("transition sequence and time must be monotonic");
        }
        previous_at = transition.at;
    }
    let expected = match draft.final_phase {
        RunPhaseV2::Completed => RunJournalKindV2::InvocationSucceeded,
        RunPhaseV2::Failed => RunJournalKindV2::InvocationFailed,
        RunPhaseV2::OutcomeUnknown => RunJournalKindV2::OutcomeUnknown,
        _ => return invalid("receipt final phase is not terminal"),
    };
    if draft.transitions.last().map(|event| event.kind) != Some(expected) {
        return invalid("last transition does not match final phase");
    }
    Ok(())
}

fn validate_results(draft: &RunReceiptDraftV2) -> Result<(), RunReceiptError> {
    let decisions: BTreeMap<_, _> = draft
        .decisions
        .iter()
        .map(|decision| (decision.attempt, &decision.route))
        .collect();
    let mut attempts = BTreeSet::new();
    for result in &draft.results {
        if result.attempt == 0 || !attempts.insert(result.attempt) {
            return invalid("result attempts must be positive and unique");
        }
        if decisions.get(&result.attempt).copied() != Some(&result.route) {
            return invalid("every result must match an attempt decision");
        }
        if let Some(hash) = &result.provider_manifest_hash {
            validate_hash(hash)?;
        }
        if result.outcome_unknown
            && (result.output.is_some()
                || result.failure.is_some()
                || result.usage != TokenUsage::default())
        {
            return invalid("an ambiguous result cannot claim output, usage or failure");
        }
        if result.failure.is_some() && result.output.is_some() {
            return invalid("a failed result cannot claim successful output");
        }
    }
    Ok(())
}

pub(super) fn calculate_hash(draft: &RunReceiptDraftV2) -> Result<String, RunReceiptError> {
    let bytes = serde_json::to_vec(draft).map_err(RunReceiptError::Json)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn componentwise_lte(left: BudgetAmount, right: BudgetAmount) -> bool {
    left.requests <= right.requests
        && left.input_tokens <= right.input_tokens
        && left.output_tokens <= right.output_tokens
        && left.wall_time_ms <= right.wall_time_ms
}

fn validate_hash(hash: &str) -> Result<(), RunReceiptError> {
    let Some(hex) = hash.strip_prefix("sha256:") else {
        return invalid("hash must use the sha256 prefix");
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return invalid("hash must contain exactly 64 hexadecimal digits");
    }
    Ok(())
}

pub(super) fn validate_id(id: &str) -> Result<(), RunReceiptError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return invalid("identifier contains unsupported characters");
    }
    Ok(())
}

fn invalid<T>(message: &str) -> Result<T, RunReceiptError> {
    Err(RunReceiptError::Invalid(message.to_string()))
}
