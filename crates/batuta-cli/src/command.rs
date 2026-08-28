//! La orden `canary`, de la línea de órdenes al recibo en disco.

use std::path::{Path, PathBuf};

use batuta_receipt::Receipt;

use crate::error::CliError;
use crate::paths::Layout;

/// Lo que deja un canario: el recibo, y dónde quedó.
#[derive(Debug)]
pub struct CanaryOutcome {
    /// El recibo sellado. **Puede ser rojo**: eso es un resultado, no un fallo.
    pub receipt: Receipt,
    /// Dónde se escribió.
    pub receipt_path: PathBuf,
}

/// Lanza el canario de un proveedor.
///
/// Los manifiestos se releen en cada invocación (R7): batuta no cachea una
/// política que alguien puede haber cambiado desde la última vez, porque un
/// proceso largo que se quedó con la copia vieja fue lo que hizo que un cambio
/// de manifiesto no surtiera efecto sin reiniciar.
///
/// # Errors
///
/// Cualquier [`CliError`]. Un canario **rojo no es un error**: sale por el
/// `Ok`, con su recibo y su motivo.
pub fn canary(
    provider: &str,
    model: Option<&str>,
    providers_dir: &Path,
    layout: &Layout,
    dsh_home: &Path,
) -> Result<CanaryOutcome, CliError> {
    // Fase roja: los parámetros son el contrato, no sobran.
    let _ = (provider, model, providers_dir, layout, dsh_home);
    todo!()
}
