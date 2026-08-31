//! Ejecución supervisada de una delegación.
//!
//! El reparto con el recibo: **aquí se recogen los hechos, allí se concluye el
//! veredicto**. Este crate no decide si una corrida vale; produce lo que hizo
//! falta para saberlo —`argv` real, código de salida, stderr íntegro, ficheros de
//! corrida, procedencia observada— y `batuta-receipt` lo sella.

pub mod canary;
pub mod error;
pub mod executor;
pub mod manifest_executor;
pub mod materialize;
pub mod profile;
pub mod provenance;
pub mod run;
pub mod substitution;

pub use canary::{
    CanaryRequest, capability_was_observed, generate_token, run_canary, run_capability_canary,
};
pub use error::ExecError;
pub use executor::{
    ExecutorError, FakeHarnessExecutor, HarnessExecutor, InvocationFailure, InvocationRequestV2,
    NormalizedInvocationResult, ProcessHarnessExecutor, TokenUsage,
};
pub use manifest_executor::{ManifestExecutorError, ManifestHarnessExecutor};
pub use materialize::materialize;
pub use profile::{ExecutionProfileDraftV1, ExecutionProfileError, ExecutionProfileV1};
pub use provenance::{parse_log, project_key, read_after, read_stderr, snapshot};
pub use run::{RunOutcome, build_env, run};
pub use substitution::{RunContext, resolve, resolve_argv};
