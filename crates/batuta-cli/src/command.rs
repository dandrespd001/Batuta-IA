//! La orden `canary`, de la línea de órdenes al recibo en disco.
// generado: deepseek-v4-flash - revisado: Arquitecto

use std::path::{Path, PathBuf};

use batuta_manifest::{ModelEntry, ProviderManifest};
use batuta_receipt::Receipt;

use crate::error::CliError;
use crate::paths::Layout;

/// Dos minutos de pared.
///
/// Un canario contra un proveedor real tarda decenas de segundos —arranque del
/// CLI, autenticación, primera respuesta—, así que 120 s es holgado sin ser
/// eterno, y es el mismo límite que el sistema viejo usaba antes de que las
/// colas de admisión lo atasquen.
const TIMEOUT_CANARIO: std::time::Duration = std::time::Duration::from_secs(120);

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
    let manifiestos = cargar(providers_dir)?;
    let manifiesto = hallar(&manifiestos, provider)?;
    let modelo = elegir_modelo(manifiesto, provider, model)?;

    ejecutar(manifiesto, modelo, provider, layout, dsh_home)
}

/// Lanza el canario de **todos** los modelos de un proveedor, uno tras otro.
///
/// Es la respuesta concreta a que añadir o quitar un modelo sea sencillo: añadir
/// uno son cinco líneas de manifiesto **más un canario que pase** —R2: un modelo
/// sin recibo verde no es enrutable— y ésta es la orden que lo pasa.
///
/// **Un modelo rojo no detiene a los demás.** Un rojo es el resultado de ese
/// modelo, no un fallo del lote; parar en el primero dejaría sin medir a los que
/// van detrás, y el lote existe justamente para saber cuáles valen.
///
/// # Errors
///
/// Sólo lo que impide llegar a tener veredictos: el proveedor no existe, los
/// manifiestos no cargan, el disco no coopera. Un `Err` de una corrida concreta
/// **sí** corta el lote, porque significa que la máquina no está en condiciones
/// —no hay ejecutable, no se pudo admitir— y las siguientes fallarían igual.
pub fn canary_all(
    provider: &str,
    providers_dir: &Path,
    layout: &Layout,
    dsh_home: &Path,
) -> Result<Vec<CanaryOutcome>, CliError> {
    let manifiestos = cargar(providers_dir)?;
    let manifiesto = hallar(&manifiestos, provider)?;

    manifiesto
        .models()
        .iter()
        .map(|modelo| ejecutar(manifiesto, modelo, provider, layout, dsh_home))
        .collect()
}

/// Los manifiestos del directorio, releídos.
///
/// R7: nunca se cachean. Un directorio con un manifiesto roto falla aquí, antes
/// de tocar nada (R1).
pub(crate) fn cargar(providers_dir: &Path) -> Result<Vec<ProviderManifest>, CliError> {
    ProviderManifest::load_dir(providers_dir).map_err(|e| CliError::Manifest {
        source: Box::new(e),
    })
}

/// El manifiesto de un proveedor, o un error que **enumera los que sí hay** (R8).
pub(crate) fn hallar<'a>(
    manifiestos: &'a [ProviderManifest],
    provider: &str,
) -> Result<&'a ProviderManifest, CliError> {
    manifiestos
        .iter()
        .find(|m| m.id().as_str() == provider)
        .ok_or_else(|| {
            let mut available: Vec<String> = manifiestos
                .iter()
                .map(|m| m.id().as_str().to_string())
                .collect();
            available.sort();
            CliError::UnknownProvider {
                asked: provider.to_string(),
                available,
            }
        })
}

/// Una corrida concreta: directorios, canario y recibo en disco.
fn ejecutar(
    manifiesto: &ProviderManifest,
    modelo: &ModelEntry,
    provider: &str,
    layout: &Layout,
    dsh_home: &Path,
) -> Result<CanaryOutcome, CliError> {
    let nombre = nombre_corrida(provider, modelo.id().as_str());
    let corrida = layout.runs().join(&nombre);
    let workdir = corrida.join("arbol");
    let run_dir = corrida.join("corrida");

    std::fs::create_dir_all(&workdir).map_err(|source| CliError::Io {
        path: workdir.clone(),
        source,
    })?;
    std::fs::create_dir_all(&run_dir).map_err(|source| CliError::Io {
        path: run_dir.clone(),
        source,
    })?;
    std::fs::create_dir_all(layout.receipts()).map_err(|source| CliError::Io {
        path: layout.receipts(),
        source,
    })?;
    std::fs::create_dir_all(layout.leases()).map_err(|source| CliError::Io {
        path: layout.leases(),
        source,
    })?;

    let peticion = batuta_exec::CanaryRequest {
        workdir,
        run_dir,
        state_dir: layout.leases(),
        dsh_home: dsh_home.to_path_buf(),
        timeout: TIMEOUT_CANARIO,
        task_id: nombre.clone(),
    };
    let recibo =
        batuta_exec::run_canary(manifiesto, modelo, &peticion).map_err(|e| CliError::Exec {
            source: Box::new(e),
        })?;

    let receipt_path = layout.receipts().join(format!("{nombre}.json"));
    let json = recibo.to_json().map_err(|source| CliError::Io {
        path: receipt_path.clone(),
        source: std::io::Error::other(source),
    })?;
    // Con salto de línea final: un fichero de texto sin el último salto confunde
    // a todas las herramientas de línea.
    std::fs::write(&receipt_path, format!("{json}\n")).map_err(|source| CliError::Io {
        path: receipt_path.clone(),
        source,
    })?;

    Ok(CanaryOutcome {
        receipt: recibo,
        receipt_path,
    })
}

/// El modelo de una corrida: el pedido, si vino; si no, el único declarado.
///
/// Con varios modelos y ninguno pedido **no se elige en silencio**: se para y se
/// enumeran (R8). El caso sin modelos no puede llegar aquí —la carga del
/// manifiesto lo rechaza— y si llegara cae en la misma variante con la lista
/// vacía.
fn elegir_modelo<'a>(
    manifiesto: &'a ProviderManifest,
    provider: &str,
    pedido: Option<&str>,
) -> Result<&'a ModelEntry, CliError> {
    let modelos = manifiesto.models();
    match pedido {
        Some(pedido) => {
            let Some(modelo) = modelos.iter().find(|m| m.id().as_str() == pedido) else {
                let mut available: Vec<String> = modelos
                    .iter()
                    .map(|m| m.id().as_str().to_string())
                    .collect();
                available.sort();
                return Err(CliError::UnknownModel {
                    asked: pedido.to_string(),
                    provider: provider.to_string(),
                    available,
                });
            };
            Ok(modelo)
        }
        None => match modelos {
            [unico] => Ok(unico),
            varios => {
                let mut available: Vec<String> =
                    varios.iter().map(|m| m.id().as_str().to_string()).collect();
                available.sort();
                Err(CliError::AmbiguousModel {
                    provider: provider.to_string(),
                    available,
                })
            }
        },
    }
}

/// Un nombre único por corrida: `<proveedor>-<modelo>-<milisegundos>`.
///
/// Los milisegundos desde la época separan dos corridas del mismo modelo. Si el
/// reloj dice que estamos antes de 1970 —un reloj de muro desajustado— se usa
/// `0` y se sigue: un nombre feo no justifica abortar un canario.
fn nombre_corrida(provider: &str, modelo: &str) -> String {
    let milisegundos = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duracion) => duracion.as_millis(),
        Err(_) => 0,
    };
    sanear(&format!("{provider}-{modelo}-{milisegundos}"))
}

/// Sustituye por `-` cualquier carácter que no sea `[A-Za-z0-9._-]`.
///
/// El id de modelo admite `/` y batuta lo aplana a `-` para que la corrida quepa
/// en un nombre de directorio.
fn sanear(nombre: &str) -> String {
    nombre
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}
