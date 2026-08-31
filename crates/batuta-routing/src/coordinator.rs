//! Coordinación durable K4, dividida por responsabilidad.

#![allow(clippy::missing_errors_doc)]

mod attempt;
mod model;
mod recovery;
mod runtime;
mod service;
mod support;

pub use model::{
    RunAttemptV2, RunJournalEventV2, RunJournalKindV2, RunNextActionV2, RunPhaseV2, RunRequestV2,
    RunStatusV2,
};
pub use runtime::{RunClock, RunSleeper};
pub use service::RunCoordinator;
pub use support::RunCoordinatorError;
