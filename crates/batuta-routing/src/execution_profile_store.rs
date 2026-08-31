//! Staging y publicación CAS del perfil operativo.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use batuta_exec::ExecutionProfileV1;
use batuta_lease::{LeaseSpace, LeaseStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::snapshot_store::atomic_write;

/// Base explícita usada cuando todavía no hay perfil activo.
pub const EMPTY_EXECUTION_PROFILE_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Propuesta inmutable de perfil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfileProposalV1 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Identificador que el operador debe volver a escribir.
    pub id: String,
    /// Creación Unix UTC.
    pub created_at: u64,
    /// Hash activo observado al crear staging.
    pub expected_active_hash: String,
    /// Perfil sellado propuesto.
    pub proposed_profile: ExecutionProfileV1,
    /// Hash redundante para revisión rápida.
    pub proposed_profile_hash: String,
    /// Diferencia textual determinista.
    pub diff: String,
    /// Sello de la propuesta completa.
    pub proposal_hash: String,
}

#[derive(Serialize)]
struct ProposalBody<'a> {
    schema_version: u16,
    id: &'a str,
    created_at: u64,
    expected_active_hash: &'a str,
    proposed_profile: &'a ExecutionProfileV1,
    proposed_profile_hash: &'a str,
    diff: &'a str,
}

/// Vista compartida por CLI y TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfileStatusV1 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Perfil activo, si ya se confirmó uno.
    pub active: Option<ExecutionProfileV1>,
    /// Hash activo o la base vacía explícita.
    pub active_hash: String,
    /// Propuestas append-only ordenadas por ID.
    pub proposals: Vec<ExecutionProfileProposalV1>,
}

/// Almacén transaccional del perfil operativo.
#[derive(Debug, Clone)]
pub struct ExecutionProfileStore {
    root: PathBuf,
    leases: PathBuf,
}

impl ExecutionProfileStore {
    /// Abre las ubicaciones sin crear documentos.
    pub const fn open(root: PathBuf, leases: PathBuf) -> Self {
        Self { root, leases }
    }

    /// Crea una propuesta sin modificar el perfil activo.
    ///
    /// # Errors
    ///
    /// Si identidad, perfil, serialización o persistencia fallan.
    pub fn stage(
        &self,
        id: &str,
        created_at: u64,
        proposed_profile: ExecutionProfileV1,
    ) -> Result<ExecutionProfileProposalV1, ExecutionProfileStoreError> {
        validate_id(id)?;
        proposed_profile
            .validate()
            .map_err(|error| ExecutionProfileStoreError::Invalid(error.to_string()))?;
        let status = self.status()?;
        let proposed_profile_hash = proposed_profile.profile_hash().to_string();
        let diff = profile_diff(status.active.as_ref(), &proposed_profile)?;
        let mut proposal = ExecutionProfileProposalV1 {
            schema_version: 1,
            id: id.to_string(),
            created_at,
            expected_active_hash: status.active_hash,
            proposed_profile,
            proposed_profile_hash,
            diff,
            proposal_hash: String::new(),
        };
        proposal.proposal_hash = proposal_hash(&proposal)?;
        let staging = self.root.join("staging");
        std::fs::create_dir_all(&staging).map_err(ExecutionProfileStoreError::Io)?;
        let path = staging.join(format!("{id}.json"));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(ExecutionProfileStoreError::Io)?;
        let mut bytes =
            serde_json::to_vec_pretty(&proposal).map_err(ExecutionProfileStoreError::Json)?;
        bytes.push(b'\n');
        file.write_all(&bytes)
            .map_err(ExecutionProfileStoreError::Io)?;
        file.sync_all().map_err(ExecutionProfileStoreError::Io)?;
        File::open(staging)
            .and_then(|directory| directory.sync_all())
            .map_err(ExecutionProfileStoreError::Io)?;
        Ok(proposal)
    }

    /// Carga perfil y propuestas revalidando todos sus sellos.
    ///
    /// # Errors
    ///
    /// Si un documento activo o staged fue alterado.
    pub fn status(&self) -> Result<ExecutionProfileStatusV1, ExecutionProfileStoreError> {
        let active = match std::fs::read(self.active_path()) {
            Ok(bytes) => {
                let profile: ExecutionProfileV1 =
                    serde_json::from_slice(&bytes).map_err(ExecutionProfileStoreError::Json)?;
                profile
                    .validate()
                    .map_err(|error| ExecutionProfileStoreError::Invalid(error.to_string()))?;
                Some(profile)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(ExecutionProfileStoreError::Io(error)),
        };
        let active_hash = active.as_ref().map_or_else(
            || EMPTY_EXECUTION_PROFILE_HASH.to_string(),
            |profile| profile.profile_hash().to_string(),
        );
        let mut proposals = Vec::new();
        match std::fs::read_dir(self.root.join("staging")) {
            Ok(entries) => {
                for entry in entries {
                    let path = entry.map_err(ExecutionProfileStoreError::Io)?.path();
                    if path
                        .extension()
                        .is_some_and(|extension| extension == "json")
                    {
                        let bytes = std::fs::read(path).map_err(ExecutionProfileStoreError::Io)?;
                        let proposal: ExecutionProfileProposalV1 =
                            serde_json::from_slice(&bytes)
                                .map_err(ExecutionProfileStoreError::Json)?;
                        validate_proposal(&proposal)?;
                        proposals.push(proposal);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ExecutionProfileStoreError::Io(error)),
        }
        proposals.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(ExecutionProfileStatusV1 {
            schema_version: 1,
            active,
            active_hash,
            proposals,
        })
    }

    /// Publica una propuesta sólo con base y confirmación explícitas.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, cambió la base, el ID no existe o falla el commit.
    pub fn apply(
        &self,
        id: &str,
        expected_hash: &str,
        confirmed: bool,
    ) -> Result<ExecutionProfileV1, ExecutionProfileStoreError> {
        validate_id(id)?;
        validate_hash(expected_hash)?;
        if !confirmed {
            return Err(ExecutionProfileStoreError::ConfirmationRequired);
        }
        let leases = LeaseStore::open(&self.leases)
            .map_err(|error| ExecutionProfileStoreError::Lease(error.to_string()))?;
        let _guard = leases
            .acquire(LeaseSpace::Repository, "execution-profile", id)
            .map_err(|error| ExecutionProfileStoreError::Lease(error.to_string()))?;
        let path = self.root.join("staging").join(format!("{id}.json"));
        let bytes = std::fs::read(path).map_err(ExecutionProfileStoreError::Io)?;
        let proposal: ExecutionProfileProposalV1 =
            serde_json::from_slice(&bytes).map_err(ExecutionProfileStoreError::Json)?;
        validate_proposal(&proposal)?;
        if proposal.id != id || proposal.expected_active_hash != expected_hash {
            return Err(ExecutionProfileStoreError::Conflict {
                expected: proposal.expected_active_hash,
                actual: expected_hash.to_string(),
            });
        }
        let status = self.status()?;
        if status.active_hash != expected_hash {
            return Err(ExecutionProfileStoreError::Conflict {
                expected: expected_hash.to_string(),
                actual: status.active_hash,
            });
        }
        let mut active_bytes = serde_json::to_vec_pretty(&proposal.proposed_profile)
            .map_err(ExecutionProfileStoreError::Json)?;
        active_bytes.push(b'\n');
        atomic_write(&self.active_path(), &active_bytes).map_err(ExecutionProfileStoreError::Io)?;
        Ok(proposal.proposed_profile)
    }

    fn active_path(&self) -> PathBuf {
        self.root.join("active.json")
    }
}

fn validate_proposal(
    proposal: &ExecutionProfileProposalV1,
) -> Result<(), ExecutionProfileStoreError> {
    if proposal.schema_version != 1 {
        return Err(ExecutionProfileStoreError::Invalid(
            "execution profile proposal schema_version must be 1".to_string(),
        ));
    }
    validate_id(&proposal.id)?;
    validate_hash(&proposal.expected_active_hash)?;
    validate_hash(&proposal.proposed_profile_hash)?;
    proposal
        .proposed_profile
        .validate()
        .map_err(|error| ExecutionProfileStoreError::Invalid(error.to_string()))?;
    if proposal.proposed_profile.profile_hash() != proposal.proposed_profile_hash
        || proposal_hash(proposal)? != proposal.proposal_hash
    {
        return Err(ExecutionProfileStoreError::HashMismatch);
    }
    Ok(())
}

fn proposal_hash(
    proposal: &ExecutionProfileProposalV1,
) -> Result<String, ExecutionProfileStoreError> {
    let body = ProposalBody {
        schema_version: proposal.schema_version,
        id: &proposal.id,
        created_at: proposal.created_at,
        expected_active_hash: &proposal.expected_active_hash,
        proposed_profile: &proposal.proposed_profile,
        proposed_profile_hash: &proposal.proposed_profile_hash,
        diff: &proposal.diff,
    };
    let bytes = serde_json::to_vec(&body).map_err(ExecutionProfileStoreError::Json)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn profile_diff(
    active: Option<&ExecutionProfileV1>,
    proposed: &ExecutionProfileV1,
) -> Result<String, ExecutionProfileStoreError> {
    let before = active
        .map_or_else(|| Ok("null".to_string()), serde_json::to_string_pretty)
        .map_err(ExecutionProfileStoreError::Json)?;
    let after = serde_json::to_string_pretty(proposed).map_err(ExecutionProfileStoreError::Json)?;
    Ok(format!("active:\n{before}\nproposed:\n{after}"))
}

fn validate_id(id: &str) -> Result<(), ExecutionProfileStoreError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(ExecutionProfileStoreError::Invalid(format!(
            "invalid identifier '{id}'"
        )));
    }
    Ok(())
}

fn validate_hash(hash: &str) -> Result<(), ExecutionProfileStoreError> {
    let valid = hash.len() == 71
        && hash.starts_with("sha256:")
        && hash[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(ExecutionProfileStoreError::Invalid(format!(
            "invalid content hash '{hash}'"
        )))
    }
}

/// Fallo del almacén de perfiles.
#[derive(Debug)]
pub enum ExecutionProfileStoreError {
    /// Documento o identidad inválidos.
    Invalid(String),
    /// Sello alterado.
    HashMismatch,
    /// Aplicación sin `--confirm`.
    ConfirmationRequired,
    /// La base activa cambió.
    Conflict {
        /// Base esperada.
        expected: String,
        /// Base observada.
        actual: String,
    },
    /// Exclusión durable.
    Lease(String),
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
}

impl fmt::Display for ExecutionProfileStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Lease(message) => f.write_str(message),
            Self::HashMismatch => f.write_str("execution_profile_proposal_hash_mismatch"),
            Self::ConfirmationRequired => f.write_str("execution_profile_confirmation_required"),
            Self::Conflict { expected, actual } => write!(
                f,
                "execution_profile_conflict: expected {expected}, received {actual}"
            ),
            Self::Io(error) => write!(f, "execution profile store I/O failed: {error}"),
            Self::Json(error) => write!(f, "execution profile store JSON failed: {error}"),
        }
    }
}

impl std::error::Error for ExecutionProfileStoreError {}
