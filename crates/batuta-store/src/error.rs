//! El error de consultar el almacén de recibos.

use std::fmt;
use std::path::PathBuf;

/// Lo que puede impedir **el escaneo entero**, no un recibo suelto.
///
/// Un recibo ilegible no es esto: va a `Lookup::unreadable`, porque el resto
/// del almacén sigue siendo consultable. Esto es para cuando el propio
/// directorio no se puede listar.
#[derive(Debug)]
pub enum StoreError {
    /// No se pudo listar o leer el directorio de recibos.
    Read {
        /// La ruta que falló.
        path: PathBuf,
        /// El error del sistema de ficheros.
        source: std::io::Error,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "no se pudo leer {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
        }
    }
}
