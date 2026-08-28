// generado: deepseek-v4-flash - revisado: Arquitecto
//! Leer del registro de la máquina qué ocurrió de verdad.
//!
//! **La regla que ordena el módulo: se anota lo observado, no lo pedido.** Se
//! pidió `deepseek-v4-flash` tres veces y corrió otro modelo las tres, porque el
//! modelo lo decidía un fichero que batuta no controlaba. Un recibo que hubiera
//! anotado la petición habría mentido sobre lo único que le da valor.
//!
//! Y la segunda, que es su consecuencia incómoda: **una procedencia que no se
//! puede leer es recibo en rojo**, jamás un hueco que se rellena con lo pedido.
//! «No pude leerlo» y «no pasó nada» son cosas distintas.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use batuta_receipt::ObservedProvenance;

/// El nombre de directorio que dsh deriva de un `cwd`.
///
/// Reimplementado, no adivinado: separadores a `-`, los caracteres seguros
/// (`[A-Za-z0-9._-]`) literales, el resto escapado como `~XXXX` con el valor
/// hexadecimal de la unidad de código, truncado a 251 y envuelto en `--..--`.
///
/// Hay dos valores medidos contra el disco que lo fijan, y uno de ellos lleva un
/// acento precisamente porque el escape es la parte que más fácil se equivoca.
///
/// **La normalización es deliberadamente lossy** —lo dice la documentación de
/// dsh—, así que dos rutas largas pueden compartir directorio. De ahí sale la
/// regla de que los worktrees de batuta tengan rutas cortas y distinguibles cerca
/// del final.
pub fn project_key(cwd: &Path) -> String {
    let mut clave = String::new();
    let mut buffer = [0u16; 2];
    for ch in cwd.to_string_lossy().chars() {
        match ch {
            // Separadores de ruta, los tres: se vuelven el separador del nombre.
            '/' | '\\' | ':' => clave.push('-'),
            // Los caracteres seguros se quedan literales.
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' => clave.push(ch),
            // El resto se escapa por unidad de código UTF-16, en mayúsculas y
            // rellenada a cuatro dígitos: `á` es `~00E1`.
            otro => {
                for unidad in otro.encode_utf16(&mut buffer) {
                    let _ = write!(clave, "~{unidad:04X}");
                }
            }
        }
    }

    // El guion inicial es el residuo del separador absoluto de la ruta.
    let clave = clave.trim_start_matches('-');

    // Un cwd sin nombre propio (p. ej. la raíz) no puede llamarse `--`.
    let clave = if clave.is_empty() { "root" } else { clave };

    let mut clave: String = clave.chars().take(251).collect();
    clave.insert_str(0, "--");
    clave.push_str("--");
    clave
}

/// El directorio donde dsh guarda las sesiones de un `cwd`.
pub fn sessions_dir(dsh_home: &Path, cwd: &Path) -> PathBuf {
    dsh_home.join("sessions").join(project_key(cwd))
}

/// Los identificadores de sesión que hay ahora mismo.
///
/// Se toma **antes y después** de la corrida, y la diferencia es la sesión del
/// encargo. No se usa «la más reciente»: un directorio de proyecto acumula las
/// sesiones de todos los intentos, y hubo un caso con dos sesiones bajo el mismo
/// `cwd` que no se pudo explicar. La instantánea es inmune a eso.
pub fn snapshot(dir: &Path) -> BTreeSet<String> {
    let mut nombres = BTreeSet::new();
    // Si el directorio no existe todavía es que aún no hubo ninguna corrida:
    // conjunto vacío, que no es un error sino un hecho.
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return nombres;
    };
    for entrada in entradas.flatten() {
        if entrada.path().is_dir()
            && let Some(nombre) = entrada.file_name().to_str()
        {
            nombres.insert(nombre.to_string());
        }
    }
    nombres
}

/// Lee la procedencia de las sesiones que aparecieron durante la corrida.
///
/// # Errors
///
/// Devuelve el motivo por el que **no se pudo leer**, que el recibo convierte en
/// rojo. Los casos: no apareció ninguna sesión, el fichero no se pudo
/// descomprimir, o los registros no nombran ni proveedor ni modelo.
///
/// La lectura **tolera la cola partida**: si el último registro viene a medias se
/// descarta y se usa lo anterior. Lo que no hace es tratar un fichero ilegible
/// como una corrida sin procedencia.
pub fn read_after(dir: &Path, before: &BTreeSet<String>) -> Result<ObservedProvenance, String> {
    let despues = snapshot(dir);
    let nuevas: Vec<String> = despues.difference(before).cloned().collect();

    let Some(id) = nuevas.first() else {
        return Err("no apareció ninguna sesión durante la corrida".to_string());
    };

    let registro = dir.join(id).join("session.jsonl.zstd");
    let fichero = std::fs::File::open(&registro)
        .map_err(|e| format!("no se pudo abrir `{}`: {e}", registro.display()))?;
    let jsonl = zstd::stream::decode_all(fichero)
        .map_err(|e| format!("no se pudo descomprimir `{}`: {e}", registro.display()))?;
    let jsonl = String::from_utf8_lossy(&jsonl);
    parse_log(&jsonl, &nuevas)
}

/// Extrae la procedencia de un registro ya descomprimido.
///
/// Separado de la lectura del disco para poder probarlo con un registro escrito a
/// mano, incluido uno tronchado.
///
/// # Errors
///
/// El motivo por el que el registro no dice qué corrió: no nombra proveedor ni
/// modelo, o no queda ningún registro completo del que sacarlo.
pub fn parse_log(jsonl: &str, session_ids: &[String]) -> Result<ObservedProvenance, String> {
    let mut proveedor: Option<String> = None;
    let mut modelo: Option<String> = None;
    let mut sandbox_mode: Option<String> = None;
    let mut permission_preset: Option<String> = None;
    let mut herramientas: Vec<(String, u32)> = Vec::new();

    for linea in jsonl.lines() {
        let linea = linea.trim();
        if linea.is_empty() {
            continue;
        }
        // Una línea que no parsea se descarta: la cola partida es normal cuando
        // la sesión estaba en vuelo, y no debe tirar lo anterior.
        let valor: serde_json::Value = match serde_json::from_str(linea) {
            Ok(valor) => valor,
            Err(_) => continue,
        };

        let tipo = valor.get("type").and_then(serde_json::Value::as_str);

        // Proveedor y modelo viven en cualquier registro que los lleve — los
        // reales aparecen en request/context y en assistant/message.
        if proveedor.is_none() {
            proveedor = buscar_texto(&valor, "provider");
        }
        if modelo.is_none() {
            modelo = buscar_texto(&valor, "model");
        }

        match tipo {
            Some("sandbox/mode") => {
                if sandbox_mode.is_none() {
                    sandbox_mode = valor
                        .get("data")
                        .and_then(|datos| datos.get("mode"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                }
            }
            Some("permission/preset") => {
                if permission_preset.is_none() {
                    permission_preset = valor
                        .get("data")
                        .and_then(|datos| datos.get("preset"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                }
            }
            Some("tool/call") => {
                let nombre = valor
                    .get("data")
                    .and_then(|datos| datos.get("name"))
                    .and_then(serde_json::Value::as_str);
                if let Some(nombre) = nombre {
                    match herramientas.iter_mut().find(|(n, _)| n == nombre) {
                        Some((_, cuenta)) => *cuenta += 1,
                        None => herramientas.push((nombre.to_string(), 1)),
                    }
                }
            }
            _ => {}
        }
    }

    match (proveedor, modelo) {
        (None, None) => Err("el registro no nombra proveedor ni modelo".to_string()),
        (_, None) => Err("el registro no nombra el modelo".to_string()),
        (None, _) => Err("el registro no nombra el proveedor".to_string()),
        (Some(proveedor), Some(modelo)) => Ok(ObservedProvenance::new(
            proveedor,
            modelo,
            session_ids.to_vec(),
            herramientas,
            sandbox_mode,
            permission_preset,
        )),
    }
}

/// El primer valor de texto bajo la llave `clave`, a cualquier profundidad.
///
/// Recorrido recursivo a propósito: los eventos tipados anidan los datos a
/// distinta profundidad según el tipo, y la regla es «cualquier registro que los
/// lleve», no una ruta fija por tipo de evento.
fn buscar_texto(valor: &serde_json::Value, clave: &str) -> Option<String> {
    match valor {
        serde_json::Value::Object(mapa) => {
            for (k, v) in mapa {
                if k == clave
                    && let Some(texto) = v.as_str()
                {
                    return Some(texto.to_string());
                }
                if let Some(encontrado) = buscar_texto(v, clave) {
                    return Some(encontrado);
                }
            }
            None
        }
        serde_json::Value::Array(entradas) => {
            for entrada in entradas {
                if let Some(encontrado) = buscar_texto(entrada, clave) {
                    return Some(encontrado);
                }
            }
            None
        }
        _ => None,
    }
}
