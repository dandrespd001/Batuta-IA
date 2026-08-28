//! `ReceiptStore`: el recibo verde más reciente de un modelo, si sigue vigente.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use batuta_receipt::Receipt;

use crate::error::StoreError;

/// Cuánto se confía en un recibo verde sin volver a canariar.
///
/// **Por qué 24 horas y no otra cosa**: no hay una medición que fije este
/// número —a diferencia de `{reasoning_effort}` en T1, aquí no hay un
/// validador de proveedor que lo diga—. Es una elección entre dos costes: un
/// TTL corto obliga a repetir canarios que cuestan cuota y tiempo; uno largo
/// deja que un proveedor roto siga pareciendo enrutable. Un día es lo
/// bastante corto para no confiar en una semana vieja y lo bastante largo
/// para no recanariar en cada consulta del panel. **Se declara aquí, se
/// exporta, y se puede pasar uno distinto**: no es una constante enterrada en
/// la lógica de `latest_green`.
pub const DEFAULT_TTL: Duration = Duration::from_hours(24);

/// Un recibo que estaba ahí y no se pudo leer.
///
/// Existe para que un fichero roto nunca se confunda con la ausencia de
/// evidencia: las dos cosas piden reacciones distintas de quien lee el panel.
#[derive(Debug)]
pub struct Unreadable {
    /// El fichero que no se pudo interpretar.
    pub path: PathBuf,
    /// Qué falló, en prosa.
    pub reason: String,
}

/// El estado de la evidencia para un modelo.
#[derive(Debug)]
pub enum LatestGreen {
    /// Hay un recibo verde, del `manifest_sha256` actual, dentro del TTL.
    ///
    /// En caja: `Receipt` lleva el `argv`, el `stdout`/`stderr` íntegros y
    /// cada fichero materializado, así que es, con mucho, la variante más
    /// grande — sin caja, cada `LatestGreen::Absent` pagaría ese tamaño.
    Fresh(Box<Receipt>),
    /// Hubo un recibo verde y vigente en `manifest_sha256`, pero ya caducó.
    /// `at` es el instante en que dejó de ser fresco (`mtime` del recibo, más
    /// el TTL con el que se consultó) — la respuesta directa a «¿cuándo
    /// caducó?», no un mero «hace tiempo».
    Expired {
        /// Cuándo caducó.
        at: SystemTime,
    },
    /// No hay ningún recibo verde del `manifest_sha256` actual para este
    /// modelo. Puede ser porque nunca se canarió, porque el manifiesto
    /// cambió desde el último canario, o porque el último salió rojo:
    /// `latest_green` no distingue esos tres casos, porque de cara a
    /// enrutar los tres significan lo mismo, «sin evidencia utilizable».
    Absent,
}

/// El resultado de consultar el almacén: el estado, y lo que no se pudo leer
/// en el camino.
#[derive(Debug)]
pub struct Lookup {
    /// El estado de la evidencia.
    pub result: LatestGreen,
    /// Recibos que estaban en el directorio y no se pudieron interpretar.
    /// Vacío no significa «no había ninguno roto»: significa exactamente eso.
    pub unreadable: Vec<Unreadable>,
}

/// El directorio de recibos, consultable.
///
/// No toma ningún cerrojo (R9): los recibos son ficheros inmutables, uno por
/// corrida —`batuta-cli` nunca reescribe uno—, así que no hay nada que
/// bloquee una lectura mientras otra corrida escribe la suya.
pub struct ReceiptStore {
    root: PathBuf,
}

impl ReceiptStore {
    /// Abre el almacén sobre un directorio. No falla si el directorio no
    /// existe todavía: una consulta sobre él simplemente no encuentra nada.
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// El recibo verde más reciente para `model_id`, cuyo `manifest_sha256`
    /// coincida con el que se pasa y que siga dentro de `ttl`.
    ///
    /// # Errors
    ///
    /// [`StoreError::Read`] si el propio directorio no se pudo listar —no si
    /// un recibo suelto no se pudo leer: eso va a
    /// [`Lookup::unreadable`](Lookup::unreadable).
    pub fn latest_green(
        &self,
        model_id: &str,
        manifest_sha256: &str,
        ttl: Duration,
    ) -> Result<Lookup, StoreError> {
        let mut mejor: Option<(SystemTime, Receipt)> = None;
        let mut unreadable = Vec::new();

        let entradas = match std::fs::read_dir(&self.root) {
            Ok(entradas) => entradas,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Lookup {
                    result: LatestGreen::Absent,
                    unreadable,
                });
            }
            Err(source) => {
                return Err(StoreError::Read {
                    path: self.root.clone(),
                    source,
                });
            }
        };

        for entrada in entradas {
            let entrada = entrada.map_err(|source| StoreError::Read {
                path: self.root.clone(),
                source,
            })?;
            let ruta = entrada.path();
            if ruta.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
                continue;
            }

            match leer_candidato(&ruta) {
                Ok((receipt, mtime)) => {
                    if receipt.model_requested() != model_id
                        || receipt.manifest_sha256() != manifest_sha256
                        || !receipt.verdict().is_green()
                    {
                        continue;
                    }
                    let es_mas_nuevo = mejor
                        .as_ref()
                        .is_none_or(|(mtime_actual, _)| mtime > *mtime_actual);
                    if es_mas_nuevo {
                        mejor = Some((mtime, receipt));
                    }
                }
                Err(reason) => unreadable.push(Unreadable { path: ruta, reason }),
            }
        }

        let result = match mejor {
            None => LatestGreen::Absent,
            Some((mtime, receipt)) => {
                let expira = mtime + ttl;
                if SystemTime::now() >= expira {
                    LatestGreen::Expired { at: expira }
                } else {
                    LatestGreen::Fresh(Box::new(receipt))
                }
            }
        };

        Ok(Lookup { result, unreadable })
    }
}

/// Lee un recibo y la fecha de modificación de su fichero.
///
/// Devuelve el `mtime` en vez de un campo del propio JSON porque el recibo no
/// lleva ninguno: `batuta-cli` escribe cada recibo una única vez, así que el
/// `mtime` del fichero **es** el instante en que se selló.
fn leer_candidato(path: &Path) -> Result<(Receipt, SystemTime), String> {
    let metadata = std::fs::metadata(path).map_err(|source| source.to_string())?;
    let mtime = metadata.modified().map_err(|source| source.to_string())?;
    let texto = std::fs::read_to_string(path).map_err(|source| source.to_string())?;
    let receipt: Receipt = serde_json::from_str(&texto).map_err(|source| source.to_string())?;
    Ok((receipt, mtime))
}
