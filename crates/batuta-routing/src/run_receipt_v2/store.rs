//! Persistencia append-only del recibo sellado.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::PathBuf;

use super::validation::validate_id;
use super::{RunReceiptError, RunReceiptV2};

/// Almacén append-only de recibos finales.
#[derive(Debug, Clone)]
pub struct RunReceiptStoreV2 {
    root: PathBuf,
}

impl RunReceiptStoreV2 {
    /// Abre un directorio fijado por el layout.
    pub const fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Añade un recibo; nunca sobrescribe un ID existente.
    pub fn append(&self, receipt: &RunReceiptV2) -> Result<(), RunReceiptError> {
        receipt.validate()?;
        std::fs::create_dir_all(&self.root).map_err(RunReceiptError::Io)?;
        let path = self.path_for(&receipt.id)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == ErrorKind::AlreadyExists {
                    RunReceiptError::AlreadyExists(receipt.id.clone())
                } else {
                    RunReceiptError::Io(error)
                }
            })?;
        let mut bytes = serde_json::to_vec(receipt).map_err(RunReceiptError::Json)?;
        bytes.push(b'\n');
        file.write_all(&bytes).map_err(RunReceiptError::Io)?;
        file.sync_all().map_err(RunReceiptError::Io)?;
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(RunReceiptError::Io)
    }

    /// Carga y revalida un recibo histórico.
    pub fn load(&self, id: &str) -> Result<RunReceiptV2, RunReceiptError> {
        let path = self.path_for(id)?;
        let bytes = std::fs::read(path).map_err(RunReceiptError::Io)?;
        let receipt: RunReceiptV2 =
            serde_json::from_slice(&bytes).map_err(RunReceiptError::Json)?;
        receipt.validate()?;
        Ok(receipt)
    }

    fn path_for(&self, id: &str) -> Result<PathBuf, RunReceiptError> {
        validate_id(id)?;
        Ok(self.root.join(format!("{id}.json")))
    }
}
