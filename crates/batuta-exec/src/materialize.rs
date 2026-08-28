//! Escribir los ficheros de configuración que la corrida necesita.
//!
//! Nacen de una medición: en dsh el modelo **no viaja en `argv`**. Va en un
//! documento de settings que gana a la capa de composición, y sin escribirlo no
//! hay forma de fijar qué modelo corre. Se probó con `--patch` solo: el árbol de
//! composición cambiaba y la corrida seguía yendo a otro modelo.

use std::path::Path;

use batuta_manifest::ProviderManifest;
use batuta_receipt::MaterializedFile;

use crate::error::ExecError;
use crate::substitution::RunContext;

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
    _manifest: &ProviderManifest,
    _context: &RunContext,
) -> Result<Vec<MaterializedFile>, ExecError> {
    todo!()
}

/// ¿Cae `candidata` dentro de `worktree`?
///
/// Compara por componentes tras normalizar, no por prefijo de cadena: `/tmp/ar`
/// no está dentro de `/tmp/arbol` aunque su ruta empiece igual. Es el mismo error
/// que `cubre()` evita en la allowlist del `TaskSpec`, donde exige frontera de
/// `/` para que `addons` no cuente como padre de `addons_extra`.
pub fn cae_dentro(_candidata: &Path, _worktree: &Path) -> bool {
    todo!()
}
