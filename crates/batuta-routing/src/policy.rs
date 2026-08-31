use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use batuta_contract::{ReasoningEffort, RouteRef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Límites explícitos de recuperación usados por el coordinador K4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicyV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Invocaciones máximas, incluida la primera.
    pub max_attempts: u32,
    /// Mayor `Retry-After` que puede aceptarse.
    pub max_retry_after_ms: u64,
    /// Número máximo de cambios de ruta.
    pub max_handoffs: u32,
}

impl ExecutionPolicyV2 {
    /// Construye una política validada sin rellenar ningún valor implícito.
    ///
    /// # Errors
    ///
    /// Si no hay al menos un intento o el techo de espera es cero.
    pub fn new(
        max_attempts: u32,
        max_retry_after_ms: u64,
        max_handoffs: u32,
    ) -> Result<Self, PolicyError> {
        let policy = Self {
            schema_version: 2,
            max_attempts,
            max_retry_after_ms,
            max_handoffs,
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Revalida un documento deserializado.
    ///
    /// # Errors
    ///
    /// Si la versión o los límites no pertenecen al contrato.
    pub fn validate(self) -> Result<Self, PolicyError> {
        if self.schema_version != 2 {
            return Err(PolicyError::InvalidSetting {
                field: "execution.schema_version",
                value: self.schema_version.to_string(),
            });
        }
        if self.max_attempts == 0 || self.max_retry_after_ms == 0 {
            return Err(PolicyError::InvalidSetting {
                field: "execution.attempts_or_retry_after",
                value: format!(
                    "attempts={}, retry_after_ms={}",
                    self.max_attempts, self.max_retry_after_ms
                ),
            });
        }
        Ok(self)
    }
}

/// Alias humanos resueltos sin heurísticas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasCatalog(BTreeMap<String, RouteRef>);

impl AliasCatalog {
    /// Valida nombres y crea el catálogo.
    ///
    /// # Errors
    ///
    /// Si un alias está vacío o contiene caracteres inseguros.
    pub fn new(aliases: BTreeMap<String, RouteRef>) -> Result<Self, AliasError> {
        for alias in aliases.keys() {
            if alias.is_empty()
                || alias.len() > 128
                || !alias
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err(AliasError::InvalidAlias {
                    alias: alias.clone(),
                });
            }
        }
        Ok(Self(aliases))
    }

    /// Resuelve primero un alias exacto y después una `RouteRef` literal.
    ///
    /// # Errors
    ///
    /// Si el texto no es un alias conocido ni una ruta válida.
    pub fn resolve(&self, value: &str) -> Result<RouteRef, AliasError> {
        if let Some(route) = self.0.get(value) {
            return Ok(route.clone());
        }
        RouteRef::from_str(value).map_err(|_| AliasError::Unknown {
            value: value.to_string(),
            aliases: self.0.keys().cloned().collect(),
        })
    }
}

/// Error al validar o resolver alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasError {
    /// Nombre fuera del vocabulario de alias.
    InvalidAlias {
        /// Alias rechazado.
        alias: String,
    },
    /// Ni alias conocido ni ruta válida.
    Unknown {
        /// Texto pedido.
        value: String,
        /// Alias disponibles.
        aliases: Vec<String>,
    },
}

impl fmt::Display for AliasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAlias { alias } => write!(f, "invalid route alias '{alias}'"),
            Self::Unknown { value, aliases } => {
                write!(f, "unknown route or alias '{value}'; aliases: {aliases:?}")
            }
        }
    }
}

impl std::error::Error for AliasError {}

/// Valores nuevos que una migración v1 no puede inventar.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationSettings {
    /// Umbral de calidad.
    pub minimum_quality: f64,
    /// Margen respecto de `Qmax`.
    pub selection_margin: f64,
    /// Cobertura mínima.
    pub minimum_coverage: u8,
    /// Antigüedad máxima de evidencia.
    pub max_evidence_age_seconds: u64,
    /// Fallbacks no listados.
    pub allow_any_eligible: bool,
    /// Calidad no verificada.
    pub allow_unverified_quality: bool,
    /// Invocaciones máximas, incluida la primera.
    pub max_attempts: u32,
    /// Mayor espera explícita aceptable.
    pub max_retry_after_ms: u64,
    /// Relevos máximos; cero los deshabilita.
    pub max_handoffs: u32,
}

impl MigrationSettings {
    fn validate(self) -> Result<Self, PolicyError> {
        for (field, value) in [
            ("minimum_quality", self.minimum_quality),
            ("selection_margin", self.selection_margin),
        ] {
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(PolicyError::InvalidSetting {
                    field,
                    value: value.to_string(),
                });
            }
        }
        if self.minimum_coverage > 100 || self.max_evidence_age_seconds == 0 {
            return Err(PolicyError::InvalidSetting {
                field: "coverage_or_age",
                value: format!(
                    "coverage={}, age={}",
                    self.minimum_coverage, self.max_evidence_age_seconds
                ),
            });
        }
        ExecutionPolicyV2::new(
            self.max_attempts,
            self.max_retry_after_ms,
            self.max_handoffs,
        )?;
        Ok(self)
    }
}

/// Elección heredada de un modelo durante la migración.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyModelPolicy {
    /// Conserva `habilitado`.
    pub enabled: bool,
    /// Conserva `esfuerzo`.
    pub effort: Option<ReasoningEffort>,
}

/// Política de routing versionada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicy {
    schema_version: u16,
    settings: MigrationSettings,
    legacy_models: BTreeMap<String, LegacyModelPolicy>,
    aliases: BTreeMap<String, RouteRef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDocument {
    schema_version: u16,
    modelos: BTreeMap<String, LegacyChoice>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyChoice {
    habilitado: bool,
    esfuerzo: Option<ReasoningEffort>,
}

impl RoutingPolicy {
    /// Carga únicamente v2; una v1 exige llamar a [`Self::migrate_v1`].
    ///
    /// # Errors
    ///
    /// Si el TOML, la versión, ajustes o alias no validan.
    pub fn from_toml(text: &str) -> Result<Self, PolicyError> {
        let value: toml::Value = toml::from_str(text).map_err(PolicyError::Parse)?;
        let version = value
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            .ok_or(PolicyError::MissingSchemaVersion)?;
        if version == 1 {
            return Err(PolicyError::MigrationRequired);
        }
        if version != 2 {
            return Err(PolicyError::SchemaVersion { received: version });
        }
        let policy: Self = toml::from_str(text).map_err(PolicyError::Parse)?;
        policy.settings.validate()?;
        AliasCatalog::new(policy.aliases.clone()).map_err(PolicyError::Alias)?;
        Ok(policy)
    }

    /// Migra v1 sólo con todos los valores nuevos explícitos.
    ///
    /// # Errors
    ///
    /// Si el documento no es v1 válido o los ajustes nuevos no validan.
    pub fn migrate_v1(text: &str, settings: MigrationSettings) -> Result<Self, PolicyError> {
        let settings = settings.validate()?;
        let legacy: LegacyDocument = toml::from_str(text).map_err(PolicyError::Parse)?;
        if legacy.schema_version != 1 {
            return Err(PolicyError::SchemaVersion {
                received: i64::from(legacy.schema_version),
            });
        }
        let legacy_models = legacy
            .modelos
            .into_iter()
            .map(|(id, choice)| {
                (
                    id,
                    LegacyModelPolicy {
                        enabled: choice.habilitado,
                        effort: choice.esfuerzo,
                    },
                )
            })
            .collect();
        Ok(Self {
            schema_version: 2,
            settings,
            legacy_models,
            aliases: BTreeMap::new(),
        })
    }

    /// Serializa una política v2 revisable.
    ///
    /// # Errors
    ///
    /// Si el serializador TOML rechaza la foto.
    pub fn to_toml(&self) -> Result<String, PolicyError> {
        toml::to_string_pretty(self).map_err(PolicyError::Serialize)
    }

    /// Versión validada.
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Valores globales explícitos.
    pub const fn settings(&self) -> &MigrationSettings {
        &self.settings
    }

    /// Política de ejecución derivada sólo de campos explícitos de esta foto.
    /// Devuelve la política operativa explícita de K4.
    ///
    /// # Errors
    ///
    /// Si alguno de los tres límites persistidos es incoherente.
    pub fn execution_policy(&self) -> Result<ExecutionPolicyV2, PolicyError> {
        ExecutionPolicyV2::new(
            self.settings.max_attempts,
            self.settings.max_retry_after_ms,
            self.settings.max_handoffs,
        )
    }

    /// Modelos conservados desde v1.
    pub const fn legacy_models(&self) -> &BTreeMap<String, LegacyModelPolicy> {
        &self.legacy_models
    }

    /// Hash estable de la política usada en un recibo.
    ///
    /// # Errors
    ///
    /// Si la política no se puede serializar para sellarla.
    pub fn policy_hash(&self) -> Result<String, PolicyError> {
        let bytes = serde_json::to_vec(self).map_err(PolicyError::Json)?;
        let digest = Sha256::digest(bytes);
        Ok(format!("sha256:{digest:x}"))
    }
}

/// Error de carga o migración de política.
#[derive(Debug)]
pub enum PolicyError {
    /// TOML inválido.
    Parse(toml::de::Error),
    /// No declaró versión.
    MissingSchemaVersion,
    /// V1 necesita migración explícita.
    MigrationRequired,
    /// Otra versión desconocida.
    SchemaVersion {
        /// Versión recibida.
        received: i64,
    },
    /// Valor nuevo inválido.
    InvalidSetting {
        /// Campo.
        field: &'static str,
        /// Valor.
        value: String,
    },
    /// Alias inválido.
    Alias(AliasError),
    /// No se pudo serializar TOML.
    Serialize(toml::ser::Error),
    /// No se pudo serializar JSON canónico.
    Json(serde_json::Error),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "invalid routing policy TOML: {error}"),
            Self::MissingSchemaVersion => f.write_str("routing policy is missing schema_version"),
            Self::MigrationRequired => {
                f.write_str("routing policy v1 requires explicit migrate_v1 settings")
            }
            Self::SchemaVersion { received } => {
                write!(
                    f,
                    "unsupported routing policy schema_version {received}; supported: 2"
                )
            }
            Self::InvalidSetting { field, value } => {
                write!(f, "invalid routing policy setting {field}={value}")
            }
            Self::Alias(error) => write!(f, "{error}"),
            Self::Serialize(error) => write!(f, "cannot serialize routing policy: {error}"),
            Self::Json(error) => write!(f, "cannot hash routing policy: {error}"),
        }
    }
}

impl std::error::Error for PolicyError {}
