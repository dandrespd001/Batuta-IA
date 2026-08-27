// generado: deepseek-v4-flash - revisado: Arquitecto
//! Traducción entre el vocabulario de batuta y el del proveedor.
//!
//! dsh llama `workspace-write` a lo que batuta llama `validated_patch`. Meter esa
//! equivalencia en `batuta-contract` sería meter el vocabulario de dsh en el
//! núcleo, y el núcleo no conoce ningún proveedor. Así que la declara el
//! manifiesto:
//!
//! ```toml
//! [substitutions.sandbox_mode]
//! read_only       = "read-only"
//! validated_patch = "workspace-write"
//! validated_apply = "workspace-write"
//! ```
//!
//! **El invariante que esto regala:** el mapa tiene que cubrir el vocabulario
//! entero. Si mañana entra un `write_mode` nuevo, todo manifiesto que no lo
//! contemple falla al cargar nombrando el que falta, en vez de caer en un valor
//! por defecto que nadie escribió.
//!
//! Hoy toda sustitución declarada deriva de `write_mode`, y por eso el tipo dice
//! `WriteMode` en vez de generalizar a «un vocabulario cualquiera». Cuando
//! aparezca una que derive de otro, el tipo crece. Antes no (R2).

use std::collections::BTreeMap;

use batuta_contract::WriteMode;

/// Llaves que batuta rellena sin que nadie las declare.
///
/// El orden es el del mensaje de error, y por eso está fijado por una prueba.
pub const BUILTIN_PLACEHOLDERS: &[&str] = &[
    "model",
    "route_model",
    "route_provider",
    "workdir",
    "run_dir",
    "prompt",
    "token",
];

/// Los mapas de sustitución de un manifiesto, ya validados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitutions {
    map: BTreeMap<String, BTreeMap<WriteMode, String>>,
}

impl Substitutions {
    /// Construye el mapa ya validado. Sólo lo usa la carga.
    pub(crate) fn new(map: BTreeMap<String, BTreeMap<WriteMode, String>>) -> Self {
        Self { map }
    }

    /// Las llaves declaradas, en orden alfabético.
    pub fn declared_keys(&self) -> Vec<&str> {
        self.map.keys().map(String::as_str).collect()
    }

    /// Todas las llaves admitidas: incorporadas y declaradas.
    ///
    /// Es la lista que sale en el error de R8 cuando alguien escribe una que no
    /// existe.
    pub fn allowed_placeholders(&self) -> Vec<String> {
        let mut out: Vec<String> = BUILTIN_PLACEHOLDERS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        for key in self.map.keys() {
            if !out.iter().any(|admitida| admitida == key) {
                out.push(key.clone());
            }
        }
        out
    }

    /// El valor de una llave declarada para un modo de escritura.
    ///
    /// Devuelve `None` si la llave no está declarada. **No puede devolver `None`
    /// por un `WriteMode` no cubierto**: la carga ya lo habría rechazado.
    pub fn resolve(&self, key: &str, write_mode: WriteMode) -> Option<&str> {
        self.map.get(key)?.get(&write_mode).map(String::as_str)
    }
}
