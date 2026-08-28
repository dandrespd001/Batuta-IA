//! El canario: la corrida más pequeña que demuestra que un proveedor responde.
//!
//! **Observacional, nunca por subcadena sobre juicio propio.** R3 se paga aquí:
//! `provider-canary` devolvió `QUOTA_UNAVAILABLE` en 126 ms sin tocar la red,
//! porque leyó el `status` del mismo fichero que él debía informar. Aquí se
//! genera un token irrepetible, se pide que lo devuelva, y se comprueba que
//! **volvió ése**.
//!
//! Es también donde las cinco piezas se juntan por primera vez: manifiesto,
//! admisión, sustitución, ejecución y recibo.

use std::path::PathBuf;
use std::time::Duration;

use batuta_manifest::{ModelEntry, ProviderManifest};
use batuta_receipt::Receipt;

use crate::error::ExecError;

/// Lo que hace falta para lanzar un canario.
#[derive(Debug, Clone)]
pub struct CanaryRequest {
    /// Dónde trabaja el proceso. Para un canario basta un directorio temporal:
    /// es de sólo lectura y no hay diff que calcular.
    pub workdir: PathBuf,
    /// Dónde se materializan los ficheros de corrida. **Fuera del workdir.**
    pub run_dir: PathBuf,
    /// Dónde viven los leases.
    pub state_dir: PathBuf,
    /// La raíz del proveedor donde buscar su registro de sesión.
    pub dsh_home: PathBuf,
    /// Límite de pared.
    pub timeout: Duration,
    /// Identificador del encargo, para que el lease ocupado diga quién lo tiene.
    pub task_id: String,
}

/// Un token irrepetible para el canario.
///
/// De `/dev/urandom`, sin dependencia nueva. **Un token predecible dejaría de ser
/// observacional**: si se pudiera adivinar, un proveedor que devolviera texto
/// plausible sin llamar a nadie pasaría el canario, que es exactamente el fallo
/// de la puerta circular.
///
/// # Errors
///
/// Si `/dev/urandom` no se puede leer.
pub fn generate_token() -> std::io::Result<String> {
    todo!("16 bytes en hexadecimal, con un prefijo legible")
}

/// Ejecuta el canario entero y devuelve su recibo.
///
/// Toma los dos leases —por modelo y por repositorio— **antes** de arrancar y los
/// suelta al terminar. Es la otra mitad de R6: matar la tarea mata el árbol *y
/// libera el lease*, y el fallo que la paga dejaba un lease de repositorio
/// bloqueando a cualquier otro modelo.
///
/// El recibo sale sellado, verde o rojo. **Que salga rojo no es un error de esta
/// función**: es su respuesta.
///
/// # Errors
///
/// [`ExecError::Admission`] si otro encargo tiene los leases, y los de
/// sustitución, materialización o lanzamiento. Nada de eso es un veredicto: son
/// las cosas que impiden llegar a tener uno.
pub fn run_canary(
    manifest: &ProviderManifest,
    model: &ModelEntry,
    request: &CanaryRequest,
) -> Result<Receipt, ExecError> {
    // Fase roja: los parámetros son el contrato, no sobran.
    let _ = (manifest, model, request);
    todo!()
}
