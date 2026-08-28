//! Admisión por leases: por modelo y por repositorio.
//!
//! El fallo que lo paga (R6): `TaskStop` dejaba el hijo vivo gastando cuota **y
//! su lease de repositorio bloqueando a cualquier otro modelo** con
//! `AdmissionUnavailable`. Las dos mitades del fallo son la misma regla: el
//! proceso es el límite, y matar la tarea tiene que liberar lo que tomó.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::LeaseError;
use crate::owner::Owner;

/// Sobre qué se pide exclusión.
///
/// Dos, y no uno: un encargo ocupa **un modelo** —para no doblar la cuota del
/// mismo sitio— y **un repositorio** —para que dos escritores no se pisen—. Son
/// independientes: dos modelos distintos pueden trabajar en repositorios
/// distintos a la vez, y ése es justamente el caso que el sistema viejo
/// bloqueaba.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseSpace {
    /// Un modelo concreto de un proveedor.
    Model,
    /// Un repositorio, identificado por su ruta canónica.
    Repository,
}

/// Lo que hay dentro de un fichero de lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRecord {
    /// Espacio de nombres.
    pub space: LeaseSpace,
    /// Clave ocupada.
    pub key: String,
    /// Encargo que lo tomó, para que el mensaje de ocupado sea accionable.
    pub task_id: String,
    /// Quién lo tiene, con su prueba de vida.
    pub owner: Owner,
    /// Cuándo se tomó, en segundos desde la época.
    ///
    /// **Es informativo y nada más.** Ninguna decisión de este crate mira esta
    /// marca: la caducidad se decide por evidencia, no por antigüedad.
    pub acquired_at: u64,
}

impl LeaseRecord {
    /// ¿Sigue vivo el dueño de este lease?
    pub fn is_held(&self) -> bool {
        todo!("delegar en Owner::is_alive")
    }
}

/// El directorio donde viven los leases.
#[derive(Debug, Clone)]
pub struct LeaseStore {
    _root: PathBuf,
}

impl LeaseStore {
    /// Abre —o crea— el almacén bajo `root`.
    ///
    /// # Errors
    ///
    /// Si el directorio no se puede crear.
    pub fn open(_root: &Path) -> Result<Self, LeaseError> {
        todo!()
    }

    /// Dónde vive el fichero de un lease.
    ///
    /// Es API pública a propósito: **la disposición en disco es parte del
    /// contrato**, no un detalle. Un lease es un hecho que otros procesos —y una
    /// persona con `ls`— tienen que poder inspeccionar sin pedirle permiso a
    /// batuta. La clave se normaliza, porque una ruta de repositorio lleva
    /// barras.
    pub fn path_for(&self, _space: LeaseSpace, _key: &str) -> PathBuf {
        todo!()
    }

    /// Toma un lease, o falla nombrando a quien lo tiene.
    ///
    /// La adquisición es una creación exclusiva: quien pierde la carrera **no
    /// espera**. Un encargo que se queda esperando es un encargo que consume una
    /// ranura sin hacer nada, y el sistema viejo se atascaba así.
    ///
    /// Si el fichero existe pero su dueño **está demostrablemente muerto**, se
    /// reclama. Si el dueño vive, se rechaza, por muy viejo que sea el lease.
    ///
    /// # Errors
    ///
    /// [`LeaseError::AdmissionUnavailable`] si otro lo tiene vivo; `Store` o
    /// `Corrupt` si el almacén no está en condiciones.
    pub fn acquire(
        &self,
        _space: LeaseSpace,
        _key: &str,
        _task_id: &str,
    ) -> Result<LeaseGuard, LeaseError> {
        todo!()
    }

    /// Enumera los leases de un espacio.
    ///
    /// **No toma ningún cerrojo, y ésa es la propiedad**: R9 exige que la
    /// inspección no haga cola. Dos `orchestrator_inventory` se fueron a segundo
    /// plano tras 120 s porque había una delegación en curso; aquí eso no puede
    /// pasar, porque listar es leer.
    ///
    /// # Errors
    ///
    /// Si el directorio no se puede leer.
    pub fn list(&self, _space: LeaseSpace) -> Result<Vec<LeaseRecord>, LeaseError> {
        todo!()
    }
}

/// Un lease tomado. Se libera al soltarlo.
///
/// Es un guardián y no un identificador a propósito: un lease que hay que
/// acordarse de liberar es un lease que algún día no se libera.
#[derive(Debug)]
pub struct LeaseGuard {
    _record: LeaseRecord,
    _path: PathBuf,
}

impl LeaseGuard {
    /// El registro de este lease.
    pub fn record(&self) -> &LeaseRecord {
        todo!()
    }
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        todo!("borrar el fichero; un fallo aquí no debe entrar en pánico")
    }
}
