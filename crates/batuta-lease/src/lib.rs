//! Admisión por leases.
//!
//! Dos espacios —modelo y repositorio—, exclusión sin espera, y **caducidad por
//! evidencia: un lease se reclama sólo si se puede demostrar que su dueño murió,
//! nunca porque sea viejo**.
//!
//! La inspección no toma cerrojos, así que no puede hacer cola detrás de una
//! delegación (R9).

pub mod error;
pub mod lease;
pub mod owner;

pub use error::LeaseError;
pub use lease::{LeaseGuard, LeaseRecord, LeaseSpace, LeaseStore};
pub use owner::Owner;
