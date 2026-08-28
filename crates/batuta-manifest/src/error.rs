// generado: deepseek-v4-flash - revisado: Arquitecto
//! El error de carga. **Cada variante nombra el fichero, la línea y el campo.**
//!
//! R1 se paga aquí: `abacus_cli` estaba declarado en un registro y ausente del
//! otro, así que la tarea fallaba *después* de pagar la corrida. Un manifiesto
//! incoherente tiene que reventar al cargarse, cuando aún no ha costado nada.
//!
//! Y R8 manda sobre los mensajes: el que rechaza un valor **lista los válidos**.
//! `unknown output_contract: 'patch'` sin decir cuáles valían es el fallo que
//! esa regla paga.

use std::fmt;
use std::path::PathBuf;

use batuta_contract::{IdentifierError, SchemaVersionError, VocabularyError};

/// Dónde ocurrió: fichero y línea, para que el mensaje sea accionable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Fichero del manifiesto.
    pub file: PathBuf,
    /// Línea, empezando en 1.
    pub line: u32,
}

/// Manifiesto que no se puede cargar.
#[derive(Debug)]
pub enum ManifestError {
    /// El fichero no se pudo leer.
    Read {
        /// Ruta que se intentó leer.
        file: PathBuf,
        /// Causa del sistema de ficheros.
        source: std::io::Error,
    },
    /// Versión de esquema que batuta no sabe leer.
    ///
    /// No es «TOML mal formado»: el fichero está bien y lo que no encaja es el
    /// acuerdo sobre su forma. Decir una cosa por otra en el mensaje es
    /// exactamente lo que R8 evita, y quien lo lea se pondría a buscar una coma
    /// que no falta.
    UnsupportedSchemaVersion {
        /// Dónde.
        at: SourceLocation,
        /// El error del contrato, que ya enumera las versiones admitidas.
        source: SchemaVersionError,
    },
    /// TOML mal formado.
    Syntax {
        /// Dónde.
        at: SourceLocation,
        /// Mensaje del analizador.
        message: String,
    },
    /// Un valor fuera de un vocabulario cerrado.
    Vocabulary {
        /// Dónde.
        at: SourceLocation,
        /// Campo, en notación punteada.
        field: String,
        /// El error que ya lista los valores válidos (R8).
        source: VocabularyError,
    },
    /// Un identificador mal formado.
    Identifier {
        /// Dónde.
        at: SourceLocation,
        /// Campo, en notación punteada.
        field: String,
        /// Qué tenía de malo.
        source: IdentifierError,
    },
    /// El programa del manifiesto no existe o no es ejecutable. **Es R1.**
    ExecutableNotFound {
        /// Dónde.
        at: SourceLocation,
        /// Lo que se buscó.
        program: PathBuf,
        /// Las rutas que se probaron, en orden.
        tried: Vec<PathBuf>,
    },
    /// El binario no coincide con el `sha256` fijado (R11).
    DigestMismatch {
        /// Dónde.
        at: SourceLocation,
        /// Lo que decía el manifiesto.
        expected: String,
        /// Lo que hay en disco.
        found: String,
    },
    /// Un `[[runtime_files]]` con ruta absoluta.
    RuntimeFilePathAbsolute {
        /// Dónde.
        at: SourceLocation,
        /// La ruta ofensiva.
        path: PathBuf,
    },
    /// Un `[[runtime_files]]` que se sale del directorio de corrida con `..`.
    RuntimeFilePathEscapes {
        /// Dónde.
        at: SourceLocation,
        /// La ruta ofensiva.
        path: PathBuf,
    },
    /// Un `[[runtime_files]]` que declara `entry` y `content` a la vez.
    DocumentShapeAmbiguous {
        /// Dónde.
        at: SourceLocation,
        /// La ruta del documento afectado.
        path: PathBuf,
    },
    /// Un `[[runtime_files]]` que no declara ninguno de los dos.
    DocumentShapeMissing {
        /// Dónde.
        at: SourceLocation,
        /// La ruta del documento afectado.
        path: PathBuf,
    },
    /// Una llave `{...}` que no es ni incorporada ni declarada.
    UnknownPlaceholder {
        /// Dónde.
        at: SourceLocation,
        /// Campo donde aparece.
        field: String,
        /// La llave que no se reconoce.
        placeholder: String,
        /// Todas las admitidas, incorporadas y declaradas (R8).
        expected: Vec<String>,
    },
    /// Un mapa de sustitución que no cubre su vocabulario entero.
    ///
    /// Es el invariante que hace que añadir un `write_mode` rompa en voz alta en
    /// vez de elegir en silencio.
    SubstitutionIncomplete {
        /// Dónde.
        at: SourceLocation,
        /// La clave de sustitución incompleta.
        key: String,
        /// Vocabulario que debía cubrir.
        vocabulary: &'static str,
        /// Las variantes que faltan.
        missing: Vec<&'static str>,
    },
    /// La misma variable de entorno se fija y se deniega.
    ///
    /// No se resuelve eligiendo ganador. Cualquiera de las dos respuestas deja un
    /// manifiesto que dice dos cosas contrarias y una de ellas no se cumple, en
    /// silencio. Es el mismo criterio que R1 aplica al ejecutor: lo incoherente
    /// falla al **cargar**, no en la corrida.
    ConflictingEnvVar {
        /// Dónde.
        at: SourceLocation,
        /// La variable que aparece en `set` y en `deny`.
        name: String,
    },
    /// El prompt entra por `argv` y el `argv` no lo emite en ninguna posición.
    ///
    /// El encargo se perdería en el camino: el proveedor recibiría una llamada
    /// sin tarea, contestaría algo plausible, y el recibo lo sellaría. Es la
    /// clase de fallo que este proyecto existe para impedir —algo declarado que
    /// nadie demuestra (R2)—, y `providers/abacus.toml` lo tenía.
    PromptNeverDelivered {
        /// Dónde.
        at: SourceLocation,
    },
    /// Un manifiesto sin ningún `[[models]]`.
    NoModels {
        /// Dónde.
        at: SourceLocation,
    },
    /// Dos `[[models]]` con el mismo `id`.
    DuplicateModel {
        /// Dónde.
        at: SourceLocation,
        /// El identificador repetido.
        id: String,
    },
}

/// Una lista de valores separada por comas, para los mensajes que R8 exige.
fn lista<T: fmt::Display>(f: &mut fmt::Formatter<'_>, valores: &[T]) -> fmt::Result {
    for (indice, valor) in valores.iter().enumerate() {
        if indice > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{valor}")?;
    }
    Ok(())
}

/// El `Display` es un `match` lineal, una variante por mensaje: partirlo en
/// funciones no lo haría más legible y sí más difícil de contrastar contra las
/// variantes.
#[allow(clippy::too_many_lines)]
impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { file, source } => {
                write!(f, "no se pudo leer {}: {source}", file.display())
            }
            Self::UnsupportedSchemaVersion { at, source } => {
                write!(f, "{}:{}: {source}", at.file.display(), at.line)
            }
            Self::Syntax { at, message } => write!(
                f,
                "{}:{}: TOML inválido: {message}",
                at.file.display(),
                at.line
            ),
            Self::Vocabulary { at, field, source } => write!(
                f,
                "{}:{}: campo `{field}`: {source}",
                at.file.display(),
                at.line
            ),
            Self::Identifier { at, field, source } => write!(
                f,
                "{}:{}: campo `{field}`: {source}",
                at.file.display(),
                at.line
            ),
            Self::ExecutableNotFound { at, program, tried } => {
                write!(
                    f,
                    "{}:{}: ejecutable `{}` no encontrado; probado: ",
                    at.file.display(),
                    at.line,
                    program.display()
                )?;
                for (indice, ruta) in tried.iter().enumerate() {
                    if indice > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "`{}`", ruta.display())?;
                }
                Ok(())
            }
            Self::DigestMismatch {
                at,
                expected,
                found,
            } => write!(
                f,
                "{}:{}: sha256 no coincide: se esperaba `{expected}` y se encontró `{found}`",
                at.file.display(),
                at.line
            ),
            Self::RuntimeFilePathAbsolute { at, path } => write!(
                f,
                "{}:{}: ruta absoluta en runtime_files: `{}`",
                at.file.display(),
                at.line,
                path.display()
            ),
            Self::RuntimeFilePathEscapes { at, path } => write!(
                f,
                "{}:{}: ruta que escapa del directorio de corrida: `{}`",
                at.file.display(),
                at.line,
                path.display()
            ),
            Self::DocumentShapeAmbiguous { at, path } => write!(
                f,
                "{}:{}: documento `{}` declara `entry` y `content` a la vez",
                at.file.display(),
                at.line,
                path.display()
            ),
            Self::DocumentShapeMissing { at, path } => write!(
                f,
                "{}:{}: documento `{}` no declara ni `entry` ni `content`",
                at.file.display(),
                at.line,
                path.display()
            ),
            Self::UnknownPlaceholder {
                at,
                field,
                placeholder,
                expected,
            } => {
                write!(
                    f,
                    "{}:{}: llave de sustitución desconocida `{{{placeholder}}}` en `{field}`; admitidas: ",
                    at.file.display(),
                    at.line
                )?;
                for (indice, admitida) in expected.iter().enumerate() {
                    if indice > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{{{admitida}}}")?;
                }
                Ok(())
            }
            Self::SubstitutionIncomplete {
                at,
                key,
                vocabulary,
                missing,
            } => {
                write!(
                    f,
                    "{}:{}: la sustitución `{key}` no cubre el vocabulario `{vocabulary}`; faltan: ",
                    at.file.display(),
                    at.line
                )?;
                lista(f, missing)
            }
            Self::PromptNeverDelivered { at } => write!(
                f,
                "{}:{}: `invoke.prompt.via` es `argv` y el `invoke.argv` no lleva \
                 `{{prompt}}` en ninguna posición: el encargo no llegaría al proveedor",
                at.file.display(),
                at.line
            ),
            Self::NoModels { at } => write!(
                f,
                "{}:{}: el manifiesto no declara ningún `[[models]]`",
                at.file.display(),
                at.line
            ),
            Self::ConflictingEnvVar { at, name } => write!(
                f,
                "{}:{}: `{name}` está a la vez en `env.set` y en `env.deny`: un manifiesto no \
                 puede fijar y denegar la misma variable",
                at.file.display(),
                at.line
            ),
            Self::DuplicateModel { at, id } => write!(
                f,
                "{}:{}: modelo duplicado: `{id}`",
                at.file.display(),
                at.line
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl ManifestError {
    /// Dónde ocurrió, si la variante lo sabe.
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            Self::Read { .. } => None,
            Self::UnsupportedSchemaVersion { at, .. }
            | Self::Syntax { at, .. }
            | Self::Vocabulary { at, .. }
            | Self::Identifier { at, .. }
            | Self::ExecutableNotFound { at, .. }
            | Self::DigestMismatch { at, .. }
            | Self::RuntimeFilePathAbsolute { at, .. }
            | Self::RuntimeFilePathEscapes { at, .. }
            | Self::DocumentShapeAmbiguous { at, .. }
            | Self::DocumentShapeMissing { at, .. }
            | Self::UnknownPlaceholder { at, .. }
            | Self::SubstitutionIncomplete { at, .. }
            | Self::PromptNeverDelivered { at, .. }
            | Self::NoModels { at, .. }
            | Self::ConflictingEnvVar { at, .. }
            | Self::DuplicateModel { at, .. } => Some(at),
        }
    }
}
