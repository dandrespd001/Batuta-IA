//! `enable`, `disable`, `effort`: las tres órdenes que cambian la política.
//!
//! Cada una lee la política, la cambia, la guarda. Ninguna toca el manifiesto
//! ni los recibos: declaración y evidencia no son suyas para tocar (§1 de
//! `docs/FASE5_PANEL.md`).

use std::path::Path;

use batuta_contract::{ModelId, ReasoningEffort};
use batuta_manifest::{ModelEntry, ProviderManifest};
use batuta_policy::EleccionModelo;

use crate::error::CliError;
use crate::panel::cargar_politica;
use crate::paths::Layout;

/// Activa un modelo: `habilitado = true`, el esfuerzo que ya tuviera
/// declarado se conserva tal cual.
///
/// # Errors
///
/// Los de [`resolver`], más [`CliError::Policy`] si la política no se pudo
/// leer o guardar.
pub fn enable(providers_dir: &Path, layout: &Layout, model_ref: &str) -> Result<(), CliError> {
    let manifiestos = crate::command::cargar(providers_dir)?;
    let (_, modelo) = resolver(&manifiestos, model_ref)?;

    aplicar(layout, modelo.id().clone(), |anterior| EleccionModelo {
        habilitado: true,
        esfuerzo: anterior.and_then(|e| e.esfuerzo),
    })
}

/// Lo apaga: `habilitado = false`, el esfuerzo se conserva igual. No borra
/// nada: ni el manifiesto ni los recibos son suyos, y el esfuerzo que alguien
/// fijó sigue siendo la elección para cuando vuelva a activarse.
///
/// # Errors
///
/// Los mismos que [`enable`].
pub fn disable(providers_dir: &Path, layout: &Layout, model_ref: &str) -> Result<(), CliError> {
    let manifiestos = crate::command::cargar(providers_dir)?;
    let (_, modelo) = resolver(&manifiestos, model_ref)?;

    aplicar(layout, modelo.id().clone(), |anterior| EleccionModelo {
        habilitado: false,
        esfuerzo: anterior.and_then(|e| e.esfuerzo),
    })
}

/// Fija el nivel de esfuerzo. `habilitado` se conserva —fijar el esfuerzo no
/// activa el modelo por su cuenta— y por defecto es `false`, la misma
/// política de nacimiento que el resto de la política.
///
/// # Errors
///
/// Los de [`resolver`], más [`CliError::InvalidReasoningEffort`] si `nivel`
/// no es un token de `ReasoningEffort`, y [`CliError::EffortUnsupported`] si
/// el proveedor de ese modelo no declara ningún mapa de esfuerzo — no se
/// guarda un nivel que nunca se va a poder honrar.
pub fn effort(
    providers_dir: &Path,
    layout: &Layout,
    model_ref: &str,
    nivel: &str,
) -> Result<(), CliError> {
    let nivel: ReasoningEffort = nivel
        .parse()
        .map_err(|source| CliError::InvalidReasoningEffort { source })?;

    let manifiestos = crate::command::cargar(providers_dir)?;
    let (manifiesto, modelo) = resolver(&manifiestos, model_ref)?;

    if !manifiesto.substitutions().declares_reasoning_effort() {
        return Err(CliError::EffortUnsupported {
            provider: manifiesto.id().as_str().to_string(),
        });
    }

    aplicar(layout, modelo.id().clone(), |anterior| EleccionModelo {
        habilitado: anterior.is_some_and(|e| e.habilitado),
        esfuerzo: Some(nivel),
    })
}

/// Carga la política, aplica el cambio sobre la elección de un modelo ya
/// resuelto, y la guarda.
fn aplicar(
    layout: &Layout,
    id: ModelId,
    cambio: impl FnOnce(Option<&EleccionModelo>) -> EleccionModelo,
) -> Result<(), CliError> {
    let mut politica = cargar_politica(layout)?;
    let nueva = cambio(politica.eleccion(&id));
    politica.fijar(id, nueva);

    // Como `canary` con `leases()`/`receipts()`: quien escribe crea el
    // directorio, `guardar` no lo hace por su cuenta. La primera elección de
    // una instalación nueva no puede fallar por esto.
    std::fs::create_dir_all(layout.root()).map_err(|source| CliError::Io {
        path: layout.root().to_path_buf(),
        source,
    })?;

    politica
        .guardar(&layout.politica())
        .map_err(|source| CliError::Policy {
            source: Box::new(source),
        })
}

/// Encuentra el proveedor y el modelo que nombra `<proveedor>/<modelo>`.
///
/// # Errors
///
/// [`CliError::MalformedModelRef`] si no lleva barra,
/// [`CliError::UnknownProvider`] o [`CliError::UnknownModel`] si no existen —
/// los dos enumeran lo que sí hay (R8), igual que `canary`.
pub(crate) fn resolver<'a>(
    manifiestos: &'a [ProviderManifest],
    model_ref: &str,
) -> Result<(&'a ProviderManifest, &'a ModelEntry), CliError> {
    let Some((provider, model)) = model_ref.split_once('/') else {
        return Err(CliError::MalformedModelRef {
            given: model_ref.to_string(),
        });
    };

    let manifiesto = crate::command::hallar(manifiestos, provider)?;
    let modelo = manifiesto
        .models()
        .iter()
        .find(|m| m.id().as_str() == model)
        .ok_or_else(|| {
            let mut available: Vec<String> = manifiesto
                .models()
                .iter()
                .map(|m| m.id().as_str().to_string())
                .collect();
            available.sort();
            CliError::UnknownModel {
                asked: model.to_string(),
                provider: provider.to_string(),
                available,
            }
        })?;

    Ok((manifiesto, modelo))
}
