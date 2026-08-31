//! Contrato de batuta: tipos, errores y vocabularios cerrados.
//!
//! Este crate no depende de ningún otro de batuta y todos dependen de él.
//!
//! # Cero E/S, demostrado
//!
//! El brief §4 exige que `batuta-contract` no haga entrada/salida. Aquí eso no
//! es una promesa: el crate es `no_std` + `alloc`, así que `std::fs`,
//! `std::net`, `std::process` y `std::time` **no están disponibles**. La
//! propiedad la comprueba el compilador en cada compilación, no una revisión.
//!
//! Es la misma idea que R2 aplicada al propio código: no se declara, se
//! demuestra.
//!
//! # Qué vive aquí
//!
//! - [`vocabulary`]: la maquinaria de R8 —vocabularios enumerables cuyos errores
//!   listan los valores válidos.
//! - [`vocabularies`]: los quince vocabularios cerrados.
//! - [`ids`]: identificadores validados —proveedor, modelo, credencial, ruta.
//! - [`task`]: el `TaskSpec` y sus invariantes.
//! - [`error`]: el error que reúne a todos los anteriores.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod ids;
pub mod route;
pub mod task;
pub mod vocabularies;
pub mod vocabulary;

pub use error::ContractError;
pub use ids::{
    CredentialName, EnvVarName, GateProfileId, IdentifierError, IdentifierProblem, ModelId,
    ProviderId, RelativePath, RouteModel, SchemaVersion, SchemaVersionError,
};
pub use route::{RouteRef, RouteRefError};
pub use task::{TaskSpec, TaskSpecDraft, TaskSpecError};
pub use vocabularies::{
    AuthMethod, CanaryExpectation, Capability, DocumentFormat, ExecutionProfile, OutputContract,
    ParserKind, PromptDelivery, ProvenanceSource, ProviderKind, ReasoningEffort, Role, Sensitivity,
    TrustTier, WriteMode,
};
pub use vocabulary::{ClosedVocabulary, VocabularyError};
