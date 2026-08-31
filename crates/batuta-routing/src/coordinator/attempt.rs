//! Planificación, reserva y cierre de una única invocación.

use std::collections::BTreeSet;

use batuta_exec::{HarnessExecutor, InvocationRequestV2, NormalizedInvocationResult, TokenUsage};

use super::model::{RunAttemptV2, RunJournalKindV2, RunPhaseV2, RunStatusV2};
use super::service::RunCoordinator;
use super::support::{
    RunCoordinatorError, candidate_hash, invocation_objective, refresh_budget, reservation_for,
    reservation_id, upsert_consumption,
};
use crate::{
    BudgetAmount, BudgetError, ExecutionGrantV1, GrantOperation, HealthOutcomeV2, RouteDecision,
    RunCandidateReceiptV2, RunConsumptionReceiptV2, RunDecisionReceiptV2, RunReservationKindV2,
    RunReservationReceiptV2, RunResultReceiptV2,
};

impl<E: HarnessExecutor + ?Sized> RunCoordinator<'_, E> {
    pub(super) fn append_attempt(
        &self,
        status: &mut RunStatusV2,
        decision: RouteDecision,
    ) -> Result<(), RunCoordinatorError> {
        let number = u32::try_from(status.attempts.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| RunCoordinatorError::Invalid("attempt overflow".to_string()))?;
        if number > status.execution_policy.max_attempts {
            return Err(RunCoordinatorError::Invalid(
                "execution policy attempt limit reached".to_string(),
            ));
        }
        let amount = reservation_for(&status.request)?;
        let candidate_hash = candidate_hash(&decision, &status.request.routing.request.action)?;
        let candidate = RunCandidateReceiptV2 {
            route: decision.route.clone(),
            action: status.request.routing.request.action.clone(),
            candidate_hash: candidate_hash.clone(),
        };
        if !status.candidates.contains(&candidate) {
            status.candidates.push(candidate);
        }
        status.discards.extend(decision.discarded.clone());
        status.decisions.push(RunDecisionReceiptV2 {
            attempt: u64::from(number),
            manifest_hash: decision.manifest_hash.clone(),
            route: decision.route.clone(),
            candidate_hash: candidate_hash.clone(),
        });
        status.attempts.push(RunAttemptV2 {
            number,
            route: decision.route.clone(),
            state_manifest_hash: decision.manifest_hash,
            candidate_hash,
            provider_manifest_hash: None,
            reservation_id: reservation_id(&status.id, 'a', number),
            reserved: amount,
            started_at_ms: None,
            finished_at_ms: None,
            failure: None,
            outcome_unknown: false,
            health_recorded: false,
        });
        status.route = Some(decision.route.clone());
        status.phase = RunPhaseV2::Planned;
        status.push(
            self.clock.now_millis(),
            RunJournalKindV2::Planned,
            Some(decision.route),
        );
        Ok(())
    }

    pub(super) fn reserve_current(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let attempt = status
            .attempts
            .last()
            .ok_or_else(|| RunCoordinatorError::Invalid("run has no planned attempt".to_string()))?
            .clone();
        self.authorize(&grant.id, self.clock.now_millis())?;
        self.ensure_reservations(
            grant,
            &mut status,
            &[RunReservationReceiptV2 {
                id: attempt.reservation_id,
                kind: RunReservationKindV2::Attempt,
                amount: attempt.reserved,
            }],
        )?;
        status.phase = RunPhaseV2::Reserved;
        status.push(
            self.clock.now_millis(),
            RunJournalKindV2::Reserved,
            Some(attempt.route),
        );
        self.save(&status)?;
        Ok(status)
    }

    pub(super) fn ensure_reservations(
        &self,
        grant: &ExecutionGrantV1,
        status: &mut RunStatusV2,
        requested: &[RunReservationReceiptV2],
    ) -> Result<(), RunCoordinatorError> {
        let ledger = self.ledger();
        let pairs = requested
            .iter()
            .map(|item| (item.id.clone(), item.amount))
            .collect::<Vec<_>>();
        let ledger_status = match ledger.reserve_many(grant, &pairs) {
            Ok(ledger_status) => ledger_status,
            Err(BudgetError::DuplicateReservation(_)) => {
                let existing = ledger
                    .status(&grant.id)
                    .map_err(|error| RunCoordinatorError::Budget(error.to_string()))?;
                for item in requested {
                    let Some(reservation) = existing.reservations.get(&item.id) else {
                        return Err(RunCoordinatorError::Budget(
                            "partial durable reservation set detected".to_string(),
                        ));
                    };
                    if reservation.reserved != item.amount {
                        return Err(RunCoordinatorError::Budget(
                            "durable reservation amount changed".to_string(),
                        ));
                    }
                }
                existing
            }
            Err(error) => return Err(RunCoordinatorError::Budget(error.to_string())),
        };
        for item in requested {
            if let Some(existing) = status
                .reservations
                .iter()
                .find(|existing| existing.id == item.id)
            {
                if existing != item {
                    return Err(RunCoordinatorError::Invalid(
                        "status reservation conflicts with ledger".to_string(),
                    ));
                }
            } else {
                status.reservations.push(item.clone());
            }
        }
        refresh_budget(status, &ledger_status)?;
        Ok(())
    }

    pub(super) fn invoke_current(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let now_ms = self.clock.now_millis();
        let active_grant = self.authorize(&grant.id, now_ms)?;
        let route = status
            .route
            .clone()
            .ok_or_else(|| RunCoordinatorError::Invalid("run has no route".to_string()))?;
        if !active_grant.permits(
            &route,
            &status.request.routing.request.action,
            GrantOperation::Run,
        ) {
            return Err(RunCoordinatorError::Grant(
                "grant no longer permits this exact invocation".to_string(),
            ));
        }
        self.refresh_current_selection(&mut status, &active_grant, now_ms)?;
        let (reservation_id, reserved, number) = {
            let attempt = status.attempts.last().ok_or_else(|| {
                RunCoordinatorError::Invalid("run has no current attempt".to_string())
            })?;
            (
                attempt.reservation_id.clone(),
                attempt.reserved,
                attempt.number,
            )
        };
        status.phase = RunPhaseV2::InvocationStarted;
        status.attempts.last_mut().unwrap().started_at_ms = Some(now_ms);
        status.push(
            now_ms,
            RunJournalKindV2::InvocationStarted,
            Some(route.clone()),
        );
        self.save(&status)?;
        let invocation = InvocationRequestV2 {
            run_id: status.id.clone(),
            route,
            objective: invocation_objective(&status)?,
            task: status.request.task.clone(),
            max_output_bytes: reserved.output_tokens.saturating_mul(8).max(1),
            timeout_ms: reserved.wall_time_ms,
        };
        match self.executor.invoke(&invocation) {
            Ok(result) => self.finish_known(grant, status, reservation_id, number, result),
            Err(_) => self.finish_unknown(grant, status),
        }
    }

    fn refresh_current_selection(
        &self,
        status: &mut RunStatusV2,
        grant: &ExecutionGrantV1,
        now_ms: u64,
    ) -> Result<(), RunCoordinatorError> {
        let route = status
            .route
            .clone()
            .ok_or_else(|| RunCoordinatorError::Invalid("run has no route".to_string()))?;
        let service = self.load_service(now_ms)?;
        let decision = service
            .route_with_allowed_routes(
                status.request.routing.clone(),
                &BTreeSet::from([route.clone()]),
                &BTreeSet::new(),
            )
            .map_err(|error| RunCoordinatorError::Route(error.to_string()))?;
        if !grant.routes.contains(&decision.route) {
            return Err(RunCoordinatorError::Grant(
                "current manifest selected a route outside the grant".to_string(),
            ));
        }
        let hash = candidate_hash(&decision, &status.request.routing.request.action)?;
        let candidate = RunCandidateReceiptV2 {
            route: decision.route.clone(),
            action: status.request.routing.request.action.clone(),
            candidate_hash: hash.clone(),
        };
        if !status.candidates.contains(&candidate) {
            status.candidates.push(candidate);
        }
        let attempt = status.attempts.last_mut().ok_or_else(|| {
            RunCoordinatorError::Invalid("run has no current attempt".to_string())
        })?;
        attempt
            .state_manifest_hash
            .clone_from(&decision.manifest_hash);
        attempt.candidate_hash.clone_from(&hash);
        let receipt_decision = status.decisions.last_mut().ok_or_else(|| {
            RunCoordinatorError::Invalid("run has no current decision".to_string())
        })?;
        receipt_decision.manifest_hash = decision.manifest_hash;
        receipt_decision.candidate_hash = hash;
        status.discards.extend(decision.discarded);
        self.save(status)
    }

    fn finish_known(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
        reservation_id: String,
        number: u32,
        result: NormalizedInvocationResult,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let now_ms = self.clock.now_millis();
        let actual = BudgetAmount {
            requests: 1,
            input_tokens: result.usage.input_tokens,
            output_tokens: result.usage.output_tokens,
            wall_time_ms: result.latency_ms,
        };
        let Ok(ledger_status) = self.ledger().confirm(grant, &reservation_id, actual) else {
            return self.finish_unknown(grant, status);
        };
        upsert_consumption(
            &mut status,
            RunConsumptionReceiptV2 {
                id: reservation_id,
                amount: actual,
                known: true,
            },
        )?;
        refresh_budget(&mut status, &ledger_status)?;
        let route = status
            .route
            .clone()
            .ok_or_else(|| RunCoordinatorError::Invalid("known result has no route".to_string()))?;
        let failure = result.failure;
        status.results.push(RunResultReceiptV2 {
            attempt: u64::from(number),
            route: route.clone(),
            output: failure.is_none().then_some(result.output.clone()),
            usage: result.usage,
            latency_ms: result.latency_ms,
            provenance: result.provenance,
            provider_manifest_hash: result.manifest_hash.clone(),
            failure,
            outcome_unknown: false,
        });
        let attempt = status.attempts.last_mut().ok_or_else(|| {
            RunCoordinatorError::Invalid("known result has no attempt".to_string())
        })?;
        attempt.finished_at_ms = Some(now_ms);
        attempt.failure = failure;
        attempt.provider_manifest_hash = result.manifest_hash;
        status.failure = failure;
        if failure.is_none() {
            status.output = Some(result.output);
            status.phase = RunPhaseV2::Completed;
            status.push(
                now_ms,
                RunJournalKindV2::InvocationSucceeded,
                Some(route.clone()),
            );
        } else {
            status.output = None;
            status.phase = RunPhaseV2::AttemptFailed;
            status.push(
                now_ms,
                RunJournalKindV2::InvocationFailed,
                Some(route.clone()),
            );
        }
        self.save(&status)?;
        self.record_health(
            &route,
            if failure.is_none() {
                HealthOutcomeV2::KnownSuccess
            } else {
                HealthOutcomeV2::KnownFailure
            },
            result.latency_ms,
            now_ms,
        )?;
        status.attempts.last_mut().unwrap().health_recorded = true;
        self.save(&status)?;
        if failure.is_none() {
            self.finalize_receipt(grant, status)
        } else {
            self.recover_known_failure(grant, status)
        }
    }

    pub(super) fn finish_unknown(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let now_ms = self.clock.now_millis();
        let (reservation_id, reserved, number, route) = {
            let attempt = status.attempts.last().ok_or_else(|| {
                RunCoordinatorError::Invalid("ambiguous result has no attempt".to_string())
            })?;
            (
                attempt.reservation_id.clone(),
                attempt.reserved,
                attempt.number,
                attempt.route.clone(),
            )
        };
        let ledger_status = self
            .ledger()
            .mark_outcome_unknown(grant, &reservation_id)
            .map_err(|error| RunCoordinatorError::Budget(error.to_string()))?;
        upsert_consumption(
            &mut status,
            RunConsumptionReceiptV2 {
                id: reservation_id,
                amount: reserved,
                known: false,
            },
        )?;
        refresh_budget(&mut status, &ledger_status)?;
        status.results.push(RunResultReceiptV2 {
            attempt: u64::from(number),
            route: route.clone(),
            output: None,
            usage: TokenUsage::default(),
            latency_ms: reserved.wall_time_ms,
            provenance: None,
            provider_manifest_hash: None,
            failure: None,
            outcome_unknown: true,
        });
        let attempt = status.attempts.last_mut().unwrap();
        attempt.finished_at_ms = Some(now_ms);
        attempt.outcome_unknown = true;
        status.phase = RunPhaseV2::OutcomeUnknown;
        status.outcome_unknown = true;
        status.next_action = None;
        status.next_action_at = None;
        status.output = None;
        status.failure = None;
        status.push(
            now_ms,
            RunJournalKindV2::OutcomeUnknown,
            Some(route.clone()),
        );
        self.save(&status)?;
        self.record_health(
            &route,
            HealthOutcomeV2::Ambiguous,
            reserved.wall_time_ms,
            now_ms,
        )?;
        status.attempts.last_mut().unwrap().health_recorded = true;
        self.save(&status)?;
        self.finalize_receipt(grant, status)
    }
}
