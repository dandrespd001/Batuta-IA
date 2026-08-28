// generado: deepseek-v4-flash - revisado: Arquitecto
//! Admisión por leases: por modelo y por repositorio.
//!
//! El fallo que lo paga (R6): `TaskStop` dejaba el hijo vivo gastando cuota **y
//! su lease de repositorio bloqueando a cualquier otro modelo** con
//! `AdmissionUnavailable`. Las dos mitades del fallo son la misma regla: el
//! proceso es el límite, y matar la tarea tiene que liberar lo que tomó.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
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

impl fmt::Display for LeaseSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model => f.write_str("modelo"),
            Self::Repository => f.write_str("repositorio"),
        }
    }
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
        self.owner.is_alive()
    }
}

/// El directorio donde viven los leases.
#[derive(Debug, Clone)]
pub struct LeaseStore {
    root: PathBuf,
}

impl LeaseStore {
    /// Abre —o crea— el almacén bajo `root`.
    ///
    /// # Errors
    ///
    /// Si el directorio no se puede crear.
    pub fn open(root: &Path) -> Result<Self, LeaseError> {
        fs::create_dir_all(root).map_err(|source| LeaseError::Store {
            path: root.to_path_buf(),
            source,
        })?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Dónde vive el fichero de un lease.
    ///
    /// Es API pública a propósito: **la disposición en disco es parte del
    /// contrato**, no un detalle. Un lease es un hecho que otros procesos —y una
    /// persona con `ls`— tienen que poder inspeccionar sin pedirle permiso a
    /// batuta.
    ///
    /// La disposición: `<root>/<espacio>/<clave normalizada>`, donde `<espacio>`
    /// es `model` o `repository` —los mismos nombres que `serde` escribe en el
    /// JSON, para que `ls` y el contenido cuenten lo mismo—. La clave se
    /// normaliza porque una ruta de repositorio lleva barras, que no caben en un
    /// nombre de fichero: cada `/` se escribe como `%` y cada `%` se duplica
    /// (`%%`). La inversa es directa y visible: `%%` es `%` y `%` es `/`. Un
    /// modelo (`dsh-deepseek-v4-flash`) queda tal cual; una ruta de repositorio
    /// (`/tmp/otro-repo`) queda `%tmp%otro-repo`.
    pub fn path_for(&self, space: LeaseSpace, key: &str) -> PathBuf {
        self.root.join(dir_name(space)).join(normalizar_clave(key))
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
        space: LeaseSpace,
        key: &str,
        task_id: &str,
    ) -> Result<LeaseGuard, LeaseError> {
        let ruta = self.path_for(space, key);
        let padre = ruta
            .parent()
            .ok_or_else(|| LeaseError::Store {
                path: ruta.clone(),
                source: io::Error::new(ErrorKind::InvalidInput, "la ruta del lease no tiene padre"),
            })?
            .to_path_buf();
        fs::create_dir_all(&padre).map_err(|source| LeaseError::Store {
            path: padre,
            source,
        })?;

        let registro = LeaseRecord {
            space,
            key: key.to_string(),
            task_id: task_id.to_string(),
            owner: Owner::current().map_err(|source| LeaseError::Store {
                path: ruta.clone(),
                source,
            })?,
            acquired_at: ahora(),
        };
        let bytes = serde_json::to_vec(&registro).map_err(|e| LeaseError::Corrupt {
            path: ruta.clone(),
            detail: format!("no se pudo serializar el registro: {e}"),
        })?;

        match OpenOptions::new().write(true).create_new(true).open(&ruta) {
            // La carrera se ganó: el fichero es nuestro.
            Ok(mut fichero) => {
                if let Err(source) = fichero.write_all(&bytes) {
                    // No dejar un fichero a medias que parezca un lease corrupto.
                    let _ = fs::remove_file(&ruta);
                    return Err(LeaseError::Store { path: ruta, source });
                }
                Ok(LeaseGuard {
                    record: registro,
                    path: ruta,
                })
            }
            // El fichero ya existía: decidir por evidencia, nunca por antigüedad.
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                let viejos = fs::read(&ruta).map_err(|source| LeaseError::Store {
                    path: ruta.clone(),
                    source,
                })?;
                let existente: LeaseRecord =
                    serde_json::from_slice(&viejos).map_err(|e| LeaseError::Corrupt {
                        path: ruta.clone(),
                        detail: format!("JSON inválido: {e}"),
                    })?;
                if existente.owner.is_alive() {
                    Err(LeaseError::AdmissionUnavailable {
                        space,
                        key: key.to_string(),
                        held_by: Box::new(existente),
                    })
                } else {
                    // Dueño demostrablemente muerto: el lease se reclama.
                    // Sobrescribir es la operación que la evidencia autoriza.
                    fs::write(&ruta, &bytes).map_err(|source| LeaseError::Store {
                        path: ruta.clone(),
                        source,
                    })?;
                    Ok(LeaseGuard {
                        record: registro,
                        path: ruta,
                    })
                }
            }
            Err(source) => Err(LeaseError::Store { path: ruta, source }),
        }
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
    /// Si el directorio no se puede leer, o un fichero de lease no se puede
    /// interpretar: un lease ilegible no es un lease libre, es un estado que hay
    /// que mirar.
    pub fn list(&self, space: LeaseSpace) -> Result<Vec<LeaseRecord>, LeaseError> {
        let directorio = self.root.join(dir_name(space));
        let entradas = match fs::read_dir(&directorio) {
            Ok(entradas) => entradas,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(LeaseError::Store {
                    path: directorio,
                    source,
                });
            }
        };
        let mut vistos = Vec::new();
        for entrada in entradas {
            let entrada = entrada.map_err(|source| LeaseError::Store {
                path: directorio.clone(),
                source,
            })?;
            let tipo = entrada.file_type().map_err(|source| LeaseError::Store {
                path: directorio.clone(),
                source,
            })?;
            if !tipo.is_file() {
                continue;
            }
            let ruta = entrada.path();
            let bytes = fs::read(&ruta).map_err(|source| LeaseError::Store {
                path: ruta.clone(),
                source,
            })?;
            let registro: LeaseRecord =
                serde_json::from_slice(&bytes).map_err(|e| LeaseError::Corrupt {
                    path: ruta,
                    detail: format!("JSON inválido: {e}"),
                })?;
            vistos.push(registro);
        }
        vistos.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(vistos)
    }
}

/// El nombre del subdirectorio de un espacio: el mismo que `serde` usa en disco.
const fn dir_name(space: LeaseSpace) -> &'static str {
    match space {
        LeaseSpace::Model => "model",
        LeaseSpace::Repository => "repository",
    }
}

/// La clave en forma de nombre de fichero, reversible a simple vista.
///
/// Cada `/` se vuelve `%` y cada `%` se duplica: la inversa es `%%` → `%` y
/// `%` → `/`. Ver [`LeaseStore::path_for`].
fn normalizar_clave(clave: &str) -> String {
    clave.replace('%', "%%").replace('/', "%")
}

/// Segundos desde la época. Es informativo: ninguna decisión lo mira.
fn ahora() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duracion) => duracion.as_secs(),
        Err(_) => 0,
    }
}

/// Un lease tomado. Se libera al soltarlo.
///
/// Es un guardián y no un identificador a propósito: un lease que hay que
/// acordarse de liberar es un lease que algún día no se libera.
#[derive(Debug)]
pub struct LeaseGuard {
    record: LeaseRecord,
    path: PathBuf,
}

impl LeaseGuard {
    /// El registro de este lease.
    pub fn record(&self) -> &LeaseRecord {
        &self.record
    }
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        // Liberar no puede entrar en pánico: un fallo al borrar se ignora en
        // silencio, y el lease queda reclamable cuando su dueño muera.
        let _ = fs::remove_file(&self.path);
    }
}
