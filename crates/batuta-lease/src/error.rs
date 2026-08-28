// generado: deepseek-v4-flash - revisado: Arquitecto
//! Por qué no se pudo admitir un encargo.

use std::fmt;
use std::path::PathBuf;

use crate::lease::{LeaseRecord, LeaseSpace};

/// La admisión falló.
#[derive(Debug)]
pub enum LeaseError {
    /// Otro encargo tiene el lease, y **sigue vivo**.
    ///
    /// El error **nombra al dueño**. El sistema viejo devolvía
    /// `AdmissionUnavailable` a secas, y saber que algo está ocupado sin saber
    /// quién lo ocupa no permite hacer nada al respecto.
    AdmissionUnavailable {
        /// Espacio de nombres del lease.
        space: LeaseSpace,
        /// La clave disputada.
        key: String,
        /// Quién lo tiene, con su encargo y su proceso.
        held_by: Box<LeaseRecord>,
    },
    /// El directorio de leases no se pudo usar.
    Store {
        /// Ruta implicada.
        path: PathBuf,
        /// Causa del sistema de ficheros.
        source: std::io::Error,
    },
    /// Un fichero de lease existe pero no se puede interpretar.
    ///
    /// **No se borra a la ligera.** Un lease ilegible no es un lease libre: es un
    /// estado que hay que mirar, igual que una procedencia ilegible no es una
    /// corrida sin procedencia.
    Corrupt {
        /// Ruta del fichero.
        path: PathBuf,
        /// Qué tenía de malo.
        detail: String,
    },
}

impl fmt::Display for LeaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdmissionUnavailable {
                space,
                key,
                held_by,
            } => {
                let task_id = &held_by.task_id;
                let pid = held_by.owner.pid;
                write!(
                    f,
                    "lease de {space} ocupado: la clave `{key}` la tiene el encargo `{task_id}` \
                     (pid {pid})"
                )
            }
            Self::Store { path, source } => write!(
                f,
                "no se pudo usar el almacén en `{}`: {source}",
                path.display()
            ),
            Self::Corrupt { path, detail } => {
                write!(f, "lease ilegible en `{}`: {detail}", path.display())
            }
        }
    }
}

impl std::error::Error for LeaseError {}
