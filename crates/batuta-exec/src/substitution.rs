//! Rellenar las llaves `{...}` del manifiesto con los hechos del encargo.
//!
//! Puro: se prueba entero sin disco ni procesos, y por eso va primero. Lo que
//! sale de aquí es el `argv` **real** que acabará en el recibo, no el del
//! manifiesto — entre uno y otro hay sustituciones, y el que sirve para
//! reproducir una corrida es el de después.

use std::path::PathBuf;

use batuta_contract::{ModelId, RouteModel, WriteMode};
use batuta_manifest::ProviderManifest;

use crate::error::ExecError;

/// Los hechos del encargo que rellenan las llaves incorporadas.
#[derive(Debug, Clone)]
pub struct RunContext {
    /// Identificador del modelo dentro de batuta.
    pub model: ModelId,
    /// El nombre que el proveedor entiende.
    pub route_model: RouteModel,
    /// La ruta del proveedor, cuando distingue ruta de modelo.
    pub route_provider: Option<String>,
    /// El árbol donde trabaja el encargo.
    pub workdir: PathBuf,
    /// Dónde se materializan los ficheros de corrida. **Fuera del worktree.**
    pub run_dir: PathBuf,
    /// El encargo, ya redactado.
    pub prompt: String,
    /// El token irrepetible del canario.
    pub token: String,
    /// Lo que el encargo puede hacer, que decide las sustituciones declaradas.
    pub write_mode: WriteMode,
}

/// Sustituye las llaves de una plantilla.
///
/// # Errors
///
/// [`ExecError::UnknownPlaceholder`] si aparece una llave que no es incorporada
/// ni está declarada en `[substitutions]`. Nunca se deja una llave sin sustituir
/// ni se sustituye por vacío: una llave que sobrevive acabaría en el `argv` de un
/// proceso real, y una sustituida por vacío desaparece sin que nadie lo note.
pub fn resolve(
    _template: &str,
    _field: &str,
    _manifest: &ProviderManifest,
    _context: &RunContext,
) -> Result<String, ExecError> {
    todo!()
}

/// Sustituye el `argv` entero.
///
/// # Errors
///
/// Lo mismo que [`resolve`], nombrando `invoke.argv[<i>]` como campo.
pub fn resolve_argv(
    _manifest: &ProviderManifest,
    _context: &RunContext,
) -> Result<Vec<String>, ExecError> {
    todo!()
}
