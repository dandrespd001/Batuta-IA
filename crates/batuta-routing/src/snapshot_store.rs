//! Repositorio durable de la foto consumida por el servicio de aplicación.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{RoutingSnapshot, SelectError};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Almacén de un único snapshot activo.
#[derive(Debug, Clone)]
pub struct RoutingSnapshotStore {
    path: PathBuf,
}

impl RoutingSnapshotStore {
    /// Abre el fichero indicado sin crear nada todavía.
    pub const fn open(path: PathBuf) -> Self {
        Self { path }
    }

    /// Lee y vuelve a validar el documento completo.
    ///
    /// # Errors
    ///
    /// Si no puede leerse, deserializarse o incumple invariantes v2.
    pub fn load(&self) -> Result<RoutingSnapshot, SnapshotStoreError> {
        let bytes = std::fs::read(&self.path).map_err(SnapshotStoreError::Io)?;
        let snapshot =
            serde_json::from_slice::<RoutingSnapshot>(&bytes).map_err(SnapshotStoreError::Json)?;
        snapshot.validate().map_err(SnapshotStoreError::Invalid)
    }

    /// Guarda mediante temporal exclusivo, `flush`, `fsync`, rename y `fsync` del directorio.
    ///
    /// # Errors
    ///
    /// Si la serialización o cualquier paso durable falla. El destino anterior
    /// no se abre ni se trunca antes del rename.
    pub fn save(&self, snapshot: &RoutingSnapshot) -> Result<(), SnapshotStoreError> {
        let mut bytes = serde_json::to_vec_pretty(snapshot).map_err(SnapshotStoreError::Json)?;
        bytes.push(b'\n');
        atomic_write(&self.path, &bytes).map_err(SnapshotStoreError::Io)
    }
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("snapshot");
    let temporary = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Fallo tipado del repositorio.
#[derive(Debug)]
pub enum SnapshotStoreError {
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
    /// Invariante de la foto.
    Invalid(SelectError),
}

impl fmt::Display for SnapshotStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "routing snapshot I/O failed: {error}"),
            Self::Json(error) => write!(f, "routing snapshot JSON failed: {error}"),
            Self::Invalid(error) => write!(f, "invalid routing snapshot: {error}"),
        }
    }
}

impl std::error::Error for SnapshotStoreError {}
