//! Retry, relevo, salud, persistencia y recibo terminal.

use std::collections::BTreeSet;
use std::path::PathBuf;

use batuta_contract::RouteRef;
use batuta_exec::{HarnessExecutor, InvocationFailure};

use super::model::{RunJournalKindV2, RunNextActionV2, RunPhaseV2, RunStatusV2};
use super::service::RunCoordinator;
use super::support::{
    RunCoordinatorError, checkpoint_for, refresh_budget, reservation_for, reservation_id,
    upsert_consumption, validate_status,
};
use crate::snapshot_store::atomic_write;
use crate::{
    BudgetAmount, BudgetError, ExecutionGrantV1, HealthObservationV2, HealthOutcomeV2, LedgerStore,
    RouteDecision, RunConsumptionReceiptV2, RunReceiptDraftV2, RunReceiptError,
    RunReceiptReferenceV2, RunReceiptStoreV2, RunReceiptV2, RunReservationKindV2,
    RunReservationReceiptV2, StateStore,
};

impl<E: HarnessExecutor + ?Sized> RunCoordinator<'_, E> {
    pub(super) fn recover_known_failure(
        &self,
        grant: &ExecutionGrantV1,
        status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let failure = status.failure.ok_or_else(|| {
            RunCoordinatorError::Invalid("failed attempt has no failure".to_string())
        })?;
        if let InvocationFailure::RateLimited {
            retry_after_ms: Some(wait_ms),
        } = failure
            && wait_ms > 0
            && wait_ms <= status.execution_policy.max_retry_after_ms
            && status.attempts.len()
                < usize::try_from(status.execution_policy.max_attempts).unwrap_or(usize::MAX)
        {
            let now_ms = self.clock.now_millis();
            let due = now_ms.saturating_add(wait_ms);
            let attempt_amount = reservation_for(&status.request)?;
            if due.saturating_add(attempt_amount.wall_time_ms) <= status.deadline_at_ms
                && self.authorize(&grant.id, now_ms).is_ok()
            {
                let route = status.route.clone().ok_or_else(|| {
                    RunCoordinatorError::Invalid("retry has no route".to_string())
                })?;
                let service = self.load_service(now_ms)?;
                if let Ok(decision) = service.route_with_allowed_routes(
                    status.request.routing.clone(),
                    &BTreeSet::from([route]),
                    &BTreeSet::new(),
                ) {
                    return self.plan_retry(grant, status, decision, wait_ms, due);
                }
            }
        }
        self.plan_fallback(grant, status)
    }

    fn plan_retry(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
        decision: RouteDecision,
        wait_ms: u64,
        due: u64,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let fallback_status = status.clone();
        self.append_attempt(&mut status, decision)?;
        let attempt = status.attempts.last().unwrap().clone();
        let wait_id = reservation_id(&status.id, 'w', attempt.number);
        let planned_reservations = [
            RunReservationReceiptV2 {
                id: wait_id.clone(),
                kind: RunReservationKindV2::Wait,
                amount: BudgetAmount {
                    wall_time_ms: wait_ms,
                    ..BudgetAmount::default()
                },
            },
            RunReservationReceiptV2 {
                id: attempt.reservation_id.clone(),
                kind: RunReservationKindV2::Attempt,
                amount: attempt.reserved,
            },
        ];
        if !self.reserve_retry_if_available(grant, &planned_reservations)? {
            return self.plan_fallback(grant, fallback_status);
        }
        status.next_action_at = Some(due);
        status.next_action = Some(RunNextActionV2::RetrySameRoute {
            route: attempt.route,
            not_before_ms: due,
            wait_ms,
            wait_reservation_id: wait_id,
            attempt_reservation_id: attempt.reservation_id,
            attempt: attempt.number,
        });
        self.save(&status)?;
        self.activate_retry(grant, status)
    }

    fn reserve_retry_if_available(
        &self,
        grant: &ExecutionGrantV1,
        reservations: &[RunReservationReceiptV2],
    ) -> Result<bool, RunCoordinatorError> {
        let pairs = reservations
            .iter()
            .map(|item| (item.id.clone(), item.amount))
            .collect::<Vec<_>>();
        match self.ledger().reserve_many(grant, &pairs) {
            Ok(_) | Err(BudgetError::DuplicateReservation(_)) => Ok(true),
            Err(BudgetError::Exceeded) => Ok(false),
            Err(error) => Err(RunCoordinatorError::Budget(error.to_string())),
        }
    }

    pub(super) fn activate_retry(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let RunNextActionV2::RetrySameRoute {
            route,
            not_before_ms,
            wait_ms,
            wait_reservation_id,
            attempt_reservation_id,
            attempt,
        } = status.next_action.clone().ok_or_else(|| {
            RunCoordinatorError::Invalid("retry plan has no next action".to_string())
        })?;
        let current = status
            .attempts
            .iter()
            .find(|item| item.number == attempt)
            .ok_or_else(|| RunCoordinatorError::Invalid("retry attempt is missing".to_string()))?;
        if current.reservation_id != attempt_reservation_id || current.route != route {
            return Err(RunCoordinatorError::Invalid(
                "retry plan does not match its attempt".to_string(),
            ));
        }
        let attempt_amount = current.reserved;
        self.authorize(&grant.id, self.clock.now_millis())?;
        let reservations = [
            RunReservationReceiptV2 {
                id: wait_reservation_id,
                kind: RunReservationKindV2::Wait,
                amount: BudgetAmount {
                    wall_time_ms: wait_ms,
                    ..BudgetAmount::default()
                },
            },
            RunReservationReceiptV2 {
                id: attempt_reservation_id,
                kind: RunReservationKindV2::Attempt,
                amount: attempt_amount,
            },
        ];
        self.ensure_reservations(grant, &mut status, &reservations)?;
        status.phase = RunPhaseV2::Reserved;
        status.push(
            self.clock.now_millis(),
            RunJournalKindV2::Reserved,
            Some(route.clone()),
        );
        self.save(&status)?;
        status.phase = RunPhaseV2::WaitingRetry;
        status.push(
            self.clock.now_millis(),
            RunJournalKindV2::RetryScheduled,
            Some(route),
        );
        self.save(&status)?;
        let now_ms = self.clock.now_millis();
        if now_ms < not_before_ms {
            self.sleeper.sleep_millis(not_before_ms - now_ms);
        }
        self.continue_waiting(grant, status)
    }

    pub(super) fn continue_waiting(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        let RunNextActionV2::RetrySameRoute {
            route,
            not_before_ms,
            wait_ms,
            wait_reservation_id,
            ..
        } = status.next_action.clone().ok_or_else(|| {
            RunCoordinatorError::Invalid("waiting run has no retry action".to_string())
        })?;
        let now_ms = self.clock.now_millis();
        if now_ms < not_before_ms {
            return Err(RunCoordinatorError::ProbeNotDue {
                next_action_at: not_before_ms,
            });
        }
        self.authorize(&grant.id, now_ms)?;
        let wait_amount = BudgetAmount {
            wall_time_ms: wait_ms,
            ..BudgetAmount::default()
        };
        let ledger_status = self
            .ledger()
            .confirm(grant, &wait_reservation_id, wait_amount)
            .map_err(|error| RunCoordinatorError::Budget(error.to_string()))?;
        upsert_consumption(
            &mut status,
            RunConsumptionReceiptV2 {
                id: wait_reservation_id,
                amount: wait_amount,
                known: true,
            },
        )?;
        refresh_budget(&mut status, &ledger_status)?;
        status.next_action = None;
        status.next_action_at = None;
        status.phase = RunPhaseV2::Reserved;
        status.route = Some(route.clone());
        status.push(now_ms, RunJournalKindV2::RetryElapsed, Some(route));
        self.save(&status)?;
        self.invoke_current(grant, status)
    }

    pub(super) fn plan_fallback(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        if status.attempts.len()
            >= usize::try_from(status.execution_policy.max_attempts).unwrap_or(usize::MAX)
            || status.handoffs >= status.execution_policy.max_handoffs
        {
            return self.finish_failed(grant, status);
        }
        let now_ms = self.clock.now_millis();
        let Ok(active_grant) = self.authorize(&grant.id, now_ms) else {
            return self.finish_failed(grant, status);
        };
        let attempted = status
            .attempts
            .iter()
            .map(|attempt| attempt.route.clone())
            .collect::<BTreeSet<_>>();
        let service = self.load_service(now_ms)?;
        let Ok(decision) = service.route_with_allowed_routes(
            status.request.routing.clone(),
            &active_grant.routes,
            &attempted,
        ) else {
            return self.finish_failed(grant, status);
        };
        if status.phase != RunPhaseV2::HandoffReady {
            let checkpoint = checkpoint_for(&status, grant, &decision.route)?;
            status.checkpoint = Some(checkpoint.clone());
            status.checkpoints.push(checkpoint);
            status.handoffs = status.handoffs.saturating_add(1);
            status.phase = RunPhaseV2::HandoffReady;
            status.push(
                now_ms,
                RunJournalKindV2::HandoffCreated,
                status.route.clone(),
            );
            self.save(&status)?;
        }
        self.append_attempt(&mut status, decision)?;
        self.save(&status)?;
        self.reserve_current(grant, status)
            .and_then(|status| self.invoke_current(grant, status))
    }

    fn finish_failed(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        status.phase = RunPhaseV2::Failed;
        status.next_action = None;
        status.next_action_at = None;
        self.save(&status)?;
        self.finalize_receipt(grant, status)
    }

    pub(super) fn record_health(
        &self,
        route: &RouteRef,
        outcome: HealthOutcomeV2,
        latency_ms: u64,
        now_ms: u64,
    ) -> Result<(), RunCoordinatorError> {
        StateStore::open(self.root.join("state-v2"))
            .record_health_observation(
                route,
                &HealthObservationV2 {
                    at: now_ms / 1_000,
                    outcome,
                    latency_ms,
                },
            )
            .map(|_| ())
            .map_err(|error| RunCoordinatorError::State(error.to_string()))
    }

    pub(super) fn finalize_receipt(
        &self,
        grant: &ExecutionGrantV1,
        mut status: RunStatusV2,
    ) -> Result<RunStatusV2, RunCoordinatorError> {
        if !matches!(
            status.phase,
            RunPhaseV2::Completed | RunPhaseV2::Failed | RunPhaseV2::OutcomeUnknown
        ) {
            return Err(RunCoordinatorError::Invalid(
                "cannot seal a non-terminal run".to_string(),
            ));
        }
        let store = RunReceiptStoreV2::open(self.root.join("run-receipts"));
        let receipt = match store.load(&status.id) {
            Ok(receipt) => receipt,
            Err(RunReceiptError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                let receipt = RunReceiptV2::seal(RunReceiptDraftV2 {
                    schema_version: 2,
                    id: status.id.clone(),
                    created_at: status.created_at,
                    request: status.request.clone(),
                    grant: grant.clone(),
                    grant_hash: status.grant_hash.clone(),
                    candidates: status.candidates.clone(),
                    discards: status.discards.clone(),
                    decisions: status.decisions.clone(),
                    reservations: status.reservations.clone(),
                    consumptions: status.consumptions.clone(),
                    transitions: status.journal.clone(),
                    results: status.results.clone(),
                    checkpoints: status.checkpoints.clone(),
                    final_phase: status.phase,
                })
                .map_err(|error| RunCoordinatorError::Receipt(error.to_string()))?;
                match store.append(&receipt) {
                    Ok(()) => receipt,
                    Err(RunReceiptError::AlreadyExists(_)) => store
                        .load(&status.id)
                        .map_err(|error| RunCoordinatorError::Receipt(error.to_string()))?,
                    Err(error) => return Err(RunCoordinatorError::Receipt(error.to_string())),
                }
            }
            Err(error) => return Err(RunCoordinatorError::Receipt(error.to_string())),
        };
        if receipt.grant_hash != status.grant_hash {
            return Err(RunCoordinatorError::Receipt(
                "existing receipt belongs to another grant".to_string(),
            ));
        }
        status.receipt = Some(RunReceiptReferenceV2 {
            id: receipt.id,
            receipt_hash: receipt.receipt_hash,
        });
        self.save(&status)?;
        Ok(status)
    }

    pub(super) fn ledger(&self) -> LedgerStore {
        LedgerStore::open(self.root.join("ledger"), self.root.join("budget-leases"))
    }

    pub(super) fn save(&self, status: &RunStatusV2) -> Result<(), RunCoordinatorError> {
        validate_status(status, &status.id)?;
        let mut bytes = serde_json::to_vec(status).map_err(RunCoordinatorError::Json)?;
        bytes.push(b'\n');
        atomic_write(&self.run_path(&status.id), &bytes).map_err(RunCoordinatorError::Io)
    }

    pub(super) fn run_path(&self, id: &str) -> PathBuf {
        self.root.join("runs").join(format!("{id}.json"))
    }
}
