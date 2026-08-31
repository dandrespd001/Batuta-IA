use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::{ActiveEvidence, ProposalError, ResearchProposal};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Foto resumida del almacén de investigación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchStatus {
    /// Hash de la evidencia activa.
    pub active_hash: String,
    /// Número de observaciones activas.
    pub active_observations: usize,
    /// Identificadores que esperan confirmación.
    pub staged: Vec<String>,
}

/// Persistencia de evidencia activa y propuestas inmutables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchStore {
    root: PathBuf,
}

impl ResearchStore {
    /// Abre el almacén sin crear ni modificar nada.
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Carga y vuelve a validar la foto activa; si no existe, devuelve una vacía.
    ///
    /// # Errors
    ///
    /// Si el fichero no se puede leer, parsear o validar.
    pub fn load_active(&self) -> Result<ActiveEvidence, ProposalError> {
        let path = self.root.join("active.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return ActiveEvidence::new(Vec::new()).map_err(ProposalError::Quality);
            }
            Err(error) => return Err(io_error(&path, error)),
        };
        let active: ActiveEvidence =
            serde_json::from_str(&text).map_err(|error| ProposalError::Json {
                path: path.display().to_string(),
                message: error.to_string(),
            })?;
        active.revalidate().map_err(ProposalError::Quality)
    }

    /// Guarda una foto completa mediante temporal, `sync_all` y renombrado.
    ///
    /// # Errors
    ///
    /// Si la foto es inválida o la escritura atómica falla.
    pub fn save_active(&self, active: &ActiveEvidence) -> Result<(), ProposalError> {
        let active = active.revalidate().map_err(ProposalError::Quality)?;
        let bytes = serde_json::to_vec_pretty(&active).map_err(|error| ProposalError::Json {
            path: self.root.join("active.json").display().to_string(),
            message: error.to_string(),
        })?;
        atomic_write(&self.root.join("active.json"), &bytes)
    }

    /// Escribe una propuesta sellada en staging; no toca `active.json`.
    ///
    /// # Errors
    ///
    /// Si el id no es seguro o el documento no se puede guardar.
    pub fn stage(&self, proposal: &ResearchProposal) -> Result<(), ProposalError> {
        validate_id(&proposal.id)?;
        std::fs::create_dir_all(self.root.join("staging"))
            .map_err(|error| io_error(&self.root.join("staging"), error))?;
        let path = self
            .root
            .join("staging")
            .join(format!("{}.json", proposal.id));
        let bytes = serde_json::to_vec_pretty(proposal).map_err(|error| ProposalError::Json {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        atomic_write(&path, &bytes)
    }

    /// Estado activo y propuestas pendientes, ordenadas por identificador.
    ///
    /// # Errors
    ///
    /// Si no se puede leer el activo o listar staging.
    pub fn status(&self) -> Result<ResearchStatus, ProposalError> {
        let active = self.load_active()?;
        let staging = self.root.join("staging");
        let mut staged = Vec::new();
        match std::fs::read_dir(&staging) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(|error| io_error(&staging, error))?;
                    let path = entry.path();
                    if path.extension().and_then(std::ffi::OsStr::to_str) == Some("json")
                        && let Some(stem) = path.file_stem().and_then(std::ffi::OsStr::to_str)
                    {
                        staged.push(stem.to_string());
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&staging, error)),
        }
        staged.sort();
        Ok(ResearchStatus {
            active_hash: active.evidence_hash().to_string(),
            active_observations: active.observations().len(),
            staged,
        })
    }

    /// Activa una propuesta confirmada y la archiva de forma recuperable.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, la propuesta no existe o no valida, cambió la
    /// evidencia activa, o falla la persistencia.
    pub fn apply(
        &self,
        proposal_id: &str,
        confirmed: bool,
    ) -> Result<ActiveEvidence, ProposalError> {
        if !confirmed {
            return Err(ProposalError::NotConfirmed);
        }
        validate_id(proposal_id)?;
        let path = self
            .root
            .join("staging")
            .join(format!("{proposal_id}.json"));
        let text = std::fs::read_to_string(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ProposalError::NotFound {
                    id: proposal_id.to_string(),
                }
            } else {
                io_error(&path, error)
            }
        })?;
        let proposal: ResearchProposal =
            serde_json::from_str(&text).map_err(|error| ProposalError::Json {
                path: path.display().to_string(),
                message: error.to_string(),
            })?;
        let active = self.load_active()?;
        let applied = active.apply(&proposal, true)?;
        self.save_active(&applied)?;

        let archive = self.root.join("applied");
        std::fs::create_dir_all(&archive).map_err(|error| io_error(&archive, error))?;
        let archived_path = archive.join(format!("{proposal_id}.json"));
        std::fs::rename(&path, &archived_path).map_err(|error| io_error(&path, error))?;
        sync_directory(&archive)?;
        Ok(applied)
    }
}

fn validate_id(value: &str) -> Result<(), ProposalError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(ProposalError::Quality(crate::QualityError::InvalidField {
            field: "proposal_id",
            message: format!("'{value}' is not a safe identifier"),
        }));
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ProposalError> {
    let parent = path.parent().ok_or_else(|| ProposalError::Io {
        path: path.display().to_string(),
        message: "path has no parent directory".to_string(),
    })?;
    std::fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("state");
    let temporary = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| io_error(&temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error(&temporary, error))?;
        file.write_all(b"\n")
            .map_err(|error| io_error(&temporary, error))?;
        file.sync_all()
            .map_err(|error| io_error(&temporary, error))?;
        std::fs::rename(&temporary, path).map_err(|error| io_error(path, error))?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn sync_directory(path: &Path) -> Result<(), ProposalError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error(path, error))
}

#[allow(clippy::needless_pass_by_value)]
fn io_error(path: &Path, error: std::io::Error) -> ProposalError {
    ProposalError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}
