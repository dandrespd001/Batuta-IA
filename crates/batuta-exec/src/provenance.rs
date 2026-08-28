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
pub fn project_key(_cwd: &Path) -> String {
    todo!()
}

/// El directorio donde dsh guarda las sesiones de un `cwd`.
pub fn sessions_dir(_dsh_home: &Path, _cwd: &Path) -> PathBuf {
    todo!()
}

/// Los identificadores de sesión que hay ahora mismo.
///
/// Se toma **antes y después** de la corrida, y la diferencia es la sesión del
/// encargo. No se usa «la más reciente»: un directorio de proyecto acumula las
/// sesiones de todos los intentos, y hubo un caso con dos sesiones bajo el mismo
/// `cwd` que no se pudo explicar. La instantánea es inmune a eso.
pub fn snapshot(_dir: &Path) -> BTreeSet<String> {
    todo!()
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
pub fn read_after(_dir: &Path, _before: &BTreeSet<String>) -> Result<ObservedProvenance, String> {
    todo!()
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
pub fn parse_log(_jsonl: &str, _session_ids: &[String]) -> Result<ObservedProvenance, String> {
    todo!()
}
