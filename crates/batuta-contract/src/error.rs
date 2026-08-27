//! El error paraguas del contrato.
//!
//! Cada error de este crate se puede usar solo —y así es más preciso—, pero un
//! crate de arriba que mezcla vocabularios, identificadores y `TaskSpec` en la
//! misma función necesita un tipo donde converjan. [`ContractError`] es ese
//! tipo, y no añade texto propio: delega el `Display` en la causa para que el
//! mensaje que lee el Arquitecto sea el mismo que produjo el fallo.

use core::fmt;

use crate::ids::{IdentifierError, SchemaVersionError};
use crate::task::TaskSpecError;
use crate::vocabulary::VocabularyError;

/// Cualquier incumplimiento del contrato de batuta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// Un valor fuera de un vocabulario cerrado (R8).
    Vocabulary(VocabularyError),
    /// Un identificador mal formado.
    Identifier(IdentifierError),
    /// Una versión de esquema que batuta no sabe leer (R1).
    SchemaVersion(SchemaVersionError),
    /// Un encargo incoherente.
    TaskSpec(TaskSpecError),
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vocabulary(error) => write!(f, "{error}"),
            Self::Identifier(error) => write!(f, "{error}"),
            Self::SchemaVersion(error) => write!(f, "{error}"),
            Self::TaskSpec(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for ContractError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Vocabulary(error) => Some(error),
            Self::Identifier(error) => Some(error),
            Self::SchemaVersion(error) => Some(error),
            Self::TaskSpec(error) => Some(error),
        }
    }
}

impl From<VocabularyError> for ContractError {
    fn from(error: VocabularyError) -> Self {
        Self::Vocabulary(error)
    }
}

impl From<IdentifierError> for ContractError {
    fn from(error: IdentifierError) -> Self {
        Self::Identifier(error)
    }
}

impl From<SchemaVersionError> for ContractError {
    fn from(error: SchemaVersionError) -> Self {
        Self::SchemaVersion(error)
    }
}

impl From<TaskSpecError> for ContractError {
    fn from(error: TaskSpecError) -> Self {
        Self::TaskSpec(error)
    }
}
