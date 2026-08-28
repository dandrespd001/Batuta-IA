//! El parseo, a mano.
//!
//! Sin `clap`. Una sola orden no justifica la dependencia —R2: nada se declara,
//! se demuestra— y lo que hay cabe en cuarenta líneas. Se añadirá cuando
//! escribirlo a mano sea peor que depender de ello, y no antes.

use crate::error::CliError;

/// Lo que se puede pedir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// La corrida más pequeña que demuestra que un proveedor responde.
    Canary {
        /// Qué proveedor.
        provider: String,
        /// Qué modelo suyo. `None` sólo vale si declara uno solo.
        model: Option<String>,
    },
    /// La ayuda.
    Help,
}

/// Las órdenes que hay. El error de orden desconocida las enumera (R8).
pub const COMMANDS: &[&str] = &["canary", "help"];

/// Las banderas de `canary`.
pub const CANARY_FLAGS: &[&str] = &["--provider", "--model"];

/// La ayuda.
///
/// Un test la compara **contra el parseo**: toda bandera larga que nombre tiene
/// que ser admitida. Es lo que impide que envejezca sola.
pub const USAGE: &str = "\
batuta — orquestador de delegación

USO
    batuta canary --provider <id> [--model <id>]
    batuta help

ÓRDENES
    canary    Lanza el canario de un proveedor y deja su recibo en disco.
              Genera un token irrepetible, pide que lo devuelva, y comprueba
              que volvió ése. Nunca busca una subcadena en un juicio propio.

BANDERAS DE canary
    --provider <id>   El proveedor, tal como lo nombra su manifiesto.
    --model <id>      Uno de sus modelos. Obligatoria si declara más de uno:
                      con varios, batuta no elige en silencio.

SALIDA
    0    el canario salió verde
    1    el canario salió rojo; el motivo se imprime
    2    no llegó a haber veredicto; el motivo se imprime
";

/// Interpreta los argumentos, sin el nombre del programa.
///
/// # Errors
///
/// [`CliError::UnknownCommand`], [`CliError::MissingValue`],
/// [`CliError::MissingFlag`] o [`CliError::UnknownFlag`]. Todos enumeran lo
/// válido.
pub fn parse(args: &[String]) -> Result<Command, CliError> {
    // Fase roja: el parámetro es el contrato, no sobra.
    let _ = args;
    todo!()
}
