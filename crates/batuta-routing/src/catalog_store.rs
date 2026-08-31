//! Staging inmutable y activación atómica del catálogo.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::Catalog;
use crate::snapshot_store::atomic_write;

/// Propuesta inmutable ligada a una base activa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogProposal {
    /// Versión del documento.
    pub schema_version: u16,
    /// Identificador seguro proporcionado por la aplicación.
    pub id: String,
    /// Fecha Unix UTC.
    pub created_at: u64,
    /// Hash activo observado al crearla.
    pub expected_active_hash: String,
    /// Catálogo propuesto completo.
    pub catalog: Catalog,
    /// Hash del catálogo propuesto.
    pub proposed_catalog_hash: String,
    /// Sello del cuerpo de la propuesta.
    pub proposal_hash: String,
}

/// Resumen compartido por CLI y TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogStatus {
    /// Hash del catálogo activo.
    pub active_hash: String,
    /// Número de rutas activas.
    pub active_routes: usize,
    /// Propuestas disponibles, ordenadas.
    pub staged: Vec<String>,
}

/// Repositorio local de catálogo activo y propuestas.
#[derive(Debug, Clone)]
pub struct CatalogStore {
    root: PathBuf,
}

impl CatalogStore {
    /// Abre una raíz sin modificarla.
    pub const fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Crea una propuesta; nunca cambia el activo.
    ///
    /// # Errors
    ///
    /// Si el id no es seguro, ya existe o falla la persistencia.
    pub fn stage(
        &self,
        id: &str,
        created_at: u64,
        catalog: Catalog,
    ) -> Result<CatalogProposal, CatalogStoreError> {
        validate_id(id)?;
        let active = self.load_active()?;
        let expected_active_hash = active.hash().map_err(CatalogStoreError::Json)?;
        let proposed_catalog_hash = catalog.hash().map_err(CatalogStoreError::Json)?;
        let proposal_hash = proposal_hash(
            id,
            created_at,
            &expected_active_hash,
            &catalog,
            &proposed_catalog_hash,
        )?;
        let proposal = CatalogProposal {
            schema_version: 2,
            id: id.to_string(),
            created_at,
            expected_active_hash,
            catalog,
            proposed_catalog_hash,
            proposal_hash,
        };
        let path = self.proposals().join(format!("{id}.json"));
        if path.exists() {
            return Err(CatalogStoreError::AlreadyExists(id.to_string()));
        }
        let bytes = document_bytes(&proposal)?;
        atomic_write(&path, &bytes).map_err(CatalogStoreError::Io)?;
        Ok(proposal)
    }

    /// Describe activo y staging sin tomar cerrojos ni mutar.
    ///
    /// # Errors
    ///
    /// Si algún directorio o documento activo no puede leerse.
    pub fn status(&self) -> Result<CatalogStatus, CatalogStoreError> {
        let active = self.load_active()?;
        let mut staged = Vec::new();
        match std::fs::read_dir(self.proposals()) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(CatalogStoreError::Io)?;
                    let path = entry.path();
                    if path.extension().and_then(std::ffi::OsStr::to_str) == Some("json")
                        && let Some(id) = path.file_stem().and_then(std::ffi::OsStr::to_str)
                    {
                        staged.push(id.to_string());
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(CatalogStoreError::Io(error)),
        }
        staged.sort();
        Ok(CatalogStatus {
            active_hash: active.hash().map_err(CatalogStoreError::Json)?,
            active_routes: active.routes().len(),
            staged,
        })
    }

    /// Activa una propuesta confirmada si sello y base siguen vigentes.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, la propuesta cambió, la base cambió o falla E/S.
    pub fn apply(&self, id: &str, confirmed: bool) -> Result<Catalog, CatalogStoreError> {
        if !confirmed {
            return Err(CatalogStoreError::NotConfirmed);
        }
        validate_id(id)?;
        let path = self.proposals().join(format!("{id}.json"));
        let bytes = std::fs::read(path).map_err(CatalogStoreError::Io)?;
        let proposal: CatalogProposal =
            serde_json::from_slice(&bytes).map_err(CatalogStoreError::Json)?;
        if proposal.schema_version != 2 || proposal.id != id {
            return Err(CatalogStoreError::ProposalHashMismatch);
        }
        let expected_hash = proposal_hash(
            &proposal.id,
            proposal.created_at,
            &proposal.expected_active_hash,
            &proposal.catalog,
            &proposal.proposed_catalog_hash,
        )?;
        let catalog_hash = proposal.catalog.hash().map_err(CatalogStoreError::Json)?;
        if expected_hash != proposal.proposal_hash || catalog_hash != proposal.proposed_catalog_hash
        {
            return Err(CatalogStoreError::ProposalHashMismatch);
        }
        let active_hash = self
            .load_active()?
            .hash()
            .map_err(CatalogStoreError::Json)?;
        if active_hash != proposal.expected_active_hash {
            return Err(CatalogStoreError::BaseConflict);
        }
        let document = document_bytes(&proposal.catalog)?;
        atomic_write(&self.active_path(), &document).map_err(CatalogStoreError::Io)?;
        Ok(proposal.catalog)
    }

    /// Carga el catálogo activo; antes de la primera aplicación es vacío.
    ///
    /// # Errors
    ///
    /// Si el documento activo no puede leerse o validarse como JSON.
    pub fn load_active(&self) -> Result<Catalog, CatalogStoreError> {
        match std::fs::read(self.active_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(CatalogStoreError::Json),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Catalog::empty()),
            Err(error) => Err(CatalogStoreError::Io(error)),
        }
    }

    fn active_path(&self) -> PathBuf {
        self.root.join("active.json")
    }

    fn proposals(&self) -> PathBuf {
        self.root.join("proposals")
    }
}

#[derive(Serialize)]
struct ProposalBody<'a> {
    schema_version: u16,
    id: &'a str,
    created_at: u64,
    expected_active_hash: &'a str,
    catalog: &'a Catalog,
    proposed_catalog_hash: &'a str,
}

fn proposal_hash(
    id: &str,
    created_at: u64,
    expected_active_hash: &str,
    catalog: &Catalog,
    proposed_catalog_hash: &str,
) -> Result<String, CatalogStoreError> {
    let body = ProposalBody {
        schema_version: 2,
        id,
        created_at,
        expected_active_hash,
        catalog,
        proposed_catalog_hash,
    };
    let bytes = serde_json::to_vec(&body).map_err(CatalogStoreError::Json)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn document_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CatalogStoreError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(CatalogStoreError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn validate_id(id: &str) -> Result<(), CatalogStoreError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(CatalogStoreError::InvalidId(id.to_string()));
    }
    Ok(())
}

/// Fallo estable de staging o activación.
#[derive(Debug)]
pub enum CatalogStoreError {
    /// Identificador inseguro.
    InvalidId(String),
    /// La propuesta ya existe.
    AlreadyExists(String),
    /// Falta `--confirm`.
    NotConfirmed,
    /// El contenido no coincide con su sello.
    ProposalHashMismatch,
    /// El activo cambió desde la creación.
    BaseConflict,
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
}

impl CatalogStoreError {
    /// Código estable para CLI, MCP y TUI.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidId(_) => "invalid_proposal_id",
            Self::AlreadyExists(_) => "proposal_already_exists",
            Self::NotConfirmed => "proposal_not_confirmed",
            Self::ProposalHashMismatch => "proposal_hash_mismatch",
            Self::BaseConflict => "catalog_base_conflict",
            Self::Io(_) => "catalog_store_io",
            Self::Json(_) => "catalog_store_json",
        }
    }
}

impl fmt::Display for CatalogStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid catalog proposal id '{id}'"),
            Self::AlreadyExists(id) => write!(f, "catalog proposal '{id}' already exists"),
            Self::NotConfirmed => f.write_str("catalog proposal requires explicit confirmation"),
            Self::ProposalHashMismatch => f.write_str("catalog proposal hash mismatch"),
            Self::BaseConflict => f.write_str("active catalog changed since proposal creation"),
            Self::Io(error) => write!(f, "catalog store I/O failed: {error}"),
            Self::Json(error) => write!(f, "catalog store JSON failed: {error}"),
        }
    }
}

impl std::error::Error for CatalogStoreError {}
