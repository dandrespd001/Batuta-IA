// generado: deepseek-v4-flash - revisado: Arquitecto
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

/// El valor de una llave, incorporada o declarada.
///
/// La llave que llega aquí ya es admitida: la carga del manifiesto lo habría
/// rechazado de no serlo. `None` de `Substitutions::resolve` no puede darse
/// tras la carga —el mapa cubre el vocabulario entero— y por eso significa
/// exactamente lo mismo que una llave desconocida.
fn valor_para(
    clave: &str,
    field: &str,
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<String, ExecError> {
    match clave {
        "model" => Ok(context.model.as_str().to_string()),
        "route_model" => Ok(context.route_model.as_str().to_string()),
        // `route_provider` es la única incorporada opcional: si el manifiesto la
        // usa y el encargo no trae ruta, se sustituye por vacío (ver desviación 1).
        "route_provider" => Ok(context.route_provider.clone().unwrap_or_default()),
        "workdir" => Ok(context.workdir.to_string_lossy().into_owned()),
        "run_dir" => Ok(context.run_dir.to_string_lossy().into_owned()),
        "prompt" => Ok(context.prompt.clone()),
        "token" => Ok(context.token.clone()),
        otra => match manifest.substitutions().resolve(otra, context.write_mode) {
            Some(valor) => Ok(valor.to_string()),
            None => Err(ExecError::UnknownPlaceholder {
                field: field.to_string(),
                placeholder: otra.to_string(),
                expected: manifest.substitutions().allowed_placeholders(),
            }),
        },
    }
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
    template: &str,
    field: &str,
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<String, ExecError> {
    let mut salida = String::with_capacity(template.len());
    let mut resto = template;
    while let Some(inicio) = resto.find('{') {
        salida.push_str(&resto[..inicio]);
        let despues = &resto[inicio + 1..];
        let Some(final_llave) = despues.find('}') else {
            // Una llave sin cierre no termina la búsqueda, pero tampoco es una
            // llave: se queda literal.
            salida.push_str(&resto[inicio..]);
            return Ok(salida);
        };
        let clave = &despues[..final_llave];
        if clave.is_empty() {
            // `{}` no es una llave: la carga la ignora y aquí se queda literal.
            salida.push_str("{}");
        } else {
            salida.push_str(&valor_para(clave, field, manifest, context)?);
        }
        resto = &despues[final_llave + 1..];
    }
    salida.push_str(resto);
    Ok(salida)
}

/// Sustituye el `argv` entero.
///
/// # Errors
///
/// Lo mismo que [`resolve`], nombrando `invoke.argv[<i>]` como campo.
pub fn resolve_argv(
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<Vec<String>, ExecError> {
    manifest
        .invoke()
        .argv()
        .iter()
        .enumerate()
        .map(|(indice, argumento)| {
            resolve(
                argumento,
                &format!("invoke.argv[{indice}]"),
                manifest,
                context,
            )
        })
        .collect()
}
