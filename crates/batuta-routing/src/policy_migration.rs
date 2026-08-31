//! Migración v1 → v2 ensayable, confirmada y recuperable.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use sha2::{Digest as _, Sha256};

use crate::snapshot_store::atomic_write;
use crate::{MigrationSettings, PolicyError, RoutingPolicy};

/// Resultado idempotente de aplicar una migración.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMigrationOutcome {
    /// Se creó backup y se activó v2.
    Applied,
    /// El contenido v2 previsto ya estaba activo.
    AlreadyApplied,
}

/// Plan inmutable calculado sin escribir.
#[derive(Debug, Clone)]
pub struct PolicyMigration {
    path: PathBuf,
    backup: PathBuf,
    source: Vec<u8>,
    source_hash: String,
    migrated: Vec<u8>,
    migrated_hash: String,
    diff: String,
}

impl PolicyMigration {
    /// Lee v1 y calcula v2 y su diff, sin modificar ningún fichero.
    ///
    /// # Errors
    ///
    /// Si no puede leerse v1, no valida o no se serializa como v2.
    pub fn plan(path: PathBuf, settings: MigrationSettings) -> Result<Self, PolicyMigrationError> {
        let source = std::fs::read(&path).map_err(PolicyMigrationError::Io)?;
        let text = std::str::from_utf8(&source).map_err(PolicyMigrationError::Utf8)?;
        let policy =
            RoutingPolicy::migrate_v1(text, settings).map_err(PolicyMigrationError::Policy)?;
        let migrated_text = policy.to_toml().map_err(PolicyMigrationError::Policy)?;
        let migrated = migrated_text.into_bytes();
        let source_hash = hash(&source);
        let migrated_hash = hash(&migrated);
        let diff = format!(
            "schema_version: 1 -> 2\nlegacy_models: {}\nsource: {source_hash}\nresult: {migrated_hash}",
            policy.legacy_models().len()
        );
        let backup = path.with_extension("toml.v1.bak");
        Ok(Self {
            path,
            backup,
            source,
            source_hash,
            migrated,
            migrated_hash,
            diff,
        })
    }

    /// Diff determinista del dry-run.
    pub fn diff(&self) -> &str {
        &self.diff
    }

    /// Aplica sólo con confirmación y si la base no cambió.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, cambió la base, el backup discrepa o falla E/S.
    pub fn apply(&self, confirmed: bool) -> Result<PolicyMigrationOutcome, PolicyMigrationError> {
        let current = std::fs::read(&self.path).map_err(PolicyMigrationError::Io)?;
        let current_hash = hash(&current);
        if current_hash == self.migrated_hash {
            return Ok(PolicyMigrationOutcome::AlreadyApplied);
        }
        if !confirmed {
            return Err(PolicyMigrationError::NotConfirmed);
        }
        if current_hash != self.source_hash {
            return Err(PolicyMigrationError::BaseConflict);
        }
        self.ensure_backup()?;
        atomic_write(&self.path, &self.migrated).map_err(PolicyMigrationError::Io)?;
        Ok(PolicyMigrationOutcome::Applied)
    }

    fn ensure_backup(&self) -> Result<(), PolicyMigrationError> {
        match std::fs::read(&self.backup) {
            Ok(existing) if existing == self.source => return Ok(()),
            Ok(_) => return Err(PolicyMigrationError::BackupConflict),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(PolicyMigrationError::Io(error));
            }
            Err(_) => {}
        }
        let parent = self
            .backup
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        std::fs::create_dir_all(parent).map_err(PolicyMigrationError::Io)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.backup)
            .map_err(PolicyMigrationError::Io)?;
        file.write_all(&self.source)
            .map_err(PolicyMigrationError::Io)?;
        file.flush().map_err(PolicyMigrationError::Io)?;
        file.sync_all().map_err(PolicyMigrationError::Io)?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(PolicyMigrationError::Io)
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Fallo de planificación o aplicación.
#[derive(Debug)]
pub enum PolicyMigrationError {
    /// Falta confirmación.
    NotConfirmed,
    /// El activo cambió tras el dry-run.
    BaseConflict,
    /// Ya existe un backup con otros bytes.
    BackupConflict,
    /// E/S local.
    Io(std::io::Error),
    /// V1 no era UTF-8.
    Utf8(std::str::Utf8Error),
    /// Política inválida.
    Policy(PolicyError),
}

impl fmt::Display for PolicyMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfirmed => f.write_str("policy migration requires explicit confirmation"),
            Self::BaseConflict => f.write_str("policy changed since migration dry-run"),
            Self::BackupConflict => f.write_str("policy v1 backup exists with different bytes"),
            Self::Io(error) => write!(f, "policy migration I/O failed: {error}"),
            Self::Utf8(error) => write!(f, "policy v1 is not UTF-8: {error}"),
            Self::Policy(error) => write!(f, "policy migration failed: {error}"),
        }
    }
}

impl std::error::Error for PolicyMigrationError {}
