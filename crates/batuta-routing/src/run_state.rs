use std::fmt;

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

use crate::{HandoffCheckpoint, RecoveryAction};

/// Estado durable de una ejecución con como máximo una ruta activa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Todavía no se eligió ni arrancó ruta.
    Planned,
    /// Única ruta activa.
    Running {
        /// Ruta activa.
        route: RouteRef,
        /// Checkpoint recibido de la ruta anterior, si hubo relevo.
        handoff: Option<HandoffCheckpoint>,
    },
    /// Reintento o sonda programados sobre la misma ruta.
    WaitingRetry {
        /// Ruta que se conserva.
        route: RouteRef,
        /// Instante mínimo de reanudación.
        at: u64,
        /// Estado compacto del trabajo.
        checkpoint: HandoffCheckpoint,
    },
    /// Trabajo detenido y listo para elegir fallback.
    Checkpointed {
        /// Estado compacto del trabajo.
        checkpoint: HandoffCheckpoint,
    },
    /// Fallback elegido pero aún no arrancado.
    FallbackSelected {
        /// Ruta siguiente.
        route: RouteRef,
        /// Estado que recibirá.
        checkpoint: HandoffCheckpoint,
    },
    /// Ejecución terminada.
    Completed {
        /// Ruta que produjo el resultado.
        route: RouteRef,
    },
}

/// Evento explícito que puede mover la máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvent {
    /// Arranca la primera ruta.
    Start {
        /// Ruta elegida.
        route: RouteRef,
    },
    /// La ruta activa terminó bien.
    Success,
    /// La ruta activa falló y dejó checkpoint.
    Failure {
        /// Relevo compacto.
        checkpoint: HandoffCheckpoint,
        /// Política de recuperación ya clasificada.
        recovery: RecoveryAction,
    },
    /// Se escogió un fallback después del checkpoint.
    SelectFallback {
        /// Ruta elegida.
        route: RouteRef,
    },
    /// Reanuda una ruta seleccionada o cuyo plazo ya venció.
    Resume {
        /// Hora UTC Unix para validar esperas.
        now: u64,
    },
}

/// Aplica una transición pura; nunca ejecuta ni arranca en paralelo.
///
/// # Errors
///
/// Si el evento no pertenece al estado actual o intenta otra ruta activa.
pub fn advance_run(state: &RunState, event: RunEvent) -> Result<RunState, RunTransitionError> {
    match (state, event) {
        (RunState::Planned, RunEvent::Start { route }) => Ok(RunState::Running {
            route,
            handoff: None,
        }),
        (RunState::Running { route, .. }, RunEvent::Success) => Ok(RunState::Completed {
            route: route.clone(),
        }),
        (
            RunState::Running { route, .. },
            RunEvent::Failure {
                checkpoint,
                recovery,
            },
        ) => match recovery {
            RecoveryAction::RetrySameRoute { at } | RecoveryAction::ProbeSameRoute { at } => {
                Ok(RunState::WaitingRetry {
                    route: route.clone(),
                    at,
                    checkpoint,
                })
            }
            RecoveryAction::FallbackImmediately => Ok(RunState::Checkpointed { checkpoint }),
        },
        (RunState::Checkpointed { checkpoint }, RunEvent::SelectFallback { route }) => {
            Ok(RunState::FallbackSelected {
                route,
                checkpoint: checkpoint.clone(),
            })
        }
        (RunState::FallbackSelected { route, checkpoint }, RunEvent::Resume { .. }) => {
            Ok(RunState::Running {
                route: route.clone(),
                handoff: Some(checkpoint.clone()),
            })
        }
        (
            RunState::WaitingRetry {
                route,
                at,
                checkpoint,
            },
            RunEvent::Resume { now },
        ) if now >= *at => Ok(RunState::Running {
            route: route.clone(),
            handoff: Some(checkpoint.clone()),
        }),
        (current, attempted) => Err(RunTransitionError {
            state: state_name(current),
            event: event_name(&attempted),
        }),
    }
}

/// Evento que no es válido en el estado actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTransitionError {
    state: &'static str,
    event: &'static str,
}

impl fmt::Display for RunTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot apply '{}' while run is '{}'; one active route is allowed",
            self.event, self.state
        )
    }
}

impl std::error::Error for RunTransitionError {}

const fn state_name(state: &RunState) -> &'static str {
    match state {
        RunState::Planned => "planned",
        RunState::Running { .. } => "running",
        RunState::WaitingRetry { .. } => "waiting_retry",
        RunState::Checkpointed { .. } => "checkpointed",
        RunState::FallbackSelected { .. } => "fallback_selected",
        RunState::Completed { .. } => "completed",
    }
}

const fn event_name(event: &RunEvent) -> &'static str {
    match event {
        RunEvent::Start { .. } => "start",
        RunEvent::Success => "success",
        RunEvent::Failure { .. } => "failure",
        RunEvent::SelectFallback { .. } => "select_fallback",
        RunEvent::Resume { .. } => "resume",
    }
}
