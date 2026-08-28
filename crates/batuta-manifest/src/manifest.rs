// generado: deepseek-v4-flash - revisado: Arquitecto
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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use batuta_contract::{
    AuthMethod, CanaryExpectation, DocumentFormat, EnvVarName, ModelId, ParserKind, PromptDelivery,
    ProvenanceSource, ProviderId, ProviderKind, ReasoningEffort, Role, RouteModel, SchemaVersion,
    Sensitivity, WriteMode,
};
use serde::Deserialize;
use toml::Spanned;

use crate::error::{ManifestError, SourceLocation};
use crate::runtime_file::{RuntimeDocument, RuntimeFile, extract_placeholders};
use crate::substitution::Substitutions;

/// El ejecutable del proveedor, fijado por versión **y** hash.
///
/// R11 se paga aquí dos veces: el pin decía `abacusai@2.6.9` mientras corría la
/// 2.6.11, y el binario de dsh vive en la caché de `npx` en canal `rc`, donde un
/// `@latest` lo reescribe sin avisar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executable {
    program: PathBuf,
    version_pin: String,
    version_probe: Vec<String>,
    sha256: Option<String>,
    resolve: Vec<String>,
    at: SourceLocation,
}

/// Cómo se autentica el proveedor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    method: AuthMethod,
    store_path: Option<PathBuf>,
    credential: Option<String>,
    env: Option<EnvVarName>,
}

/// Cómo se invoca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoke {
    argv: Vec<String>,
    workdir: String,
    prompt_via: PromptDelivery,
    prompt_flag: Option<String>,
}

/// Allowlist de entorno (R5): nada se hereda sin nombrarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvPolicy {
    allow: Vec<EnvVarName>,
    deny: Vec<EnvVarName>,
    set: Vec<(EnvVarName, String)>,
}

/// Un modelo enrutable del proveedor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    id: ModelId,
    route_model: RouteModel,
    observed_as: Option<String>,
    route_provider: Option<String>,
    roles: BTreeSet<Role>,
    max_sensitivity: Sensitivity,
}

/// El canario, que es **observacional** y nunca un juicio por subcadena.
///
/// R3 se paga aquí: `provider-canary` devolvió `QUOTA_UNAVAILABLE` en 126 ms sin
/// tocar la red, porque leyó el `status` del mismo fichero que él debía informar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canary {
    prompt: String,
    expect: CanaryExpectation,
}

/// Manifiesto coherente: existe, luego cumple sus invariantes.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderManifest {
    origin: PathBuf,
    source_sha256: String,
    schema_version: SchemaVersion,
    id: ProviderId,
    kind: ProviderKind,
    executable: Executable,
    auth: Auth,
    invoke: Invoke,
    env: EnvPolicy,
    parser: ParserKind,
    provenance: ProvenanceSource,
    provenance_pattern: Option<String>,
    substitutions: Substitutions,
    runtime_files: Vec<RuntimeFile>,
    models: Vec<ModelEntry>,
    canary: Canary,
}

/// Borrador sin garantías: lo que sale de serde. Cada campo que la validación
/// tiene que nombrar con fichero y línea llega con su `Spanned` para saber
/// dónde estaba en el TOML original.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    schema_version: Spanned<u16>,
    id: Spanned<String>,
    kind: Spanned<String>,
    executable: ExecutableDraft,
    auth: AuthDraft,
    invoke: InvokeDraft,
    env: EnvDraft,
    response: ResponseDraft,
    provenance: ProvenanceDraft,
    #[serde(default)]
    substitutions: BTreeMap<String, Spanned<BTreeMap<String, String>>>,
    #[serde(default)]
    runtime_files: Vec<RuntimeFileDraft>,
    #[serde(default)]
    models: Vec<ModelDraft>,
    canary: CanaryDraft,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutableDraft {
    program: Spanned<String>,
    version_pin: String,
    version_probe: Vec<String>,
    #[serde(default)]
    sha256: Option<String>,
    resolve: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthDraft {
    method: Spanned<String>,
    #[serde(default)]
    store_path: Option<String>,
    #[serde(default)]
    credential: Option<String>,
    #[serde(default)]
    env: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvokeDraft {
    argv: Vec<Spanned<String>>,
    workdir: String,
    prompt: PromptDraft,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptDraft {
    via: Spanned<String>,
    #[serde(default)]
    flag: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvDraft {
    #[serde(default)]
    allow: Vec<Spanned<String>>,
    #[serde(default)]
    deny: Vec<Spanned<String>>,
    #[serde(default)]
    set: Option<Spanned<BTreeMap<String, String>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseDraft {
    parser: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvenanceDraft {
    source: Spanned<String>,
    #[serde(default)]
    pattern: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeFileDraft {
    path: Spanned<String>,
    format: Spanned<String>,
    #[serde(default)]
    entry: Option<Vec<toml::Value>>,
    #[serde(default)]
    content: Option<toml::Table>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelDraft {
    id: Spanned<String>,
    route_model: Spanned<String>,
    #[serde(default)]
    observed_as: Option<Spanned<String>>,
    #[serde(default)]
    route_provider: Option<Spanned<String>>,
    roles: Vec<Spanned<String>>,
    max_sensitivity: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CanaryDraft {
    prompt: String,
    expect: Spanned<String>,
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
    pub fn parse(source: &str, origin: &Path) -> Result<Self, ManifestError> {
        let draft = deserializar(source, origin)?;

        let Draft {
            schema_version,
            id,
            kind,
            executable,
            auth,
            invoke,
            env,
            response,
            provenance,
            substitutions,
            runtime_files,
            models,
            canary,
        } = draft;

        // Versión de esquema: batuta lee las suyas y nada más (R1).
        let schema_span = schema_version.span();
        let schema_version = SchemaVersion::new(*schema_version.get_ref());
        let schema_version = schema_version.require_supported().map_err(|error| {
            ManifestError::UnsupportedSchemaVersion {
                at: location_at(source, schema_span.clone(), origin),
                source: error,
            }
        })?;

        // Un manifiesto sin modelos no enruta a ninguna parte.
        if models.is_empty() {
            return Err(ManifestError::NoModels {
                at: location_at(source, schema_span, origin),
            });
        }

        let id = identifier::<ProviderId>(source, origin, &id, "id")?;
        let kind = vocabulary::<ProviderKind>(source, origin, &kind, "kind")?;
        let executable = validar_ejecutable(source, origin, executable);
        let auth = validar_auth(source, origin, auth)?;

        let InvokeDraft {
            argv,
            workdir,
            prompt,
        } = invoke;
        let prompt_via =
            vocabulary::<PromptDelivery>(source, origin, &prompt.via, "invoke.prompt.via")?;

        // Declarar por dónde entra el prompt y no emitirlo deja el encargo en el
        // camino: el proveedor contestaría a una llamada sin tarea. R1 dice que
        // un manifiesto irresoluble falla **al cargar**, no a mitad de la corrida.
        if prompt_via == PromptDelivery::Argv
            && !argv
                .iter()
                .any(|spanned| spanned.get_ref().contains("{prompt}"))
        {
            let at = argv.first().map_or_else(
                || location_at(source, 0..0, origin),
                |spanned| location_at(source, spanned.span(), origin),
            );
            return Err(ManifestError::PromptNeverDelivered { at });
        }

        let invoke = Invoke {
            argv: argv
                .iter()
                .map(|spanned| spanned.get_ref().clone())
                .collect(),
            workdir,
            prompt_via,
            prompt_flag: prompt.flag,
        };

        let env = validar_env(source, origin, env)?;

        let parser = vocabulary::<ParserKind>(source, origin, &response.parser, "response.parser")?;
        let provenance_pattern = validar_patron_procedencia(source, origin, &provenance)?;
        let provenance = vocabulary::<ProvenanceSource>(
            source,
            origin,
            &provenance.source,
            "provenance.source",
        )?;
        let canary = validar_canary(source, origin, canary)?;

        let substitutions = validar_sustituciones(source, origin, &substitutions)?;
        let models_typed = validar_models(source, origin, &models)?;
        let runtime_files_typed = validar_runtime_files(source, origin, &runtime_files)?;

        comprobar_placeholders(
            source,
            origin,
            &argv,
            &runtime_files,
            &runtime_files_typed,
            &substitutions,
        )?;

        Ok(ProviderManifest {
            origin: origin.to_path_buf(),
            source_sha256: sha256_texto(source),
            schema_version,
            id,
            kind,
            executable,
            auth,
            invoke,
            env,
            parser,
            provenance,
            provenance_pattern,
            substitutions,
            runtime_files: runtime_files_typed,
            models: models_typed,
            canary,
        })
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
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let source = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            file: path.to_path_buf(),
            source,
        })?;
        let manifest = Self::parse(&source, path)?;
        manifest.verify_executable()?;
        Ok(manifest)
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
    pub fn load_dir(dir: &Path) -> Result<Vec<Self>, ManifestError> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|source| ManifestError::Read {
                file: dir.to_path_buf(),
                source,
            })?
            .filter_map(Result::ok)
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);

        let mut manifests = Vec::new();
        for entry in entries {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                manifests.push(Self::load(&path)?);
            }
        }
        manifests.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
        Ok(manifests)
    }

    /// Resuelve el programa contra `resolve` y comprueba el hash si lo hay.
    ///
    /// # Errors
    ///
    /// Ninguna ruta de `resolve` existe y es ejecutable, o el `sha256` no cuadra.
    pub fn verify_executable(&self) -> Result<PathBuf, ManifestError> {
        let mut tried = Vec::new();
        for entry in &self.executable.resolve {
            for candidate in expand_resolve(entry, &self.executable.program) {
                tried.push(candidate.clone());
                if candidate.is_file() && is_executable(&candidate) {
                    if let Some(expected) = &self.executable.sha256 {
                        let found =
                            sha256_hex(&candidate).map_err(|source| ManifestError::Read {
                                file: candidate.clone(),
                                source,
                            })?;
                        if !found.eq_ignore_ascii_case(expected) {
                            return Err(ManifestError::DigestMismatch {
                                at: self.executable.at.clone(),
                                expected: expected.clone(),
                                found,
                            });
                        }
                    }
                    return Ok(candidate);
                }
            }
        }
        Err(ManifestError::ExecutableNotFound {
            at: self.executable.at.clone(),
            program: self.executable.program.clone(),
            tried,
        })
    }

    /// De dónde se cargó.
    ///
    /// El recibo lo lleva: un recibo que no nombra su manifiesto no se puede
    /// reproducir.
    pub fn origin(&self) -> &Path {
        &self.origin
    }

    /// El `sha256` del **texto** del manifiesto.
    ///
    /// No es el del binario del proveedor —ése lo lleva [`Executable::sha256`]—
    /// sino el de las reglas que gobernaron la corrida. Sin él, editar un
    /// manifiesto invalida en silencio todos los recibos anteriores sin que
    /// ninguno se entere.
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    /// Identificador del proveedor.
    pub fn id(&self) -> &ProviderId {
        &self.id
    }

    /// Naturaleza del proveedor.
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// El patrón con que se lee el modelo del `stderr`, si la procedencia es
    /// `stderr_pattern`.
    ///
    /// Lleva `{model}` entre dos literales, y lo que quede en medio es lo que la
    /// máquina anotó. Se **declara** y no se deriva: un normalizador que pasara
    /// `Gemini 3.7 Flash` a `GEMINI_3_7_FLASH` habría acertado en siete de nueve
    /// modelos de abacus y habría tapado los dos únicos interesantes, que
    /// resolvieron a `..._THINKING`.
    pub fn provenance_pattern(&self) -> Option<&str> {
        self.provenance_pattern.as_deref()
    }

    /// De dónde sale la procedencia del recibo.
    pub fn provenance(&self) -> ProvenanceSource {
        self.provenance
    }

    /// Los ficheros que hay que materializar por corrida. Puede estar vacío.
    pub fn runtime_files(&self) -> &[RuntimeFile] {
        &self.runtime_files
    }

    /// Los modelos enrutables. Nunca vacío: un manifiesto sin modelos no enruta.
    pub fn models(&self) -> &[ModelEntry] {
        &self.models
    }

    /// Los mapas de sustitución declarados.
    pub fn substitutions(&self) -> &Substitutions {
        &self.substitutions
    }
    /// El ejecutable y su pin.
    pub fn executable(&self) -> &Executable {
        &self.executable
    }

    /// Cómo se autentica el proveedor.
    pub fn auth(&self) -> &Auth {
        &self.auth
    }

    /// Cómo se invoca: `argv`, dónde trabaja y por dónde entra el prompt.
    pub fn invoke(&self) -> &Invoke {
        &self.invoke
    }

    /// La allowlist de entorno (R5).
    pub fn env(&self) -> &EnvPolicy {
        &self.env
    }

    /// Cómo se extrae el artefacto del flujo crudo (R14).
    pub fn parser(&self) -> ParserKind {
        self.parser
    }

    /// El canario del proveedor.
    pub fn canary(&self) -> &Canary {
        &self.canary
    }
}

impl ModelEntry {
    /// Identificador dentro de batuta.
    pub fn id(&self) -> &ModelId {
        &self.id
    }

    /// El nombre con el que **la máquina lo anota**, si no es el que se pidió.
    ///
    /// Tercer nombre del mismo modelo, y los tres son distintos a propósito:
    /// `id` es el de batuta, `route_model` el que se le manda al proveedor, y
    /// éste el que aparece en su registro. Medido: `Qwen3.8 Max` se manda así y
    /// se anota `QWEN3_8_MAX_THINKING`.
    pub fn observed_as(&self) -> Option<&str> {
        self.observed_as.as_deref()
    }

    /// El nombre que el proveedor entiende. **No se valida estáticamente:** el
    /// catálogo vive en el servidor. `ZAI GLM 5.3 Flash` no aparece en ninguna
    /// parte del paquete instalado de abacus, y aun así es el nombre correcto.
    /// La divergencia la detecta el canario.
    pub fn route_model(&self) -> &RouteModel {
        &self.route_model
    }

    /// La ruta del proveedor, cuando distingue ruta de modelo. En dsh
    /// `deepseek-official` y `deepseek` son dos rutas distintas al mismo sitio.
    pub fn route_provider(&self) -> Option<&str> {
        self.route_provider.as_deref()
    }

    /// Techo de sensibilidad de este modelo.
    pub fn max_sensitivity(&self) -> Sensitivity {
        self.max_sensitivity
    }
}

/// La línea (empezando en 1) en la que empieza el `span`, contando saltos de
/// línea en el documento original.
fn location_at(source: &str, span: std::ops::Range<usize>, file: &Path) -> SourceLocation {
    let start = span.start.min(source.len());
    let line = u32::try_from(
        source[..start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count(),
    )
    .expect("la línea de un manifiesto cabe en u32")
        + 1;
    SourceLocation {
        file: file.to_path_buf(),
        line,
    }
}

/// Deserializa el TOML; un error de sintaxis lleva el sitio que el analizador
/// conoce, y si no lo conoce, la primera línea del fichero.
fn deserializar(source: &str, origin: &Path) -> Result<Draft, ManifestError> {
    toml::from_str(source).map_err(|error| ManifestError::Syntax {
        at: error.span().map_or_else(
            || SourceLocation {
                file: origin.to_path_buf(),
                line: 1,
            },
            |span| location_at(source, span, origin),
        ),
        message: error.message().to_string(),
    })
}

/// El ejecutable: programa, pin, sonda, hash y rutas de resolución. El pin y la
/// sonda son metadatos; la resolución y el hash se comprueban en `load`.
fn validar_ejecutable(source: &str, origin: &Path, executable: ExecutableDraft) -> Executable {
    let ExecutableDraft {
        program,
        version_pin,
        version_probe,
        sha256,
        resolve,
    } = executable;
    Executable {
        program: PathBuf::from(program.get_ref().clone()),
        version_pin,
        version_probe,
        sha256,
        resolve,
        at: location_at(source, program.span(), origin),
    }
}

/// Valida un valor contra un vocabulario cerrado, con fichero, línea y campo.
fn vocabulary<T>(
    source: &str,
    origin: &Path,
    spanned: &Spanned<String>,
    field: &str,
) -> Result<T, ManifestError>
where
    T: std::str::FromStr<Err = batuta_contract::VocabularyError>,
{
    spanned
        .get_ref()
        .parse()
        .map_err(|error| ManifestError::Vocabulary {
            at: location_at(source, spanned.span(), origin),
            field: field.to_string(),
            source: error,
        })
}

/// Valida un identificador, con fichero, línea y campo.
fn identifier<T>(
    source: &str,
    origin: &Path,
    spanned: &Spanned<String>,
    field: &str,
) -> Result<T, ManifestError>
where
    T: std::str::FromStr<Err = batuta_contract::IdentifierError>,
{
    spanned
        .get_ref()
        .parse()
        .map_err(|error| ManifestError::Identifier {
            at: location_at(source, spanned.span(), origin),
            field: field.to_string(),
            source: error,
        })
}

/// Autenticación: método del vocabulario y, si llega, nombre de variable de
/// entorno para la credencial.
fn validar_auth(source: &str, origin: &Path, auth: AuthDraft) -> Result<Auth, ManifestError> {
    let AuthDraft {
        method,
        store_path,
        credential,
        env: auth_env,
    } = auth;
    Ok(Auth {
        method: vocabulary::<AuthMethod>(source, origin, &method, "auth.method")?,
        store_path: store_path.map(PathBuf::from),
        credential,
        env: auth_env
            .as_ref()
            .map(|spanned| identifier::<EnvVarName>(source, origin, spanned, "auth.env"))
            .transpose()?,
    })
}

/// Entorno (R5): allowlist, excepciones y valores fijados, todo identificadores
/// de variable validados.
fn validar_env(source: &str, origin: &Path, env: EnvDraft) -> Result<EnvPolicy, ManifestError> {
    let allow = env
        .allow
        .iter()
        .map(|spanned| identifier::<EnvVarName>(source, origin, spanned, "env.allow"))
        .collect::<Result<Vec<_>, _>>()?;
    let deny = env
        .deny
        .iter()
        .map(|spanned| identifier::<EnvVarName>(source, origin, spanned, "env.deny"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut set = Vec::new();
    if let Some(spanned) = env.set {
        for (key, value) in spanned.get_ref() {
            let name: EnvVarName = key.parse().map_err(|error| ManifestError::Identifier {
                at: location_at(source, spanned.span(), origin),
                field: format!("env.set.{key}"),
                source: error,
            })?;
            if deny.contains(&name) {
                return Err(ManifestError::ConflictingEnvVar {
                    at: location_at(source, spanned.span(), origin),
                    name: name.as_str().to_string(),
                });
            }
            set.push((name, value.clone()));
        }
    }
    Ok(EnvPolicy { allow, deny, set })
}

/// Canario: expectativa del vocabulario cerrado.
fn validar_canary(
    source: &str,
    origin: &Path,
    canary: CanaryDraft,
) -> Result<Canary, ManifestError> {
    let CanaryDraft { prompt, expect } = canary;
    Ok(Canary {
        prompt,
        expect: vocabulary::<CanaryExpectation>(source, origin, &expect, "canary.expect")?,
    })
}

/// Sustituciones: cada llave tiene que cubrir el vocabulario entero. Añadir un
/// `write_mode` sin contemplarlo en todos los mapas es error de carga, nunca un
/// valor por defecto elegido en silencio.
///
/// `reasoning_effort` es un nombre reservado: no deriva de `write_mode` como el
/// resto, así que se separa antes del bucle genérico y se valida contra su
/// propio vocabulario (T1 de `docs/FASE5_PANEL.md`).
fn validar_sustituciones(
    source: &str,
    origin: &Path,
    substitutions: &BTreeMap<String, Spanned<BTreeMap<String, String>>>,
) -> Result<Substitutions, ManifestError> {
    let mut map = BTreeMap::new();
    let mut reasoning_effort = None;
    for (key, spanned) in substitutions {
        if key == "reasoning_effort" {
            reasoning_effort = Some(validar_esfuerzo(source, origin, key, spanned)?);
            continue;
        }
        let mut inner = BTreeMap::new();
        for (mode_token, value) in spanned.get_ref() {
            let write_mode: WriteMode =
                mode_token
                    .parse()
                    .map_err(|error| ManifestError::Vocabulary {
                        at: location_at(source, spanned.span(), origin),
                        field: format!("substitutions.{key}.{mode_token}"),
                        source: error,
                    })?;
            inner.insert(write_mode, value.clone());
        }
        let missing: Vec<&'static str> = WriteMode::ALL
            .iter()
            .filter(|mode| !inner.contains_key(mode))
            .map(|mode| mode.as_str())
            .collect();
        if !missing.is_empty() {
            return Err(ManifestError::SubstitutionIncomplete {
                at: location_at(source, spanned.span(), origin),
                key: key.clone(),
                vocabulary: WriteMode::NAME,
                missing,
            });
        }
        map.insert(key.clone(), inner);
    }
    Ok(Substitutions::new(map, reasoning_effort))
}

/// `[substitutions.reasoning_effort]`: igual de exigente que el resto —cubre
/// `ReasoningEffort::ALL` entero o no carga— pero keyed por `ReasoningEffort` en
/// vez de `WriteMode`, porque dsh y abacus no toman el esfuerzo por el mismo
/// canal que el modo de escritura.
fn validar_esfuerzo(
    source: &str,
    origin: &Path,
    key: &str,
    spanned: &Spanned<BTreeMap<String, String>>,
) -> Result<BTreeMap<ReasoningEffort, String>, ManifestError> {
    let mut inner = BTreeMap::new();
    for (token, value) in spanned.get_ref() {
        let effort: ReasoningEffort = token.parse().map_err(|error| ManifestError::Vocabulary {
            at: location_at(source, spanned.span(), origin),
            field: format!("substitutions.{key}.{token}"),
            source: error,
        })?;
        inner.insert(effort, value.clone());
    }
    let missing: Vec<&'static str> = ReasoningEffort::ALL
        .iter()
        .filter(|effort| !inner.contains_key(effort))
        .map(|effort| effort.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(ManifestError::SubstitutionIncomplete {
            at: location_at(source, spanned.span(), origin),
            key: key.to_string(),
            vocabulary: ReasoningEffort::NAME,
            missing,
        });
    }
    Ok(inner)
}

/// Modelos: identificador y ruta válidos, sin duplicados, roles y techo de
/// sensibilidad del vocabulario.
fn validar_models(
    source: &str,
    origin: &Path,
    models: &[ModelDraft],
) -> Result<Vec<ModelEntry>, ManifestError> {
    let mut seen_ids = BTreeSet::new();
    let mut typed = Vec::with_capacity(models.len());
    for (indice, model) in models.iter().enumerate() {
        let model_id =
            identifier::<ModelId>(source, origin, &model.id, &format!("models[{indice}].id"))?;
        if !seen_ids.insert(model_id.to_string()) {
            return Err(ManifestError::DuplicateModel {
                at: location_at(source, model.id.span(), origin),
                id: model_id.to_string(),
            });
        }
        let route_model = identifier::<RouteModel>(
            source,
            origin,
            &model.route_model,
            &format!("models[{indice}].route_model"),
        )?;
        let route_provider = model
            .route_provider
            .as_ref()
            .map(|spanned| spanned.get_ref().clone());
        let mut roles = BTreeSet::new();
        for role in &model.roles {
            roles.insert(vocabulary::<Role>(
                source,
                origin,
                role,
                &format!("models[{indice}].roles"),
            )?);
        }
        let max_sensitivity = vocabulary::<Sensitivity>(
            source,
            origin,
            &model.max_sensitivity,
            &format!("models[{indice}].max_sensitivity"),
        )?;
        typed.push(ModelEntry {
            id: model_id,
            route_model,
            observed_as: model
                .observed_as
                .as_ref()
                .map(|spanned| spanned.get_ref().clone()),
            route_provider,
            roles,
            max_sensitivity,
        });
    }
    Ok(typed)
}

/// El patrón de procedencia existe **si y sólo si** el origen lo usa.
///
/// Con `stderr_pattern` es obligatorio: sin él no hay nada que leer. Con
/// cualquier otro origen sobra, y un campo que no hace nada acaba creyéndose.
/// Es la misma clase de fallo que un `argv` que dice entregar el prompt y no lo
/// emite, así que se resuelve igual: **falla al cargar** (R1).
fn validar_patron_procedencia(
    source: &str,
    origin: &Path,
    provenance: &ProvenanceDraft,
) -> Result<Option<String>, ManifestError> {
    let por_stderr = provenance.source.get_ref() == "stderr_pattern";
    let at = || location_at(source, provenance.source.span(), origin);

    match (por_stderr, provenance.pattern.as_ref()) {
        (true, None) => Err(ManifestError::ProvenancePattern {
            at: at(),
            problem: "falta, y `stderr_pattern` no tiene nada que leer sin él",
        }),
        (false, Some(_)) => Err(ManifestError::ProvenancePattern {
            at: at(),
            problem: "sobra: sólo `stderr_pattern` lo usa",
        }),
        (true, Some(patron)) if !patron.get_ref().contains("{model}") => {
            Err(ManifestError::ProvenancePattern {
                at: location_at(source, patron.span(), origin),
                problem: "no lleva `{model}`, así que no señala nada",
            })
        }
        (true, Some(patron)) => Ok(Some(patron.get_ref().clone())),
        (false, None) => Ok(None),
    }
}

/// Ficheros de corrida: forma decidida (lista o mapa), ruta contenida y formato
/// del vocabulario.
fn validar_runtime_files(
    source: &str,
    origin: &Path,
    runtime_files: &[RuntimeFileDraft],
) -> Result<Vec<RuntimeFile>, ManifestError> {
    let mut typed = Vec::with_capacity(runtime_files.len());
    for (indice, rf) in runtime_files.iter().enumerate() {
        let path = PathBuf::from(rf.path.get_ref().clone());
        let at = location_at(source, rf.path.span(), origin);

        let document = match (&rf.entry, &rf.content) {
            (Some(_), Some(_)) => {
                return Err(ManifestError::DocumentShapeAmbiguous { at, path });
            }
            (None, None) => {
                return Err(ManifestError::DocumentShapeMissing { at, path });
            }
            (Some(entries), None) => RuntimeDocument::List(entries.clone()),
            (None, Some(table)) => RuntimeDocument::Map(table.clone()),
        };

        if path.is_absolute() {
            return Err(ManifestError::RuntimeFilePathAbsolute { at, path });
        }
        if path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(ManifestError::RuntimeFilePathEscapes { at, path });
        }

        let format = vocabulary::<DocumentFormat>(
            source,
            origin,
            &rf.format,
            &format!("runtime_files[{indice}].format"),
        )?;
        typed.push(RuntimeFile::new(path, format, document));
    }
    Ok(typed)
}

/// Llaves `{...}`: sólo las incorporadas y las declaradas por `[substitutions]`.
/// Se buscan en `argv` y en los documentos de corrida enteros, no sólo en su
/// primer nivel.
fn comprobar_placeholders(
    source: &str,
    origin: &Path,
    argv: &[Spanned<String>],
    runtime_drafts: &[RuntimeFileDraft],
    runtime_files: &[RuntimeFile],
    substitutions: &Substitutions,
) -> Result<(), ManifestError> {
    let allowed = substitutions.allowed_placeholders();
    for arg in argv {
        for placeholder in extract_placeholders(arg.get_ref()) {
            if !allowed.contains(&placeholder) {
                return Err(ManifestError::UnknownPlaceholder {
                    at: location_at(source, arg.span(), origin),
                    field: "invoke.argv".to_string(),
                    placeholder,
                    expected: allowed.clone(),
                });
            }
        }
    }
    for (indice, runtime_file) in runtime_files.iter().enumerate() {
        let field = match runtime_file.document() {
            RuntimeDocument::List(_) => format!("runtime_files[{indice}].entry"),
            RuntimeDocument::Map(_) => format!("runtime_files[{indice}].content"),
        };
        for placeholder in runtime_file.placeholders() {
            if !allowed.contains(&placeholder) {
                return Err(ManifestError::UnknownPlaceholder {
                    at: location_at(source, runtime_drafts[indice].path.span(), origin),
                    field,
                    placeholder,
                    expected: allowed.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Expande una entrada de `resolve`: `~` a `$HOME`, y `$PATH` a la búsqueda del
/// nombre del programa por los directorios del entorno.
fn expand_resolve(entry: &str, program: &Path) -> Vec<PathBuf> {
    if entry == "$PATH" {
        let Some(name) = program.file_name() else {
            return Vec::new();
        };
        std::env::var_os("PATH")
            .map(|path| {
                std::env::split_paths(&path)
                    .map(|dir| dir.join(name))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![expand_tilde(entry)]
    }
}

/// Expande una `~` inicial a `$HOME`; cualquier otra cosa se queda como está.
fn expand_tilde(entry: &str) -> PathBuf {
    if entry == "~" {
        home()
    } else if let Some(rest) = entry.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(entry)
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// ¿Existe `path` y es un fichero regular con algún bit de ejecución?
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && (metadata.permissions().mode() & 0o111) != 0)
}

/// SHA-256 del fichero, en hexadecimal minúscula.
///
/// R11 exige fijar el binario por versión **y** hash: el pin de un proveedor
/// decía `2.6.9`, existía, y el resolutor lo prefería mientras se ejecutaba la
/// `2.6.11`. Una versión que coincide no dice que el binario sea el mismo.
///
/// Se transmite el fichero al resumen en vez de leerlo entero: el binario de un
/// proveedor puede ocupar megabytes y no hay razón para tenerlo dos veces en
/// memoria.
fn sha256_hex(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};

    let mut fichero = std::fs::File::open(path)?;
    let mut resumen = Sha256::new();
    std::io::copy(&mut fichero, &mut resumen)?;

    Ok(hexadecimal(&resumen.finalize()))
}

/// El resumen del texto de un manifiesto.
///
/// Se calcula sobre lo que se interpretó, no sobre lo que hay en disco: un
/// manifiesto que se cargó y luego se editó tiene que seguir identificado por los
/// bytes que de verdad gobernaron la corrida.
fn sha256_texto(source: &str) -> String {
    use sha2::{Digest, Sha256};

    hexadecimal(&Sha256::digest(source.as_bytes()))
}

/// Hexadecimal minúsculo, que es como se escriben los resúmenes en todo el
/// proyecto y como los espera el manifiesto al comparar.
fn hexadecimal(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

impl Executable {
    /// El programa tal como lo nombra el manifiesto.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// La versión fijada.
    pub fn version_pin(&self) -> &str {
        &self.version_pin
    }

    /// Cómo se le pregunta la versión.
    pub fn version_probe(&self) -> &[String] {
        &self.version_probe
    }

    /// El hash fijado, si lo hay.
    ///
    /// R11 lo quiere siempre: una versión que coincide no dice que el binario sea
    /// el mismo, y el pin de un proveedor decía `2.6.9` mientras corría la
    /// `2.6.11`.
    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    /// Dónde buscarlo, en orden.
    pub fn resolve(&self) -> &[String] {
        &self.resolve
    }
}

impl Auth {
    /// Método de autenticación.
    pub fn method(&self) -> AuthMethod {
        self.method
    }

    /// Dónde guarda el CLI su propia sesión, si la guarda.
    pub fn store_path(&self) -> Option<&Path> {
        self.store_path.as_deref()
    }

    /// El nombre sellado de la credencial.
    ///
    /// **Sale de aquí y de ningún otro sitio** (R10): buscar `deepseek-api-key`
    /// lo que se había sellado como `qwen-deepseek-api-key` costó semanas sin
    /// credencial teniendo la clave en la máquina.
    pub fn credential(&self) -> Option<&str> {
        self.credential.as_deref()
    }

    /// La variable de entorno que lleva la credencial.
    pub fn env(&self) -> Option<&EnvVarName> {
        self.env.as_ref()
    }
}

impl Invoke {
    /// El `argv` con sus llaves todavía sin sustituir.
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// Dónde trabaja el proceso.
    pub fn workdir(&self) -> &str {
        &self.workdir
    }

    /// Por dónde entra el prompt.
    ///
    /// El techo de sensibilidad de cada vía lo fija el contrato, no este
    /// manifiesto: `argv` se lee desde `ps`.
    pub fn prompt_via(&self) -> PromptDelivery {
        self.prompt_via
    }

    /// La bandera con la que entra el prompt cuando viaja por fichero.
    pub fn prompt_flag(&self) -> Option<&str> {
        self.prompt_flag.as_deref()
    }
}

impl EnvPolicy {
    /// Lo único que se hereda del entorno. Nada más (R5).
    pub fn allow(&self) -> &[EnvVarName] {
        &self.allow
    }

    /// Lo que se deniega aunque estuviera permitido.
    ///
    /// Existe porque hay variables que el proveedor lee para decidir su propia
    /// contención: heredarlas movería la jaula sin que nadie lo pidiera.
    pub fn deny(&self) -> &[EnvVarName] {
        &self.deny
    }

    /// Lo que se fija explícitamente.
    pub fn set(&self) -> &[(EnvVarName, String)] {
        &self.set
    }
}

impl Canary {
    /// El prompt, con `{token}` sin sustituir.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Qué se comprueba, y se comprueba **observando** (R3).
    pub fn expect(&self) -> CanaryExpectation {
        self.expect
    }
}
