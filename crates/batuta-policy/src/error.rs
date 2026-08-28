//! El error de cargar o guardar una política.

use std::fmt;
use std::path::PathBuf;

use batuta_contract::SchemaVersionError;

/// Lo que puede salir mal leyendo o escribiendo `politica.toml`.
///
/// Deliberadamente más pequeño que el error de `batuta-manifest`: la política
/// es estado escrito por las propias órdenes de batuta (`enable`, `disable`,
/// `effort`), no un fichero editado a mano bajo presión. No necesita fichero y
/// línea por campo; necesita decir qué ruta y qué falló.
#[derive(Debug)]
pub enum PoliticaError {
    /// No se pudo leer el fichero.
    Read {
        /// La ruta que se intentó leer.
        path: PathBuf,
        /// El error del sistema de ficheros.
        source: std::io::Error,
    },
    /// No se pudo escribir el fichero.
    Write {
        /// La ruta que se intentó escribir.
        path: PathBuf,
        /// El error del sistema de ficheros.
        source: std::io::Error,
    },
    /// El texto no es TOML válido, o no tiene la forma de una política.
    Parse {
        /// La ruta que no se pudo interpretar.
        path: PathBuf,
        /// El error de `toml`.
        source: toml::de::Error,
    },
    /// Una versión de esquema que batuta no sabe leer (R1).
    SchemaVersion(SchemaVersionError),
    /// El documento no se pudo serializar a TOML.
    ///
    /// No debería poder pasar con los tipos que `Politica` guarda —todos
    /// serializables sin estado externo—, y por eso no lleva ruta: si
    /// ocurriera, sería un fallo del propio serializador, no del fichero.
    Serialize(toml::ser::Error),
}

impl fmt::Display for PoliticaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "no se pudo leer la política en {}: {source}",
                    path.display()
                )
            }
            Self::Write { path, source } => {
                write!(
                    f,
                    "no se pudo escribir la política en {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(f, "{} no es una política válida: {source}", path.display())
            }
            Self::SchemaVersion(source) => write!(f, "{source}"),
            Self::Serialize(source) => write!(f, "no se pudo serializar la política: {source}"),
        }
    }
}

impl std::error::Error for PoliticaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::SchemaVersion(source) => Some(source),
            Self::Serialize(source) => Some(source),
        }
    }
}
