//! El `TaskSpec`: qué se pide, con qué límites y sobre qué se permite escribir.
//!
//! Hay dos tipos a propósito. [`TaskSpecDraft`] es lo que se rellena o se
//! deserializa: campos públicos, sin garantías. [`TaskSpec`] es lo que circula
//! por batuta: sólo se obtiene pasando por [`TaskSpec::try_from`], así que un
//! `TaskSpec` que existe es un `TaskSpec` coherente. No hay estado intermedio
//! «validado a medias» que alguien pueda olvidarse de comprobar.
//!
//! Ojo con lo que **no** está aquí: ni `repo`, ni `profile`, ni `prompt`. El
//! repositorio lo deriva el perfil y el prompt viaja aparte. `deny_unknown_fields`
//! convierte ese acuerdo en un fallo de carga en vez de un campo ignorado en
//! silencio.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{GateProfileId, RelativePath, SchemaVersion, SchemaVersionError};
use crate::vocabularies::{
    Capability, OutputContract, ReasoningEffort, Role, Sensitivity, WriteMode,
};

fn version_por_defecto() -> SchemaVersion {
    SchemaVersion::CURRENT
}

/// Borrador de `TaskSpec`: lo que se rellena o se deserializa, sin garantías.
///
/// Se convierte en [`TaskSpec`] con `TaskSpec::try_from`, que es donde viven los
/// invariantes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpecDraft {
    /// Versión del esquema del documento.
    #[serde(default = "version_por_defecto")]
    pub schema_version: SchemaVersion,

    /// Rol del trabajo. Se admite `task_type` como nombre heredado.
    #[serde(alias = "task_type")]
    pub role: Role,

    /// Sensibilidad del material que se toca.
    pub sensitivity: Sensitivity,

    /// Forma que debe tener la salida aceptada.
    pub output_contract: OutputContract,

    /// Qué se le permite hacer al árbol de trabajo.
    pub write_mode: WriteMode,

    /// Rutas, relativas al repositorio, donde se admite escribir.
    #[serde(default)]
    pub allowed_write_paths: Vec<RelativePath>,

    /// Capacidades que el trabajo exige y el modelo debe tener demostradas.
    #[serde(default)]
    pub required_capabilities: BTreeSet<Capability>,

    /// Perfil de gates con el que se juzga la entrega.
    pub gate_profile: GateProfileId,

    /// Límite de pared, en segundos.
    pub timeout_seconds: u32,

    /// Reparaciones admitidas tras un gate fallido.
    #[serde(default)]
    pub max_repairs: u8,

    /// Esfuerzo de razonamiento pedido, si se fija.
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,
}

/// Encargo coherente: existe, luego cumple sus invariantes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TaskSpecDraft")]
pub struct TaskSpec {
    schema_version: SchemaVersion,
    role: Role,
    sensitivity: Sensitivity,
    output_contract: OutputContract,
    write_mode: WriteMode,
    allowed_write_paths: Vec<RelativePath>,
    required_capabilities: BTreeSet<Capability>,
    gate_profile: GateProfileId,
    timeout_seconds: u32,
    max_repairs: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,
}

impl TaskSpec {
    /// Mínimo de `timeout_seconds`.
    pub const MIN_TIMEOUT_SECONDS: u32 = 1;

    /// Máximo de `timeout_seconds`: un día.
    pub const MAX_TIMEOUT_SECONDS: u32 = 86_400;

    /// Máximo de `max_repairs`.
    ///
    /// Son dos porque la regla de reencaminamiento del brief §5 dice que dos
    /// fallos seguidos de un modelo externo mandan el trabajo a Codex o a
    /// Claude. Si se pudieran pedir cinco reparaciones, esa regla no se
    /// dispararía nunca y el reencaminado se quedaría en buena intención.
    pub const MAX_REPAIRS: u8 = 2;

    /// Versión del esquema del documento.
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Rol del trabajo.
    pub const fn role(&self) -> Role {
        self.role
    }

    /// Sensibilidad del material.
    pub const fn sensitivity(&self) -> Sensitivity {
        self.sensitivity
    }

    /// Forma exigida a la salida.
    pub const fn output_contract(&self) -> OutputContract {
        self.output_contract
    }

    /// Modo de escritura.
    pub const fn write_mode(&self) -> WriteMode {
        self.write_mode
    }

    /// Rutas donde se admite escribir. Vacío si el modo no escribe.
    pub fn allowed_write_paths(&self) -> &[RelativePath] {
        &self.allowed_write_paths
    }

    /// Capacidades exigidas, ya completadas con las que el modo implica.
    pub const fn required_capabilities(&self) -> &BTreeSet<Capability> {
        &self.required_capabilities
    }

    /// Perfil de gates.
    pub const fn gate_profile(&self) -> &GateProfileId {
        &self.gate_profile
    }

    /// Límite de pared, en segundos.
    pub const fn timeout_seconds(&self) -> u32 {
        self.timeout_seconds
    }

    /// Reparaciones admitidas.
    pub const fn max_repairs(&self) -> u8 {
        self.max_repairs
    }

    /// Esfuerzo de razonamiento pedido.
    pub const fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        self.reasoning_effort
    }

    /// ¿Escribe este encargo en el árbol de trabajo?
    pub const fn writes(&self) -> bool {
        !matches!(self.write_mode, WriteMode::ReadOnly)
    }
}

impl TryFrom<TaskSpecDraft> for TaskSpec {
    type Error = TaskSpecError;

    fn try_from(draft: TaskSpecDraft) -> Result<Self, Self::Error> {
        let schema_version = draft.schema_version.require_supported()?;

        if !(Self::MIN_TIMEOUT_SECONDS..=Self::MAX_TIMEOUT_SECONDS).contains(&draft.timeout_seconds)
        {
            return Err(TaskSpecError::TimeoutOutOfRange {
                seconds: draft.timeout_seconds,
                min: Self::MIN_TIMEOUT_SECONDS,
                max: Self::MAX_TIMEOUT_SECONDS,
            });
        }

        if draft.max_repairs > Self::MAX_REPAIRS {
            return Err(TaskSpecError::TooManyRepairs {
                requested: draft.max_repairs,
                max: Self::MAX_REPAIRS,
            });
        }

        let escribe = !matches!(draft.write_mode, WriteMode::ReadOnly);

        if escribe {
            if draft.allowed_write_paths.is_empty() {
                return Err(TaskSpecError::MissingWritePaths {
                    write_mode: draft.write_mode,
                });
            }
        } else if !draft.allowed_write_paths.is_empty() {
            return Err(TaskSpecError::WritePathsInReadOnly {
                count: draft.allowed_write_paths.len(),
            });
        }

        if matches!(draft.output_contract, OutputContract::UnifiedDiff) && !escribe {
            return Err(TaskSpecError::DiffWithoutWriteMode {
                write_mode: draft.write_mode,
            });
        }

        if !escribe && draft.required_capabilities.contains(&Capability::Write) {
            return Err(TaskSpecError::WriteCapabilityInReadOnly);
        }

        if !matches!(draft.role, Role::Research)
            && draft
                .required_capabilities
                .contains(&Capability::WebResearch)
        {
            return Err(TaskSpecError::WebResearchWithoutResearchRole { role: draft.role });
        }

        comprobar_allowlist(&draft.allowed_write_paths)?;

        // Capacidades implícitas: lo que el encargo necesita por su forma no
        // depende de que alguien se acuerde de escribirlo. Cada sitio donde
        // hubiera que repetirlo es un sitio donde podría divergir.
        let mut required_capabilities = draft.required_capabilities;
        if escribe {
            required_capabilities.insert(Capability::Write);
        }
        if matches!(draft.role, Role::Research) {
            required_capabilities.insert(Capability::WebResearch);
        }

        Ok(Self {
            schema_version,
            role: draft.role,
            sensitivity: draft.sensitivity,
            output_contract: draft.output_contract,
            write_mode: draft.write_mode,
            allowed_write_paths: draft.allowed_write_paths,
            required_capabilities,
            gate_profile: draft.gate_profile,
            timeout_seconds: draft.timeout_seconds,
            max_repairs: draft.max_repairs,
            reasoning_effort: draft.reasoning_effort,
        })
    }
}

/// ¿Está `interna` dentro de `externa`, o son la misma?
fn cubre(externa: &str, interna: &str) -> bool {
    interna
        .strip_prefix(externa)
        .is_some_and(|resto| resto.is_empty() || resto.starts_with('/'))
}

/// Una allowlist se lee entera antes de aprobar un encargo: si una ruta tapa a
/// otra, lo que se aprueba deja de ser evidente.
fn comprobar_allowlist(rutas: &[RelativePath]) -> Result<(), TaskSpecError> {
    for (indice, primera) in rutas.iter().enumerate() {
        for segunda in &rutas[indice + 1..] {
            if primera == segunda {
                return Err(TaskSpecError::DuplicateWritePath {
                    path: primera.as_str().to_string(),
                });
            }
            let (externa, interna) = if cubre(primera.as_str(), segunda.as_str()) {
                (primera, segunda)
            } else if cubre(segunda.as_str(), primera.as_str()) {
                (segunda, primera)
            } else {
                continue;
            };
            return Err(TaskSpecError::NestedWritePath {
                outer: externa.as_str().to_string(),
                inner: interna.as_str().to_string(),
            });
        }
    }
    Ok(())
}

/// Encargo incoherente.
///
/// Cada variante nombra el campo que falla y dice qué se admitía, igual que los
/// vocabularios de R8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpecError {
    /// Versión de esquema que batuta no sabe leer.
    SchemaVersion(SchemaVersionError),
    /// `timeout_seconds` fuera de rango.
    TimeoutOutOfRange {
        /// Valor recibido.
        seconds: u32,
        /// Mínimo admitido.
        min: u32,
        /// Máximo admitido.
        max: u32,
    },
    /// Más reparaciones de las que deja la regla de reencaminamiento.
    TooManyRepairs {
        /// Valor recibido.
        requested: u8,
        /// Máximo admitido.
        max: u8,
    },
    /// Un encargo que no escribe trae rutas de escritura.
    WritePathsInReadOnly {
        /// Cuántas rutas traía.
        count: usize,
    },
    /// Un encargo que escribe no dice dónde.
    MissingWritePaths {
        /// El modo que sí escribe.
        write_mode: WriteMode,
    },
    /// Se pide un diff a un encargo que no puede escribir.
    DiffWithoutWriteMode {
        /// El modo que no escribe.
        write_mode: WriteMode,
    },
    /// Un encargo que no escribe exige la capacidad `write`.
    WriteCapabilityInReadOnly,
    /// Un encargo que no investiga exige la capacidad `web_research`.
    WebResearchWithoutResearchRole {
        /// El rol que no puede exigirla.
        role: Role,
    },
    /// La misma ruta dos veces en la allowlist.
    DuplicateWritePath {
        /// La ruta repetida.
        path: String,
    },
    /// Una ruta de la allowlist ya estaba cubierta por otra.
    NestedWritePath {
        /// La ruta que cubre.
        outer: String,
        /// La ruta cubierta, y por tanto redundante.
        inner: String,
    },
}

impl fmt::Display for TaskSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion(error) => write!(f, "{error}"),
            Self::TimeoutOutOfRange { seconds, min, max } => write!(
                f,
                "timeout_seconds out of range: {seconds}. valid range: {min}..={max}"
            ),
            Self::TooManyRepairs { requested, max } => write!(
                f,
                "max_repairs is {requested}, at most {max} is allowed: two consecutive failures \
                 must reroute the work, not repair it again"
            ),
            Self::WritePathsInReadOnly { count } => write!(
                f,
                "write_mode is read_only but allowed_write_paths has {count} entries: a task that \
                 does not write must not name anything writable"
            ),
            Self::MissingWritePaths { write_mode } => write!(
                f,
                "write_mode is {write_mode} but allowed_write_paths is empty: containment is by \
                 explicit name, and an empty allowlist names nothing"
            ),
            Self::DiffWithoutWriteMode { write_mode } => write!(
                f,
                "output_contract unified_diff needs a write_mode that writes, got {write_mode}. \
                 valid values: validated_patch, validated_apply"
            ),
            Self::WriteCapabilityInReadOnly => {
                f.write_str("write_mode is read_only but required_capabilities contains write")
            }
            Self::WebResearchWithoutResearchRole { role } => write!(
                f,
                "required_capabilities contains web_research but role is {role}: only role \
                 research may demand it, and research gets it implicitly"
            ),
            Self::DuplicateWritePath { path } => {
                write!(f, "duplicate allowed_write_paths entry: '{path}'")
            }
            Self::NestedWritePath { outer, inner } => write!(
                f,
                "allowed_write_paths entry '{inner}' is already covered by '{outer}'"
            ),
        }
    }
}

impl core::error::Error for TaskSpecError {}

impl From<SchemaVersionError> for TaskSpecError {
    fn from(error: SchemaVersionError) -> Self {
        Self::SchemaVersion(error)
    }
}
