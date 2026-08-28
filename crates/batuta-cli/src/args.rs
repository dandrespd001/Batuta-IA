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
        /// Qué modelo suyo. `None` sólo vale si declara uno solo, o con `all`.
        model: Option<String>,
        /// Todos sus modelos, uno tras otro.
        all: bool,
    },
    /// La tabla que une declaración, evidencia y elección.
    Panel {
        /// Si se pide, sólo enseña este proveedor.
        provider: Option<String>,
        /// Si se pide, la ruta donde escribir la tabla como página HTML
        /// autocontenida, en vez de imprimir la tabla de texto por stdout
        /// (§2/§3 de `docs/FASE5_PANEL.md`: `batuta panel --html <ruta>`).
        /// Todavía sin interpretar como `Path`: eso es trabajo de quien
        /// ejecuta la orden, no del parseo — el mismo trato que
        /// `model_ref: String` en `Enable`/`Disable`.
        html: Option<String>,
    },
    /// Activa un modelo en la política.
    Enable {
        /// `<proveedor>/<modelo>`, todavía sin partir: partirlo pide conocer
        /// los manifiestos, y eso no es trabajo del parseo.
        model_ref: String,
    },
    /// Lo apaga en la política, sin borrar nada.
    Disable {
        /// `<proveedor>/<modelo>`.
        model_ref: String,
    },
    /// Fija el esfuerzo de razonamiento de un modelo.
    Effort {
        /// `<proveedor>/<modelo>`.
        model_ref: String,
        /// El nivel pedido, todavía sin validar contra `ReasoningEffort`.
        level: String,
    },
    /// Escribe una plantilla comentada en `providers/<id>.toml`. La capa de
    /// **Declaración** (§1): un fichero, nunca un parche.
    NuevoProveedor {
        /// El id del proveedor nuevo, todavía sin validar contra `ProviderId`.
        id: String,
    },
    /// Añade un `[[models]]` al final de `providers/<proveedor>.toml`.
    NuevoModelo {
        /// El proveedor al que se añade.
        provider: String,
        /// El id del modelo nuevo, todavía sin validar contra `ModelId`.
        id: String,
        /// El nombre que entiende el proveedor, todavía sin validar contra
        /// `RouteModel`.
        route_model: String,
    },
    /// Quita un modelo del manifiesto e imprime el bloque que borró.
    QuitarModelo {
        /// `<proveedor>/<modelo>`.
        model_ref: String,
    },
    /// La ayuda.
    Help,
}

/// Las órdenes que hay. El error de orden desconocida las enumera (R8).
pub const COMMANDS: &[&str] = &[
    "canary",
    "panel",
    "enable",
    "disable",
    "effort",
    "nuevo-proveedor",
    "nuevo-modelo",
    "quitar-modelo",
    "help",
];

/// Las banderas de `canary` que llevan valor.
pub const CANARY_FLAGS: &[&str] = &["--provider", "--model"];

/// Los interruptores de `canary`: van solos y no llevan valor.
pub const CANARY_SWITCHES: &[&str] = &["--all"];

/// Las banderas de `panel` que llevan valor. `--html` lleva una ruta (§2/§3
/// de `docs/FASE5_PANEL.md`), no es un interruptor.
pub const PANEL_FLAGS: &[&str] = &["--provider", "--html"];

/// La ayuda.
///
/// Un test la compara **contra el parseo**: toda bandera larga que nombre tiene
/// que ser admitida. Es lo que impide que envejezca sola.
pub const USAGE: &str = "\
batuta — orquestador de delegación

USO
    batuta canary --provider <id> [--model <id>]
    batuta canary --provider <id> --all
    batuta panel [--html <ruta>] [--provider <id>]
    batuta enable  <proveedor>/<modelo>
    batuta disable <proveedor>/<modelo>
    batuta effort  <proveedor>/<modelo> <nivel>
    batuta nuevo-proveedor <id>
    batuta nuevo-modelo <proveedor> <id> <ruta>
    batuta quitar-modelo <proveedor>/<modelo>
    batuta help

ÓRDENES
    canary    Lanza el canario de un proveedor y deja su recibo en disco.
              Genera un token irrepetible, pide que lo devuelva, y comprueba
              que volvió ése. Nunca busca una subcadena en un juicio propio.
    panel     La tabla que une declaración (providers/*.toml), evidencia (los
              recibos) y elección (la política): qué hay, qué funcionó y
              cuándo, y qué se quiere usar. Sólo lee: no lanza nada.
    enable    Activa un modelo en la política. No lo canaria ni lo declara:
              sólo dice que, si tiene evidencia, se puede enrutar.
    disable   Lo apaga en la política. No borra ni el manifiesto ni sus
              recibos: la evidencia sigue siendo cierta aunque no se use.
    effort    Fija el nivel de esfuerzo de un modelo. Falla si su proveedor
              no declara ningún mapa de esfuerzo, en vez de guardar un valor
              que nunca se va a poder honrar.
    nuevo-proveedor  Escribe una plantilla comentada en providers/<id>.toml.
              No sobrescribe nunca un proveedor que ya exista.
    nuevo-modelo     Añade un [[models]] al final de
              providers/<proveedor>.toml. Nunca reescribe lo que ya había:
              los comentarios previos sobreviven byte a byte.
    quitar-modelo    Lo quita del manifiesto e imprime el bloque que borró.
              Es la única forma honesta de borrar de un fichero cuyos
              comentarios llevan mediciones reales.

BANDERAS DE canary
    --provider <id>   El proveedor, tal como lo nombra su manifiesto.
    --model <id>      Uno de sus modelos. Obligatoria si declara más de uno:
                      con varios, batuta no elige en silencio.
    --all             Todos sus modelos, uno tras otro. Un modelo rojo no
                      detiene a los demás: el lote existe para saber cuáles
                      valen. Incompatible con --model.

BANDERAS DE panel
    --provider <id>   Enseña sólo este proveedor. Sin ella, todos.
    --html <ruta>     Escribe la tabla como página HTML autocontenida en esa
                      ruta, en vez de imprimirla por stdout: sin red, sin
                      CDN, de sólo lectura. Se combina con --provider.

<proveedor>/<modelo>
    El identificador de batuta, tal como aparece en la primera columna de
    `batuta panel`: por ejemplo dsh/dsh-deepseek-v4-flash.

<nivel>
    Uno de: low, medium, high, xhigh, max.

SALIDA de canary
    0    el canario salió verde (con --all: todos)
    1    salió rojo; el motivo se imprime (con --all: al menos uno)
    2    no llegó a haber veredicto; el motivo se imprime

SALIDA de panel, enable, disable, effort, nuevo-proveedor, nuevo-modelo, quitar-modelo
    0    se pudo hacer lo que se pidió
    2    no se pudo: el motivo se imprime
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
        "panel" => parsear_panel(&args[1..]),
        "enable" => {
            parsear_referencia(&args[1..], "enable").map(|model_ref| Command::Enable { model_ref })
        }
        "disable" => parsear_referencia(&args[1..], "disable")
            .map(|model_ref| Command::Disable { model_ref }),
        "effort" => parsear_effort(&args[1..]),
        "nuevo-proveedor" => parsear_nuevo_proveedor(&args[1..]),
        "nuevo-modelo" => parsear_nuevo_modelo(&args[1..]),
        "quitar-modelo" => parsear_referencia(&args[1..], "quitar-modelo")
            .map(|model_ref| Command::QuitarModelo { model_ref }),
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
    let mut all = false;

    let mut indice = 0;
    while indice < args.len() {
        let argumento = &args[indice];
        if CANARY_SWITCHES.contains(&argumento.as_str()) {
            all = true;
            indice += 1;
        } else if CANARY_FLAGS.contains(&argumento.as_str()) {
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
            let mut available = CANARY_FLAGS.to_vec();
            available.extend_from_slice(CANARY_SWITCHES);
            return Err(CliError::UnknownFlag {
                given: argumento.clone(),
                available,
            });
        }
    }

    // Contradecirse es un error, no una preferencia que batuta resuelva por su
    // cuenta: elegir en silencio entre dos instrucciones incompatibles es la
    // forma exacta en que se pidió un modelo y corrió otro.
    if all && model.is_some() {
        return Err(CliError::ContradictoryFlags {
            one: "--all",
            other: "--model",
        });
    }

    let provider = provider.ok_or(CliError::MissingFlag { flag: "--provider" })?;
    Ok(Command::Canary {
        provider,
        model,
        all,
    })
}

/// Los argumentos de `panel`, después de la orden.
///
/// Dos banderas con valor (`--provider`, `--html`), en cualquier orden: el
/// mismo patrón que `parsear_canary` ya usa para `--provider`/`--model`.
/// Ninguna de las dos se valida ni se interpreta aquí —`--html` guarda la
/// ruta tal cual, como texto; convertirla a `Path` es trabajo de quien
/// ejecuta la orden.
fn parsear_panel(args: &[String]) -> Result<Command, CliError> {
    let mut provider: Option<String> = None;
    let mut html: Option<String> = None;

    let mut indice = 0;
    while indice < args.len() {
        let argumento = &args[indice];
        if PANEL_FLAGS.contains(&argumento.as_str()) {
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
                html = Some(valor.clone());
            }
            indice += 2;
        } else {
            return Err(CliError::UnknownFlag {
                given: argumento.clone(),
                available: PANEL_FLAGS.to_vec(),
            });
        }
    }

    Ok(Command::Panel { provider, html })
}

/// Un único posicional: `<proveedor>/<modelo>`, sin partir todavía. Partirlo y
/// comprobar que existe es trabajo de `eleccion`, que sí conoce los
/// manifiestos; aquí sólo se cuenta cuántos argumentos llegaron.
fn parsear_referencia(args: &[String], comando: &'static str) -> Result<String, CliError> {
    match args {
        [referencia] => Ok(referencia.clone()),
        [] => Err(CliError::MissingArgument {
            command: comando,
            argument: "<proveedor>/<modelo>",
        }),
        [_, sobra, ..] => Err(CliError::UnexpectedArgument {
            command: comando,
            given: sobra.clone(),
        }),
    }
}

/// `effort` lleva dos posicionales: la referencia y el nivel.
fn parsear_effort(args: &[String]) -> Result<Command, CliError> {
    match args {
        [referencia, nivel] => Ok(Command::Effort {
            model_ref: referencia.clone(),
            level: nivel.clone(),
        }),
        [] => Err(CliError::MissingArgument {
            command: "effort",
            argument: "<proveedor>/<modelo>",
        }),
        [_referencia] => Err(CliError::MissingArgument {
            command: "effort",
            argument: "<nivel>",
        }),
        [_, _, sobra, ..] => Err(CliError::UnexpectedArgument {
            command: "effort",
            given: sobra.clone(),
        }),
    }
}

/// `nuevo-proveedor` lleva un único posicional: el id. Su forma se valida
/// contra `ProviderId` más tarde, en `declaracion::nuevo_proveedor` —aquí
/// sólo se cuenta cuántos argumentos llegaron, igual que en
/// [`parsear_referencia`].
fn parsear_nuevo_proveedor(args: &[String]) -> Result<Command, CliError> {
    match args {
        [id] => Ok(Command::NuevoProveedor { id: id.clone() }),
        [] => Err(CliError::MissingArgument {
            command: "nuevo-proveedor",
            argument: "<id>",
        }),
        [_, sobra, ..] => Err(CliError::UnexpectedArgument {
            command: "nuevo-proveedor",
            given: sobra.clone(),
        }),
    }
}

/// `nuevo-modelo` lleva tres posicionales: proveedor, id y `route_model`.
fn parsear_nuevo_modelo(args: &[String]) -> Result<Command, CliError> {
    match args {
        [provider, id, route_model] => Ok(Command::NuevoModelo {
            provider: provider.clone(),
            id: id.clone(),
            route_model: route_model.clone(),
        }),
        [] => Err(CliError::MissingArgument {
            command: "nuevo-modelo",
            argument: "<proveedor>",
        }),
        [_provider] => Err(CliError::MissingArgument {
            command: "nuevo-modelo",
            argument: "<id>",
        }),
        [_provider, _id] => Err(CliError::MissingArgument {
            command: "nuevo-modelo",
            argument: "<ruta>",
        }),
        [_, _, _, sobra, ..] => Err(CliError::UnexpectedArgument {
            command: "nuevo-modelo",
            given: sobra.clone(),
        }),
    }
}
