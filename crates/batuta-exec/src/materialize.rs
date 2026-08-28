// generado: deepseek-v4-flash - revisado: Arquitecto
//! Escribir los ficheros de configuración que la corrida necesita.
//!
//! Nacen de una medición: en dsh el modelo **no viaja en `argv`**. Va en un
//! documento de settings que gana a la capa de composición, y sin escribirlo no
//! hay forma de fijar qué modelo corre. Se probó con `--patch` solo: el árbol de
//! composición cambiaba y la corrida seguía yendo a otro modelo.

use std::path::{Path, PathBuf};

use batuta_manifest::{ProviderManifest, RuntimeDocument, RuntimeFile};
use batuta_receipt::MaterializedFile;

use crate::error::ExecError;
use crate::substitution::{RunContext, resolve};

/// Escribe los `[[runtime_files]]` del manifiesto en el directorio de corrida.
///
/// Devuelve lo escrito **con su contenido**, porque el recibo lo lleva: sin eso
/// no se puede reproducir una corrida ni explicar por qué corrió lo que corrió.
///
/// # Errors
///
/// [`ExecError::RuntimeFileInsideWorktree`] si alguno cayera dentro del árbol del
/// encargo —se comprueba **antes** de escribir nada—, y `Materialize` si el disco
/// no coopera.
pub fn materialize(
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<Vec<MaterializedFile>, ExecError> {
    // Primero las comprobaciones y la serialización, después el disco: ni la
    // invasión del worktree ni una llave desconocida dejan rastro.
    let destinos: Vec<PathBuf> = manifest
        .runtime_files()
        .iter()
        .map(|fichero| context.run_dir.join(fichero.path()))
        .collect();

    for destino in &destinos {
        if cae_dentro(destino, &context.workdir) {
            return Err(ExecError::RuntimeFileInsideWorktree {
                path: destino.clone(),
                worktree: context.workdir.clone(),
            });
        }
    }

    let preparados: Vec<(PathBuf, String)> = manifest
        .runtime_files()
        .iter()
        .zip(&destinos)
        .map(|(fichero, destino)| {
            serializar(fichero, destino, manifest, context)
                .map(|contenido| (destino.clone(), contenido))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut escritos = Vec::with_capacity(preparados.len());
    for (destino, contenido) in preparados {
        if let Some(progenitor) = destino.parent() {
            std::fs::create_dir_all(progenitor).map_err(|source| ExecError::Materialize {
                path: destino.clone(),
                source,
            })?;
        }
        std::fs::write(&destino, &contenido).map_err(|source| ExecError::Materialize {
            path: destino.clone(),
            source,
        })?;
        escritos.push(MaterializedFile::new(destino, contenido));
    }
    Ok(escritos)
}

/// El contenido de un fichero de corrida: documento con las llaves sustituidas
/// y serializado como JSON.
///
/// El manifiesto declara `format = "yaml"`, pero batuta no tiene serializador
/// YAML: JSON **es** YAML válido, y está medido contra dsh 0.1.1-rc.2 (se le
/// pasó un `.yml` con contenido JSON y `--dump-config` aplicó el parche
/// correctamente).
fn serializar(
    fichero: &RuntimeFile,
    destino: &Path,
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<String, ExecError> {
    let campo = fichero.path().to_string_lossy().into_owned();
    let json = match fichero.document() {
        RuntimeDocument::List(entradas) => serde_json::to_value(entradas),
        RuntimeDocument::Map(tabla) => serde_json::to_value(tabla),
    }
    .map_err(|source| ExecError::Materialize {
        path: destino.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
    })?;

    let mut json = json;
    sustituir_en_json(&mut json, &campo, manifest, context)?;

    serde_json::to_string_pretty(&json).map_err(|source| ExecError::Materialize {
        path: destino.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
    })
}

/// Recorre el documento entero, no sólo su primer nivel: las llaves viven en
/// cualquier profundidad de las listas y los mapas de un documento de corrida.
fn sustituir_en_json(
    json: &mut serde_json::Value,
    campo: &str,
    manifest: &ProviderManifest,
    context: &RunContext,
) -> Result<(), ExecError> {
    match json {
        serde_json::Value::String(texto) => {
            *texto = resolve(texto, campo, manifest, context)?;
        }
        serde_json::Value::Array(entradas) => {
            for entrada in entradas {
                sustituir_en_json(entrada, campo, manifest, context)?;
            }
        }
        serde_json::Value::Object(mapa) => {
            for valor in mapa.values_mut() {
                sustituir_en_json(valor, campo, manifest, context)?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
    Ok(())
}

/// ¿Cae `candidata` dentro de `worktree`?
///
/// Compara por componentes tras normalizar, no por prefijo de cadena: `/tmp/ar`
/// no está dentro de `/tmp/arbol` aunque su ruta empiece igual. Es el mismo error
/// que `cubre()` evita en la allowlist del `TaskSpec`, donde exige frontera de
/// `/` para que `addons` no cuente como padre de `addons_extra`.
pub fn cae_dentro(candidata: &Path, worktree: &Path) -> bool {
    normalizar(candidata).starts_with(&normalizar(worktree))
}

/// Componentes de una ruta con `.` eliminados y `..` resueltos contra el
/// componente anterior: `/tmp/arbol/../arbol/corrida` cae dentro de
/// `/tmp/arbol`, y un mismo directorio cae dentro de sí mismo.
fn normalizar(ruta: &Path) -> Vec<PathBuf> {
    let mut componentes = Vec::new();
    for componente in ruta.components() {
        match componente {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                componentes.pop();
            }
            otro => componentes.push(PathBuf::from(otro.as_os_str())),
        }
    }
    componentes
}
