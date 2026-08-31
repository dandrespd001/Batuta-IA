//! Persistencia atómica de la máquina de ejecución.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::RunState;
use crate::snapshot_store::atomic_write;

/// Repositorio de una ejecución durable.
#[derive(Debug, Clone)]
pub struct RunStateStore {
    path: PathBuf,
}

impl RunStateStore {
    /// Abre el fichero sin modificarlo.
    pub const fn open(path: PathBuf) -> Self {
        Self { path }
    }

    /// Reemplaza el documento completo de forma atómica.
    ///
    /// # Errors
    ///
    /// Si el id no es seguro o falla serialización o E/S.
    pub fn save(&self, id: &str, state: &RunState) -> Result<(), RunStoreError> {
        validate_id(id)?;
        let document = RunDocument {
            schema_version: 2,
            id: id.to_string(),
            state: state.clone(),
        };
        let mut bytes = serde_json::to_vec_pretty(&document).map_err(RunStoreError::Json)?;
        bytes.push(b'\n');
        atomic_write(&self.path, &bytes).map_err(RunStoreError::Io)
    }

    /// Recupera la máquina y comprueba id y versión.
    ///
    /// # Errors
    ///
    /// Si el fichero falta, fue alterado o no es v2.
    pub fn load(&self, id: &str) -> Result<RunState, RunStoreError> {
        validate_id(id)?;
        let bytes = std::fs::read(&self.path).map_err(RunStoreError::Io)?;
        let document: RunDocument = serde_json::from_slice(&bytes).map_err(RunStoreError::Json)?;
        if document.schema_version != 2 || document.id != id {
            return Err(RunStoreError::IdentityMismatch);
        }
        Ok(document.state)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunDocument {
    schema_version: u16,
    id: String,
    state: RunState,
}

fn validate_id(id: &str) -> Result<(), RunStoreError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RunStoreError::InvalidId(id.to_string()));
    }
    Ok(())
}

/// Fallo de recuperación durable.
#[derive(Debug)]
pub enum RunStoreError {
    /// Identificador inseguro.
    InvalidId(String),
    /// El documento pertenece a otra ejecución o versión.
    IdentityMismatch,
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
}

impl fmt::Display for RunStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid run id '{id}'"),
            Self::IdentityMismatch => f.write_str("run state identity or schema mismatch"),
            Self::Io(error) => write!(f, "run state I/O failed: {error}"),
            Self::Json(error) => write!(f, "run state JSON failed: {error}"),
        }
    }
}

impl std::error::Error for RunStoreError {}
