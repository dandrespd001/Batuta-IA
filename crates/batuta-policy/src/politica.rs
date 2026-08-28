//! `Politica`: qué modelos queremos usar, y con qué esfuerzo.

use std::collections::BTreeMap;
use std::path::Path;

use batuta_contract::{ModelId, ReasoningEffort, SchemaVersion};
use serde::{Deserialize, Serialize};

use crate::error::PoliticaError;

/// La elección para un modelo. Sin `Default` (R13): un campo que nadie fija
/// no compila, así que un tercer campo el día de mañana rompe cada llamada
/// que construya uno, en vez de heredar un valor que nadie escribió.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleccionModelo {
    /// Si este modelo puede enrutarse.
    pub habilitado: bool,
    /// El nivel de esfuerzo a pedir, si se pide alguno. `None` significa «sin
    /// preferencia»: el proveedor recibe el suyo propio, no uno inventado.
    #[serde(default)]
    pub esfuerzo: Option<ReasoningEffort>,
}

/// El fichero de elección completo: por cada modelo que alguien ha tocado
/// alguna vez con `enable`, `disable` o `effort`, su [`EleccionModelo`].
///
/// **Un modelo que la política no menciona nace apagado.** Es la misma
/// disciplina que R5 aplica al entorno: nada se hereda sin nombrarlo. La
/// alternativa —nacer activo— routearía a un modelo en cuanto su canario
/// pasara, sin que nadie lo hubiera elegido; la política existe precisamente
/// para separar «es enrutable» (lo dice el manifiesto) de «lo queremos
/// enrutar» (lo dice este fichero), y una cosa no puede implicar la otra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Politica {
    modelos: BTreeMap<ModelId, EleccionModelo>,
}

/// Lo que se (de)serializa. `schema_version` es explícito y se valida por
/// separado (R1): un documento con una versión que batuta no conoce falla al
/// cargar, no a mitad de una corrida.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Documento {
    schema_version: SchemaVersion,
    #[serde(default)]
    modelos: BTreeMap<ModelId, EleccionModelo>,
}

impl Politica {
    /// Una política sin ningún modelo mencionado: el estado del primer
    /// arranque, antes de que nadie habilite nada.
    pub fn vacia() -> Self {
        Self {
            modelos: BTreeMap::new(),
        }
    }

    /// Fija la elección completa de un modelo, reemplazando la que hubiera.
    pub fn fijar(&mut self, id: ModelId, eleccion: EleccionModelo) {
        self.modelos.insert(id, eleccion);
    }

    /// La elección declarada para un modelo, si la política lo menciona.
    pub fn eleccion(&self, id: &ModelId) -> Option<&EleccionModelo> {
        self.modelos.get(id)
    }

    /// Si un modelo puede enrutarse. Uno que la política no menciona nace
    /// **apagado** (ver el doc de [`Politica`]).
    pub fn esta_habilitado(&self, id: &ModelId) -> bool {
        self.modelos.get(id).is_some_and(|e| e.habilitado)
    }

    /// El esfuerzo declarado para un modelo. `None` tanto si la política no
    /// lo menciona como si lo menciona sin fijar un nivel: las dos veces
    /// significan lo mismo, «sin preferencia».
    pub fn esfuerzo(&self, id: &ModelId) -> Option<ReasoningEffort> {
        self.modelos.get(id)?.esfuerzo
    }

    /// Carga la política de disco.
    ///
    /// # Errors
    ///
    /// [`PoliticaError::Read`] si no se pudo leer, [`PoliticaError::Parse`] si
    /// el TOML no tiene la forma esperada, [`PoliticaError::SchemaVersion`] si
    /// la versión no se admite.
    pub fn cargar(path: &Path) -> Result<Self, PoliticaError> {
        let texto = std::fs::read_to_string(path).map_err(|source| PoliticaError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let documento: Documento =
            toml::from_str(&texto).map_err(|source| PoliticaError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        documento
            .schema_version
            .require_supported()
            .map_err(PoliticaError::SchemaVersion)?;
        Ok(Self {
            modelos: documento.modelos,
        })
    }

    /// Guarda la política en disco, sobrescribiendo lo que hubiera.
    ///
    /// # Errors
    ///
    /// [`PoliticaError::Serialize`] si el documento no se pudo serializar,
    /// [`PoliticaError::Write`] si no se pudo escribir.
    pub fn guardar(&self, path: &Path) -> Result<(), PoliticaError> {
        let documento = Documento {
            schema_version: SchemaVersion::CURRENT,
            modelos: self.modelos.clone(),
        };
        let texto = toml::to_string_pretty(&documento).map_err(PoliticaError::Serialize)?;
        std::fs::write(path, texto).map_err(|source| PoliticaError::Write {
            path: path.to_path_buf(),
            source,
        })
    }
}
