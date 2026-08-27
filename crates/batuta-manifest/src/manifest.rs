//! El manifiesto de proveedor: **lo único que crece por proveedor**.
//!
//! La tesis del proyecto es que dar de alta un proveedor sea un fichero y nunca
//! un parche. El síntoma que la paga: `abacus_cli` estaba declarado en
//! `provider_adapters.py:92` del orquestador viejo y no tenía ejecutor; cuatro
//! modelos quedaron inalcanzables, y el que el Arquitecto quería usar estaba
//! detrás de esa pared.
//!
//! Dos tipos a propósito, como en el `TaskSpec`: lo que se deserializa no tiene
//! garantías, y lo que circula sí. Un `ProviderManifest` que existe es un
//! manifiesto coherente.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use batuta_contract::{
    AuthMethod, CanaryExpectation, EnvVarName, ModelId, ParserKind, PromptDelivery,
    ProvenanceSource, ProviderId, ProviderKind, Role, RouteModel, SchemaVersion, Sensitivity,
};

use crate::error::ManifestError;
use crate::runtime_file::RuntimeFile;
use crate::substitution::Substitutions;

/// El ejecutable del proveedor, fijado por versión **y** hash.
///
/// R11 se paga aquí dos veces: el pin decía `abacusai@2.6.9` mientras corría la
/// 2.6.11, y el binario de dsh vive en la caché de `npx` en canal `rc`, donde un
/// `@latest` lo reescribe sin avisar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executable {
    _program: PathBuf,
    _version_pin: String,
    _version_probe: Vec<String>,
    _sha256: Option<String>,
    _resolve: Vec<String>,
}

/// Cómo se autentica el proveedor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    _method: AuthMethod,
    _store_path: Option<PathBuf>,
    _credential: Option<String>,
    _env: Option<EnvVarName>,
}

/// Cómo se invoca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoke {
    _argv: Vec<String>,
    _workdir: String,
    _prompt_via: PromptDelivery,
    _prompt_flag: Option<String>,
}

/// Allowlist de entorno (R5): nada se hereda sin nombrarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvPolicy {
    _allow: Vec<EnvVarName>,
    _deny: Vec<EnvVarName>,
    _set: Vec<(EnvVarName, String)>,
}

/// Un modelo enrutable del proveedor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    _id: ModelId,
    _route_model: RouteModel,
    _route_provider: Option<String>,
    _roles: BTreeSet<Role>,
    _max_sensitivity: Sensitivity,
}

/// El canario, que es **observacional** y nunca un juicio por subcadena.
///
/// R3 se paga aquí: `provider-canary` devolvió `QUOTA_UNAVAILABLE` en 126 ms sin
/// tocar la red, porque leyó el `status` del mismo fichero que él debía informar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canary {
    _prompt: String,
    _expect: CanaryExpectation,
}

/// Manifiesto coherente: existe, luego cumple sus invariantes.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderManifest {
    _origin: PathBuf,
    _schema_version: SchemaVersion,
    _id: ProviderId,
    _kind: ProviderKind,
    _executable: Executable,
    _auth: Auth,
    _invoke: Invoke,
    _env: EnvPolicy,
    _parser: ParserKind,
    _provenance: ProvenanceSource,
    _substitutions: Substitutions,
    _runtime_files: Vec<RuntimeFile>,
    _models: Vec<ModelEntry>,
    _canary: Canary,
}

impl ProviderManifest {
    /// Valida un manifiesto **sin tocar el disco**.
    ///
    /// Aquí van las comprobaciones que no dependen de la máquina: vocabularios,
    /// identificadores, forma de los documentos, llaves de sustitución y
    /// cobertura de los mapas. Es puro a propósito: se prueba entero sin red ni
    /// ficheros, igual que `batuta-policy`.
    ///
    /// `origin` sólo se usa para nombrarlo en los errores.
    ///
    /// # Errors
    ///
    /// Cualquier incoherencia de forma, vocabulario, identificador, llave o
    /// cobertura. El error dice fichero, línea y campo.
    pub fn parse(_source: &str, _origin: &Path) -> Result<Self, ManifestError> {
        todo!("draft por serde y luego los invariantes, en el orden de tests/carga.rs")
    }

    /// Lee el fichero, lo valida y **comprueba que el ejecutable se resuelve**.
    ///
    /// La comprobación del ejecutable es lo que R1 exige: un manifiesto con
    /// ejecutor irresoluble falla al cargar, no tras pagar la corrida.
    ///
    /// # Errors
    ///
    /// Lo de [`ProviderManifest::parse`], más el fichero ilegible y el ejecutable
    /// que no se resuelve o no cuadra con su `sha256`.
    pub fn load(_path: &Path) -> Result<Self, ManifestError> {
        todo!("read_to_string -> parse -> verify_executable")
    }

    /// Todos los manifiestos de un directorio, en orden alfabético por `id`.
    ///
    /// Se relee en cada invocación: la configuración es en caliente (R7), y un
    /// manifiesto nuevo tiene que verse sin reiniciar nada.
    ///
    /// # Errors
    ///
    /// El primero que falle, nombrando su fichero: un directorio con un
    /// manifiesto roto no se carga a medias.
    pub fn load_dir(_dir: &Path) -> Result<Vec<Self>, ManifestError> {
        todo!("*.toml del directorio, cada uno por load()")
    }

    /// Resuelve el programa contra `resolve` y comprueba el hash si lo hay.
    ///
    /// # Errors
    ///
    /// Ninguna ruta de `resolve` existe y es ejecutable, o el `sha256` no cuadra.
    pub fn verify_executable(&self) -> Result<PathBuf, ManifestError> {
        todo!("primera ruta que exista y sea ejecutable; luego sha256 si está fijado")
    }

    /// Identificador del proveedor.
    pub fn id(&self) -> &ProviderId {
        todo!()
    }

    /// Naturaleza del proveedor.
    pub fn kind(&self) -> ProviderKind {
        todo!()
    }

    /// De dónde sale la procedencia del recibo.
    pub fn provenance(&self) -> ProvenanceSource {
        todo!()
    }

    /// Los ficheros que hay que materializar por corrida. Puede estar vacío.
    pub fn runtime_files(&self) -> &[RuntimeFile] {
        todo!()
    }

    /// Los modelos enrutables. Nunca vacío: un manifiesto sin modelos no enruta.
    pub fn models(&self) -> &[ModelEntry] {
        todo!()
    }

    /// Los mapas de sustitución declarados.
    pub fn substitutions(&self) -> &Substitutions {
        todo!()
    }
}

impl ModelEntry {
    /// Identificador dentro de batuta.
    pub fn id(&self) -> &ModelId {
        todo!()
    }

    /// El nombre que el proveedor entiende. **No se valida estáticamente:** el
    /// catálogo vive en el servidor. `ZAI GLM 5.3 Flash` no aparece en ninguna
    /// parte del paquete instalado de abacus, y aun así es el nombre correcto.
    /// La divergencia la detecta el canario.
    pub fn route_model(&self) -> &RouteModel {
        todo!()
    }

    /// La ruta del proveedor, cuando distingue ruta de modelo. En dsh
    /// `deepseek-official` y `deepseek` son dos rutas distintas al mismo sitio.
    pub fn route_provider(&self) -> Option<&str> {
        todo!()
    }

    /// Techo de sensibilidad de este modelo.
    pub fn max_sensitivity(&self) -> Sensitivity {
        todo!()
    }
}
