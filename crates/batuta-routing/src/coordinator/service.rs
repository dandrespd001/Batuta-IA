//! Entrada pública y apertura de una generación confiable por operación.

use std::collections::BTreeSet;
use std::path::PathBuf;

use batuta_exec::HarnessExecutor;
use batuta_lease::{LeaseSpace, LeaseStore};

use super::model::{RunPhaseV2, RunRequestV2, RunStatusV2};
use super::runtime::{RunClock, RunSleeper, SYSTEM_CLOCK, SYSTEM_SLEEPER};
use super::support::{RunCoordinatorError, validate_id, validate_request, validate_status};
use crate::{ApplicationService, ExecutionGrantV1, GrantOperation, GrantStore, StateStore};

/// Coordinador configurado sólo con layout local, reloj, sleeper y ejecutor.
pub struct RunCoordinator<'a, E: HarnessExecutor + ?Sized> {
    pub(super) root: PathBuf,
    pub(super) executor: &'a E,
    pub(super) clock: &'a dyn RunClock,
    pub(super) sleeper: &'a dyn RunSleeper,
}

impl<'a, E: HarnessExecutor + ?Sized> RunCoordinator<'a, E> {
    /// Abre el coordinador con reloj y sleeper del sistema.
    pub fn open(root: PathBuf, executor: &'a E) -> Self {
        Self::with_runtime(root, executor, &SYSTEM_CLOCK, &SYSTEM_SLEEPER)
    }

    /// Abre el coordinador con efectos temporales inyectados.
    pub fn with_runtime(
        root: PathBuf,
        executor: &'a E,
        clock: &'a dyn RunClock,
        sleeper: &'a dyn RunSleeper,
    ) -> Self {
        Self {
            root,
            executor,
            clock,
            sleeper,
        }
    }

    /// Valida, selecciona, reserva y ejecuta hasta un estado terminal o espera.
    pub fn execute(&self, request: RunRequestV2) -> Result<RunStatusV2, RunCoordinatorError> {
        validate_request(&request)?;
        let leases = LeaseStore::open(&self.root.join("leases"))
            .map_err(|error| RunCoordinatorError::Lease(error.to_string()))?;
        let _guard = leases
            .acquire(
                LeaseSpace::Repository,
                &format!("run-{}", request.id),
                &request.id,
            )
            .map_err(|error| RunCoordinatorError::Lease(error.to_string()))?;
        if self.run_path(&request.id).exists() {
            return Err(RunCoordinatorError::AlreadyExists(request.id));
        }
        let now_ms = self.clock.now_millis();
        let grant = self.authorize(&request.grant_id, now_ms)?;
        if !grant.actions.contains(&request.routing.request.action)
            || !grant.operations.contains(&GrantOperation::Run)
        {
            return Err(RunCoordinatorError::Grant(
                "grant does not permit the exact action and run operation".to_string(),
            ));
        }
        let service = self.load_service(now_ms)?;
        let policy = service
            .execution_policy()
            .map_err(|error| RunCoordinatorError::Route(error.to_string()))?;
        let decision = service
            .route_with_allowed_routes(request.routing.clone(), &grant.routes, &BTreeSet::new())
            .map_err(|error| RunCoordinatorError::Route(error.to_string()))?;
        let mut status = RunStatusV2::empty(request, &grant, policy, now_ms);
        self.append_attempt(&mut status, decision)?;
        self.save(&status)?;
        self.reserve_current(&grant, status)
            .and_then(|status| self.invoke_current(&grant, status))
    }

    /// Continúa un estado durable sin duplicar una llamada ambigua.
    pub fn resume(&self, id: &str) -> Result<RunStatusV2, RunCoordinatorError> {
        validate_id(id)?;
        let leases = LeaseStore::open(&self.root.join("leases"))
            .map_err(|error| RunCoordinatorError::Lease(error.to_string()))?;
        let _guard = leases
            .acquire(LeaseSpace::Repository, &format!("run-{id}"), id)
            .map_err(|error| RunCoordinatorError::Lease(error.to_string()))?;
        let status = self.status(id)?;
        let grant = GrantStore::open(self.root.join("grants"))
            .status(&status.grant_id)
            .map_err(|error| RunCoordinatorError::Grant(error.to_string()))?
            .grant;
        match status.phase {
            RunPhaseV2::InvocationStarted => self.finish_unknown(&grant, status),
            RunPhaseV2::WaitingRetry => self.continue_waiting(&grant, status),
            RunPhaseV2::Planned if status.next_action.is_some() => {
                self.activate_retry(&grant, status)
            }
            RunPhaseV2::Planned => self
                .reserve_current(&grant, status)
                .and_then(|status| self.invoke_current(&grant, status)),
            RunPhaseV2::Reserved => self.invoke_current(&grant, status),
            RunPhaseV2::AttemptFailed => self.recover_known_failure(&grant, status),
            RunPhaseV2::HandoffReady => self.plan_fallback(&grant, status),
            RunPhaseV2::Completed | RunPhaseV2::Failed | RunPhaseV2::OutcomeUnknown => {
                self.finalize_receipt(&grant, status)
            }
        }
    }

    /// Alias compatible que aplica exactamente las reglas de `resume`.
    pub fn recover(&self, id: &str) -> Result<RunStatusV2, RunCoordinatorError> {
        self.resume(id)
    }

    /// Lee y valida el estado durable sin construir una invocación.
    pub fn status(&self, id: &str) -> Result<RunStatusV2, RunCoordinatorError> {
        validate_id(id)?;
        let bytes = std::fs::read(self.run_path(id)).map_err(RunCoordinatorError::Io)?;
        let status: RunStatusV2 =
            serde_json::from_slice(&bytes).map_err(RunCoordinatorError::Json)?;
        validate_status(&status, id)?;
        Ok(status)
    }

    pub(super) fn authorize(
        &self,
        grant_id: &str,
        now_ms: u64,
    ) -> Result<ExecutionGrantV1, RunCoordinatorError> {
        GrantStore::open(self.root.join("grants"))
            .authorize(grant_id, now_ms / 1_000)
            .map_err(|error| RunCoordinatorError::Grant(error.to_string()))
    }

    pub(super) fn load_service(
        &self,
        now_ms: u64,
    ) -> Result<ApplicationService, RunCoordinatorError> {
        ApplicationService::from_state_store(
            &StateStore::open(self.root.join("state-v2")),
            now_ms / 1_000,
            false,
        )
        .map_err(|error| RunCoordinatorError::State(error.to_string()))
    }
}
