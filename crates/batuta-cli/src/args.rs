//! El parseo, a mano.
//!
//! Sin `clap`. Una sola orden no justifica la dependencia —R2: nada se declara,
//! se demuestra— y lo que hay cabe en cuarenta líneas. Se añadirá cuando
//! escribirlo a mano sea peor que depender de ello, y no antes.
// generado: deepseek-v4-flash - revisado: Arquitecto

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
    let Some(primera) = args.first() else {
        return Ok(Command::Help);
    };

    match primera.as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "canary" => parsear_canary(&args[1..]),
        otra => Err(CliError::UnknownCommand {
            given: otra.to_string(),
            available: COMMANDS.to_vec(),
        }),
    }
}

/// Los argumentos de `canary`, después de la orden.
///
/// Bandera por bandera: cada una coge el siguiente argumento como valor, y un
/// valor que empiece por `--` no es un valor sino una bandera que vino sin el
/// suyo. Todo lo demás —una bandera desconocida o un argumento suelto— es un
/// `UnknownFlag` que enumera lo admitido (R8).
fn parsear_canary(args: &[String]) -> Result<Command, CliError> {
    let mut provider: Option<String> = None;
    let mut model: Option<String> = None;

    let mut indice = 0;
    while indice < args.len() {
        let argumento = &args[indice];
        if CANARY_FLAGS.contains(&argumento.as_str()) {
            let Some(valor) = args.get(indice + 1) else {
                return Err(CliError::MissingValue {
                    flag: argumento.clone(),
                });
            };
            if valor.starts_with("--") {
                return Err(CliError::MissingValue {
                    flag: argumento.clone(),
                });
            }
            if argumento == "--provider" {
                provider = Some(valor.clone());
            } else {
                model = Some(valor.clone());
            }
            indice += 2;
        } else {
            return Err(CliError::UnknownFlag {
                given: argumento.clone(),
                available: CANARY_FLAGS.to_vec(),
            });
        }
    }

    let provider = provider.ok_or(CliError::MissingFlag { flag: "--provider" })?;
    Ok(Command::Canary { provider, model })
}
