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
//! Toda sustitución *declarada libremente* deriva de `write_mode`, y por eso el
//! tipo dice `WriteMode` en vez de generalizar a «un vocabulario cualquiera».
//! `reasoning_effort` es la primera que deriva de otro (T1 de
//! `docs/FASE5_PANEL.md`): un nombre **reservado**, con su propio mapa keyed por
//! `ReasoningEffort`, porque dsh y abacus no toman el esfuerzo de razonamiento
//! por el mismo canal que el modo de escritura. Cuando aparezca una tercera, el
//! tipo vuelve a crecer. Antes no (R2).

use std::collections::BTreeMap;

use batuta_contract::{ReasoningEffort, WriteMode};

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
    /// `[substitutions.reasoning_effort]`, reservado y keyed por
    /// `ReasoningEffort` en vez de `WriteMode`. `None` cuando el proveedor no lo
    /// declara — dsh sí, abacus no tiene con qué (medido: `abacusai --help` de
    /// la 2.6.11 no ofrece ninguna bandera de esfuerzo).
    reasoning_effort: Option<BTreeMap<ReasoningEffort, String>>,
}

impl Substitutions {
    /// Construye el mapa ya validado. Sólo lo usa la carga.
    pub(crate) fn new(
        map: BTreeMap<String, BTreeMap<WriteMode, String>>,
        reasoning_effort: Option<BTreeMap<ReasoningEffort, String>>,
    ) -> Self {
        Self {
            map,
            reasoning_effort,
        }
    }

    /// Las llaves declaradas, en orden alfabético.
    ///
    /// `reasoning_effort` no aparece aquí: no vive en el mapa genérico, vive en
    /// su propio campo. Para saber si un manifiesto lo declara, mira
    /// [`Self::declares_reasoning_effort`].
    pub fn declared_keys(&self) -> Vec<&str> {
        self.map.keys().map(String::as_str).collect()
    }

    /// Si el manifiesto declara `[substitutions.reasoning_effort]`.
    pub fn declares_reasoning_effort(&self) -> bool {
        self.reasoning_effort.is_some()
    }

    /// Todas las llaves admitidas: incorporadas y declaradas.
    ///
    /// Es la lista que sale en el error de R8 cuando alguien escribe una que no
    /// existe. `reasoning_effort` sólo entra si el manifiesto trae su mapa: sin
    /// él, usar `{reasoning_effort}` falla al cargar (R1) en vez de fallar en
    /// una corrida real.
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
        if self.reasoning_effort.is_some() {
            out.push("reasoning_effort".to_string());
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

    /// El nombre que el proveedor entiende para un nivel de esfuerzo.
    ///
    /// Devuelve `None` si el proveedor no declara el mapa: es la única forma en
    /// que puede faltar, porque un mapa declarado cubre `ReasoningEffort::ALL`
    /// entero o la carga lo habría rechazado.
    pub fn resolve_reasoning_effort(&self, effort: ReasoningEffort) -> Option<&str> {
        self.reasoning_effort
            .as_ref()?
            .get(&effort)
            .map(String::as_str)
    }
}
