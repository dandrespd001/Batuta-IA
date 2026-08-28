//! Ejecutar el proceso del proveedor, y **poseer su límite**.
//!
//! R6 en una frase: *el proceso es el límite; matar la tarea mata el árbol y
//! libera el lease*. El fallo que la paga: `TaskStop` dejaba el hijo vivo
//! gastando cuota, y su lease de repositorio bloqueando a cualquier otro modelo.
//!
//! La mitad difícil no necesita dependencias: `CommandExt::process_group(0)` es
//! biblioteca estándar y lanza al hijo como líder de su propio grupo. Comprobado
//! con sonda antes de escribir una línea: matar el grupo dejó cero nietos. Sólo
//! `killpg` viene de fuera.

use std::path::{Path, PathBuf};
use std::time::Duration;

use batuta_manifest::EnvPolicy;

use crate::error::ExecError;

/// Lo que la corrida produjo. **Hechos, no juicio.**
///
/// Va entero al recibo, que es quien concluye. `exit_code` es `Option` porque
/// `None` —lo mató una señal— y `Some(1)` son cosas distintas, y el diagnóstico
/// tiene que poder distinguirlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// El `argv` real con el que se lanzó, ya sustituido.
    pub argv: Vec<String>,
    /// Los **nombres** de las variables que se pasaron. Nunca los valores (R10).
    pub env_names: Vec<String>,
    /// Código de salida; `None` si murió por señal.
    pub exit_code: Option<i32>,
    /// Salida estándar completa.
    pub stdout: String,
    /// Error estándar **íntegro**, aunque el proceso saliera con cero.
    pub stderr: String,
    /// Cuánto duró de pared.
    pub duration: Duration,
    /// Si se agotó el límite y hubo que matar el grupo.
    pub timed_out: bool,
}

/// Construye el entorno del hijo **desde cero** (R5).
///
/// Nada se hereda sin nombrarlo: se parte de vacío, se copian sólo las variables
/// de `allow` que existan en el entorno actual, se retiran las de `deny` —que
/// gana siempre, porque hay variables que el proveedor lee para decidir su propia
/// contención— y se aplican las de `set`.
///
/// El fallo que lo paga: `--approval-mode auto` dio `auto_accept` 1 y 0 en dos
/// canarios seguidos para la misma clase de llamada. Contención determinista
/// significa que dos corridas iguales ven lo mismo.
pub fn build_env(_policy: &EnvPolicy) -> Vec<(String, String)> {
    todo!()
}

/// Lanza el proceso, espera su límite, y si vence mata **el grupo entero**.
///
/// # Errors
///
/// [`ExecError::Spawn`] si el programa no se pudo lanzar. Que el proceso salga
/// con error **no** es un error de esta función: es un hecho que va al recibo.
pub fn run(
    _program: &Path,
    _argv: &[String],
    _env: &[(String, String)],
    _cwd: &Path,
    _timeout: Duration,
) -> Result<RunOutcome, ExecError> {
    todo!()
}

/// Resuelve el programa contra las rutas de `resolve` del manifiesto.
///
/// Existe aparte de `ProviderManifest::verify_executable` porque aquí se admite
/// una raíz distinta para las pruebas: los fixtures apuntan a `/bin/echo` y a
/// `/bin/sleep`, que no tienen `sha256` fijado ni falta que hace.
pub fn resolve_program(_candidates: &[String]) -> Option<PathBuf> {
    todo!()
}
