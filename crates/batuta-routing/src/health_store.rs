use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

use crate::RouteHealth;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HealthDocument {
    schema_version: u16,
    routes: BTreeMap<RouteRef, RouteHealth>,
}

/// Fichero durable de salud por ruta, escrito atómicamente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStore {
    path: PathBuf,
}

impl HealthStore {
    /// Abre un fichero; no lo crea hasta la primera actualización.
    pub fn open(path: PathBuf) -> Self {
        Self { path }
    }

    /// Carga toda la foto. Un fichero ausente es el estado inicial vacío.
    ///
    /// # Errors
    ///
    /// Si el fichero no se puede leer, parsear o validar.
    pub fn load(&self) -> Result<BTreeMap<RouteRef, RouteHealth>, HealthStoreError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(error) => return Err(HealthStoreError::Io(io_message(&self.path, error))),
        };
        let document: HealthDocument = serde_json::from_str(&text)
            .map_err(|error| HealthStoreError::Json(error.to_string()))?;
        if document.schema_version != 1 {
            return Err(HealthStoreError::SchemaVersion {
                received: document.schema_version,
            });
        }
        for (route, health) in &document.routes {
            if !health.recent_success_rate.is_finite()
                || !(0.0..=1.0).contains(&health.recent_success_rate)
            {
                return Err(HealthStoreError::InvalidHealth {
                    route: route.clone(),
                });
            }
        }
        Ok(document.routes)
    }

    /// Reemplaza la salud de una ruta conservando las demás.
    ///
    /// # Errors
    ///
    /// Si falla la carga previa o la escritura atómica.
    pub fn update(&self, route: RouteRef, health: RouteHealth) -> Result<(), HealthStoreError> {
        let mut routes = self.load()?;
        routes.insert(route, health);
        self.save(&routes)
    }

    /// Guarda una foto completa mediante temporal sincronizado y renombrado.
    ///
    /// # Errors
    ///
    /// Si la foto no se puede serializar o persistir.
    pub fn save(&self, routes: &BTreeMap<RouteRef, RouteHealth>) -> Result<(), HealthStoreError> {
        let document = HealthDocument {
            schema_version: 1,
            routes: routes.clone(),
        };
        let mut bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| HealthStoreError::Json(error.to_string()))?;
        bytes.push(b'\n');
        atomic_write(&self.path, &bytes)
    }
}

/// Fallo al leer o guardar salud durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStoreError {
    /// Fallo del sistema de ficheros.
    Io(String),
    /// JSON inválido.
    Json(String),
    /// Versión desconocida.
    SchemaVersion {
        /// Versión recibida.
        received: u16,
    },
    /// Tasa de éxito inválida para una ruta.
    InvalidHealth {
        /// Ruta afectada.
        route: RouteRef,
    },
}

impl fmt::Display for HealthStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) | Self::Json(message) => f.write_str(message),
            Self::SchemaVersion { received } => {
                write!(f, "invalid health schema_version {received}; supported: 1")
            }
            Self::InvalidHealth { route } => write!(f, "invalid health for route '{route}'"),
        }
    }
}

impl std::error::Error for HealthStoreError {}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), HealthStoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| HealthStoreError::Io(format!("{} has no parent", path.display())))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| HealthStoreError::Io(io_message(parent, error)))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".health.{}.{}.tmp", std::process::id(), sequence));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| HealthStoreError::Io(io_message(&temporary, error)))?;
        file.write_all(bytes)
            .map_err(|error| HealthStoreError::Io(io_message(&temporary, error)))?;
        file.sync_all()
            .map_err(|error| HealthStoreError::Io(io_message(&temporary, error)))?;
        std::fs::rename(&temporary, path)
            .map_err(|error| HealthStoreError::Io(io_message(path, error)))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| HealthStoreError::Io(io_message(parent, error)))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[allow(clippy::needless_pass_by_value)]
fn io_message(path: &Path, error: std::io::Error) -> String {
    format!("{}: {error}", path.display())
}
