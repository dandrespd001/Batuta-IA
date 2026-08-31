use std::fmt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::FailureCategory;

/// Estado observado de una prueba en el punto de relevo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// Terminó en verde.
    Passed,
    /// Terminó en rojo.
    Failed,
    /// No se ejecutó todavía.
    NotRun,
}

/// Hecho compacto sobre una prueba.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestFact {
    /// Orden ejecutada.
    pub command: String,
    /// Resultado.
    pub status: TestStatus,
    /// Resumen acotado.
    pub summary: String,
}

/// Forma editable/deserializable del checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffDraft {
    /// Versión del documento.
    pub schema_version: u16,
    /// Objetivo vigente.
    pub objective: String,
    /// Restricciones que sobreviven al relevo.
    pub constraints: Vec<String>,
    /// Decisiones ya tomadas.
    pub decisions: Vec<String>,
    /// Ficheros relativos relevantes.
    pub files: Vec<String>,
    /// Resumen del diff, no el historial.
    pub diff_summary: String,
    /// Pruebas observadas.
    pub tests: Vec<TestFact>,
    /// Categoría del fallo.
    pub failure: FailureCategory,
    /// Mensaje literal acotado.
    pub failure_message: String,
    /// Próximo paso ejecutable.
    pub next_step: String,
    /// Presupuesto de tokens restante.
    pub remaining_tokens: u64,
    /// Presupuesto de pared restante.
    pub remaining_wall_seconds: u64,
}

/// Checkpoint validado que reemplaza el reenvío de historial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HandoffDraft", into = "HandoffDraft")]
pub struct HandoffCheckpoint(HandoffDraft);

impl TryFrom<HandoffDraft> for HandoffCheckpoint {
    type Error = HandoffError;

    fn try_from(draft: HandoffDraft) -> Result<Self, Self::Error> {
        if draft.schema_version != 1 {
            return Err(HandoffError::SchemaVersion {
                received: draft.schema_version,
            });
        }
        for (field, value) in [
            ("objective", draft.objective.as_str()),
            ("failure_message", draft.failure_message.as_str()),
            ("next_step", draft.next_step.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(HandoffError::Empty { field });
            }
        }
        for file in &draft.files {
            let path = Path::new(file);
            if path.is_absolute()
                || path.components().any(|part| {
                    matches!(
                        part,
                        Component::ParentDir | Component::CurDir | Component::RootDir
                    )
                })
            {
                return Err(HandoffError::NonRelativeFile { file: file.clone() });
            }
        }
        Ok(Self(draft))
    }
}

impl From<HandoffCheckpoint> for HandoffDraft {
    fn from(checkpoint: HandoffCheckpoint) -> Self {
        checkpoint.0
    }
}

/// Error de un checkpoint incoherente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffError {
    /// Versión desconocida.
    SchemaVersion {
        /// Versión recibida.
        received: u16,
    },
    /// Campo obligatorio vacío.
    Empty {
        /// Campo rechazado.
        field: &'static str,
    },
    /// Ruta absoluta o con traversal.
    NonRelativeFile {
        /// Ruta rechazada.
        file: String,
    },
}

impl fmt::Display for HandoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion { received } => {
                write!(
                    f,
                    "invalid checkpoint schema_version {received}; supported: 1"
                )
            }
            Self::Empty { field } => write!(f, "checkpoint field '{field}' cannot be empty"),
            Self::NonRelativeFile { file } => {
                write!(f, "checkpoint file '{file}' must be a safe relative path")
            }
        }
    }
}

impl std::error::Error for HandoffError {}
