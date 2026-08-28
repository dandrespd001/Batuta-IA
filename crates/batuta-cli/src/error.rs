//! Lo que impide llegar a tener un veredicto.
//!
//! Ninguno de estos es un canario rojo. Un canario rojo **es** una respuesta: el
//! proveedor contestó y lo que contestó no vale. Esto otro es no haber llegado a
//! preguntar, y confundir las dos cosas es cómo un informe acaba diciendo que
//! algo se hizo cuando no se hizo.

use std::fmt;
use std::path::PathBuf;

/// Por qué la orden no llegó a producir un recibo.
#[derive(Debug)]
#[non_exhaustive]
pub enum CliError {
    /// La orden no existe.
    UnknownCommand {
        /// Lo que se escribió.
        given: String,
        /// Las que hay.
        available: Vec<&'static str>,
    },
    /// Una bandera vino sin su valor.
    ///
    /// No se traga el siguiente argumento: `--provider --model dsh` es un error,
    /// no un proveedor llamado `--model`.
    MissingValue {
        /// La bandera.
        flag: String,
    },
    /// Una bandera obligatoria no vino.
    MissingFlag {
        /// La bandera.
        flag: &'static str,
    },
    /// Dos banderas que se contradicen.
    ///
    /// No se resuelve por preferencia ni por orden de aparición: elegir en
    /// silencio entre dos instrucciones incompatibles es la forma exacta en que
    /// se pidió un modelo y corrió otro.
    ContradictoryFlags {
        /// Una.
        one: &'static str,
        /// La otra.
        other: &'static str,
    },
    /// Una bandera que no se admite.
    UnknownFlag {
        /// Lo que se escribió.
        given: String,
        /// Las que hay.
        available: Vec<&'static str>,
    },
    /// Ningún manifiesto del directorio declara ese proveedor.
    ///
    /// El error **enumera los que sí hay** (R8): un `"provider not found"`
    /// obliga a ir a mirar el directorio a mano.
    UnknownProvider {
        /// Lo que se pidió.
        asked: String,
        /// Los que hay.
        available: Vec<String>,
    },
    /// El proveedor declara varios modelos y no se pidió ninguno.
    ///
    /// **No se elige en silencio.** Elegir en silencio es exactamente cómo se
    /// pidió un modelo tres veces y corrió otro las tres.
    AmbiguousModel {
        /// El proveedor.
        provider: String,
        /// Sus modelos.
        available: Vec<String>,
    },
    /// El modelo pedido no es de ese proveedor.
    UnknownModel {
        /// Lo que se pidió.
        asked: String,
        /// El proveedor.
        provider: String,
        /// Los suyos.
        available: Vec<String>,
    },
    /// Un manifiesto del directorio no se pudo cargar.
    ///
    /// Un manifiesto irresoluble falla **al cargar**, no a mitad de una corrida
    /// (R1): el directorio entero se lee antes de tocar nada.
    ///
    /// La causa va en caja: un `ManifestError` lleva fichero, línea y columna, y
    /// sin la caja cada `Ok` de cada función que pueda devolver este error
    /// pagaría ese tamaño. El error raro no debe encarecer el camino común.
    Manifest {
        /// Causa.
        source: Box<batuta_manifest::ManifestError>,
    },
    /// La corrida no se pudo llevar a cabo.
    Exec {
        /// Causa.
        source: Box<batuta_exec::ExecError>,
    },
    /// El disco no cooperó.
    Io {
        /// Qué se intentaba.
        path: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { given, available } => write!(
                f,
                "`{given}` no es una orden de batuta; las que hay: {}",
                available.join(", ")
            ),
            Self::MissingValue { flag } => {
                write!(f, "`{flag}` necesita un valor y vino sola")
            }
            Self::MissingFlag { flag } => write!(f, "falta `{flag}`"),
            Self::ContradictoryFlags { one, other } => {
                write!(f, "`{one}` y `{other}` se contradicen: elige una")
            }
            Self::UnknownFlag { given, available } => write!(
                f,
                "`{given}` no es una bandera de esta orden; las que hay: {}",
                available.join(", ")
            ),
            Self::UnknownProvider { asked, available } => write!(
                f,
                "no hay ningún proveedor `{asked}`; los que hay: {}",
                available.join(", ")
            ),
            Self::AmbiguousModel {
                provider,
                available,
            } => write!(
                f,
                "`{provider}` declara varios modelos: elige uno con `--model` entre {}",
                available.join(", ")
            ),
            Self::UnknownModel {
                asked,
                provider,
                available,
            } => write!(
                f,
                "`{provider}` no declara ningún modelo `{asked}`; los suyos: {}",
                available.join(", ")
            ),
            Self::Manifest { source } => write!(f, "{source}"),
            Self::Exec { source } => write!(f, "{source}"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest { source } => Some(source),
            Self::Exec { source } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
