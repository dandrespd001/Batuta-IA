//! Estado v2 por objetos inmutables y un único manifest activo.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use batuta_lease::{LeaseSpace, LeaseStore};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::snapshot_store::atomic_write;
use crate::{CapabilityIndexV2, CatalogStateV2, EvidenceStateV2, HealthStateV2, PolicyStateV2};
use crate::{HealthObservationV2, RouteHealth};
use batuta_contract::RouteRef;

const MANIFEST_NAME: &str = "state-manifest-v2.json";

/// Documentos completos que forman una generación de estado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateComponentsV2 {
    /// Catálogo normalizado.
    pub catalog: CatalogStateV2,
    /// Política consolidada.
    pub policy: PolicyStateV2,
    /// Evidencia activa e historial de overrides.
    pub evidence: EvidenceStateV2,
    /// Salud durable.
    pub health: HealthStateV2,
    /// Recibos vigentes de capacidades.
    pub capabilities: CapabilityIndexV2,
}

/// Único puntero mutable del estado v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateManifestV2 {
    /// Versión cerrada del documento.
    pub schema_version: u16,
    /// Generación monótona.
    pub generation: u64,
    /// Objeto de catálogo.
    pub catalog_hash: String,
    /// Objeto de política.
    pub policy_hash: String,
    /// Objeto de evidencia.
    pub evidence_hash: String,
    /// Objeto de salud.
    pub health_hash: String,
    /// Objeto de capacidades.
    pub capabilities_hash: String,
}

impl StateManifestV2 {
    /// Hashes de todos los objetos fijados por la generación.
    pub fn component_hashes(&self) -> [&str; 5] {
        [
            &self.catalog_hash,
            &self.policy_hash,
            &self.evidence_hash,
            &self.health_hash,
            &self.capabilities_hash,
        ]
    }

    /// Hash canónico del propio manifest para identificar cachés derivadas.
    ///
    /// # Errors
    ///
    /// Si el documento no puede serializarse.
    pub fn manifest_hash(&self) -> Result<String, StateStoreError> {
        canonical_bytes(self).map(|bytes| content_hash(&bytes))
    }

    fn validate(&self) -> Result<(), StateStoreError> {
        if self.schema_version != 2 {
            return Err(StateStoreError::Invalid(format!(
                "state manifest schema_version {} is unsupported; supported: 2",
                self.schema_version
            )));
        }
        if self.generation == 0 {
            return Err(StateStoreError::Invalid(
                "state manifest generation must be positive".to_string(),
            ));
        }
        for hash in self.component_hashes() {
            validate_hash(hash)?;
        }
        Ok(())
    }
}

/// Generación consistente cargada desde un único manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateSnapshotV2 {
    /// Manifest leído una sola vez.
    pub manifest: StateManifestV2,
    /// Componentes verificados contra él.
    pub components: StateComponentsV2,
}

/// Almacén local de objetos por contenido.
#[derive(Debug, Clone)]
pub struct StateStore {
    root: PathBuf,
}

impl StateStore {
    /// Abre una ubicación sin crearla todavía.
    pub const fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Publica una generación después de escribir y sincronizar sus objetos.
    ///
    /// # Errors
    ///
    /// Si falla serialización, escritura, sincronización o la generación se agota.
    pub fn commit(
        &self,
        components: &StateComponentsV2,
    ) -> Result<StateManifestV2, StateStoreError> {
        self.commit_if_base(components, None)
    }

    /// Publica sólo si el manifest activo sigue siendo la base esperada.
    ///
    /// Los escritores se serializan mediante un lease interproceso. `None`
    /// omite el CAS, pero no la exclusión.
    ///
    /// # Errors
    ///
    /// Si otro escritor conserva el lease, cambió la base o falla el commit.
    pub fn commit_if_base(
        &self,
        components: &StateComponentsV2,
        expected_manifest_hash: Option<&str>,
    ) -> Result<StateManifestV2, StateStoreError> {
        let leases = LeaseStore::open(&self.root.join("leases"))
            .map_err(|error| StateStoreError::Lease(error.to_string()))?;
        let _guard = leases
            .acquire(
                LeaseSpace::Repository,
                "state-manifest-v2",
                &format!("state-{}", std::process::id()),
            )
            .map_err(|error| StateStoreError::Lease(error.to_string()))?;
        if let Some(expected) = expected_manifest_hash {
            validate_hash(expected)?;
            let current = self.load_manifest()?;
            let actual = current.manifest_hash()?;
            if actual != expected {
                return Err(StateStoreError::Conflict {
                    expected: expected.to_string(),
                    actual,
                });
            }
        }
        let generation = match self.load_manifest() {
            Ok(current) => current
                .generation
                .checked_add(1)
                .ok_or_else(|| StateStoreError::Invalid("generation overflow".to_string()))?,
            Err(StateStoreError::NotFound) => 1,
            Err(error) => return Err(error),
        };
        let objects = self.root.join("objects");
        std::fs::create_dir_all(&objects).map_err(StateStoreError::Io)?;

        let catalog_hash = self.write_object(&components.catalog)?;
        let policy_hash = self.write_object(&components.policy)?;
        let evidence_hash = self.write_object(&components.evidence)?;
        let health_hash = self.write_object(&components.health)?;
        let capabilities_hash = self.write_object(&components.capabilities)?;
        File::open(&objects)
            .and_then(|directory| directory.sync_all())
            .map_err(StateStoreError::Io)?;

        let manifest = StateManifestV2 {
            schema_version: 2,
            generation,
            catalog_hash,
            policy_hash,
            evidence_hash,
            health_hash,
            capabilities_hash,
        };
        let mut bytes = canonical_bytes(&manifest)?;
        bytes.push(b'\n');
        atomic_write(&self.root.join(MANIFEST_NAME), &bytes).map_err(StateStoreError::Io)?;
        Ok(manifest)
    }

    /// Lee y valida únicamente el manifest activo.
    ///
    /// # Errors
    ///
    /// Si no existe, es ilegible o incumple el contrato.
    pub fn load_manifest(&self) -> Result<StateManifestV2, StateStoreError> {
        let path = self.root.join(MANIFEST_NAME);
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(StateStoreError::NotFound);
            }
            Err(error) => return Err(StateStoreError::Io(error)),
        };
        let manifest =
            serde_json::from_slice::<StateManifestV2>(&bytes).map_err(StateStoreError::Json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Lee una generación coherente y verifica todos sus objetos.
    ///
    /// # Errors
    ///
    /// Si el manifest o cualquiera de sus objetos falta, cambia o es JSON inválido.
    pub fn load(&self) -> Result<StateSnapshotV2, StateStoreError> {
        let manifest = self.load_manifest()?;
        let components = StateComponentsV2 {
            catalog: self.read_object(&manifest.catalog_hash)?,
            policy: self.read_object(&manifest.policy_hash)?,
            evidence: self.read_object(&manifest.evidence_hash)?,
            health: self.read_object(&manifest.health_hash)?,
            capabilities: self.read_object(&manifest.capabilities_hash)?,
        };
        Ok(StateSnapshotV2 {
            manifest,
            components,
        })
    }

    /// Añade una muestra de salud mediante CAS, reaplicándola sobre la última
    /// generación cuando otro proceso publica antes.
    ///
    /// # Errors
    ///
    /// Si la ruta no tiene salud explícita o la persistencia no converge.
    pub fn record_health_observation(
        &self,
        route: &RouteRef,
        observation: &HealthObservationV2,
    ) -> Result<StateManifestV2, StateStoreError> {
        let mut last_conflict = None;
        for _ in 0..256 {
            let snapshot = self.load()?;
            let expected = snapshot.manifest.manifest_hash()?;
            let mut components = snapshot.components;
            let health: &mut RouteHealth =
                components.health.routes.get_mut(route).ok_or_else(|| {
                    StateStoreError::Invalid(format!(
                        "health component has no explicit route '{route}'"
                    ))
                })?;
            health.record(observation.clone());
            match self.commit_if_base(&components, Some(&expected)) {
                Ok(manifest) => return Ok(manifest),
                Err(error @ (StateStoreError::Conflict { .. } | StateStoreError::Lease(_))) => {
                    last_conflict = Some(error);
                    std::thread::yield_now();
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_conflict.unwrap_or_else(|| {
            StateStoreError::Invalid("health CAS retry limit exhausted".to_string())
        }))
    }

    /// Ruta local de un hash validado.
    ///
    /// # Errors
    ///
    /// Si no tiene la forma `sha256:<64 hex minúsculos>`.
    pub fn object_path(&self, hash: &str) -> Result<PathBuf, StateStoreError> {
        validate_hash(hash)?;
        Ok(self
            .root
            .join("objects")
            .join(format!("{}.json", &hash[7..])))
    }

    fn write_object<T: Serialize>(&self, value: &T) -> Result<String, StateStoreError> {
        let bytes = canonical_bytes(value)?;
        let hash = content_hash(&bytes);
        let path = self.object_path(&hash)?;
        if path.exists() {
            let existing = std::fs::read(&path).map_err(StateStoreError::Io)?;
            if existing != bytes {
                return Err(StateStoreError::Invalid(format!(
                    "immutable object '{}' contains bytes that do not match its hash",
                    path.display()
                )));
            }
            return Ok(hash);
        }
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)?;
            file.write_all(&bytes)?;
            file.flush()?;
            file.sync_all()
        })();
        if let Err(error) = result {
            let _ = std::fs::remove_file(&path);
            return Err(StateStoreError::Io(error));
        }
        Ok(hash)
    }

    fn read_object<T: DeserializeOwned>(&self, hash: &str) -> Result<T, StateStoreError> {
        let path = self.object_path(hash)?;
        let bytes = std::fs::read(&path).map_err(StateStoreError::Io)?;
        let actual = content_hash(&bytes);
        if actual != hash {
            return Err(StateStoreError::Invalid(format!(
                "object hash mismatch for '{}': expected {hash}, received {actual}",
                path.display()
            )));
        }
        serde_json::from_slice(&bytes).map_err(StateStoreError::Json)
    }
}

/// Fallo del estado transaccional.
#[derive(Debug)]
pub enum StateStoreError {
    /// Todavía no existe manifest activo.
    NotFound,
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
    /// Invariante o hash inválido.
    Invalid(String),
    /// Otro proceso posee el lease de escritura.
    Lease(String),
    /// El manifest base cambió durante la transacción.
    Conflict {
        /// Hash esperado.
        expected: String,
        /// Hash observado.
        actual: String,
    },
}

impl fmt::Display for StateStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "state manifest v2 does not exist"),
            Self::Io(error) => write!(f, "state store I/O failed: {error}"),
            Self::Json(error) => write!(f, "state store JSON failed: {error}"),
            Self::Invalid(message) => write!(f, "invalid state store: {message}"),
            Self::Lease(message) => write!(f, "state writer lease failed: {message}"),
            Self::Conflict { expected, actual } => write!(
                f,
                "state manifest changed: expected {expected}, received {actual}"
            ),
        }
    }
}

impl std::error::Error for StateStoreError {}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, StateStoreError> {
    serde_json::to_vec(value).map_err(StateStoreError::Json)
}

fn content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn validate_hash(hash: &str) -> Result<(), StateStoreError> {
    let valid = hash.len() == 71
        && hash.starts_with("sha256:")
        && hash[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(StateStoreError::Invalid(format!(
            "invalid content hash '{hash}'"
        )))
    }
}
