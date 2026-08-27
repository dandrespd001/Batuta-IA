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

use batuta_contract::{IdentifierError, VocabularyError};

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

impl fmt::Display for ManifestError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!(
            "los mensajes los fija tests/carga.rs: nombran fichero, línea, campo y valores válidos"
        )
    }
}

impl std::error::Error for ManifestError {}

impl ManifestError {
    /// Dónde ocurrió, si la variante lo sabe.
    pub fn location(&self) -> Option<&SourceLocation> {
        todo!("cada variante salvo Read lleva su SourceLocation")
    }
}
