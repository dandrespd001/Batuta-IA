//! Ejecución supervisada de una delegación.
//!
//! El reparto con el recibo: **aquí se recogen los hechos, allí se concluye el
//! veredicto**. Este crate no decide si una corrida vale; produce lo que hizo
//! falta para saberlo —`argv` real, código de salida, stderr íntegro, ficheros de
//! corrida, procedencia observada— y `batuta-receipt` lo sella.

pub mod error;
pub mod materialize;
pub mod provenance;
pub mod run;
pub mod substitution;

pub use error::ExecError;
pub use materialize::materialize;
pub use provenance::{parse_log, project_key, read_after, snapshot};
pub use run::{RunOutcome, build_env, run};
pub use substitution::{RunContext, resolve, resolve_argv};
