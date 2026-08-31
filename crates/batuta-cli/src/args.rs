//! Parseo estricto de la línea de órdenes, separado por superficie.
// generado: deepseek-v4-flash - revisado: Arquitecto

use serde::Serialize;

use crate::error::CliError;

mod legacy;
mod operational;
mod routing;

pub use operational::{ExecutionProfileCommand, ExecutorCommand, GrantCommand, RunCommand};

/// Lo que se puede pedir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Grants durables de ejecución.
    Grant {
        /// Suborden cerrada.
        command: GrantCommand,
    },
    /// Inicia, consulta o continúa una corrida durable.
    Run {
        /// Suborden cerrada.
        command: RunCommand,
    },
    /// Configuración operativa del ejecutor.
    Executor {
        /// Suborden cerrada.
        command: ExecutorCommand,
    },
    /// Simula routing a partir de un sobre JSON versionado.
    Route {
        /// JSON literal; `None` usa fichero o stdin.
        json: Option<String>,
        /// Fichero JSON; `None` usa literal o stdin.
        file: Option<String>,
    },
    /// Importación DSH mediante staging y aplicación confirmada.
    Catalog {
        /// Suborden cerrada.
        command: CatalogCommand,
    },
    /// Gestión bajo demanda de evidencia investigada.
    Research {
        /// Suborden cerrada.
        command: ResearchCommand,
    },
    /// Interfaz terminal sin servidor.
    Tui {
        /// Sobre JSON cuya decisión explica la interfaz al abrir.
        route_file: Option<String>,
    },
    /// Servidor MCP JSON-RPC por stdio.
    Mcp,
    /// La corrida más pequeña que demuestra que un proveedor responde.
    Canary {
        /// Qué proveedor.
        provider: String,
        /// Qué modelo suyo. `None` sólo vale si declara uno solo, o con `all`.
        model: Option<String>,
        /// Todos sus modelos, uno tras otro.
        all: bool,
        /// Escenario de capacidad que debe demostrar; `None` usa el eco básico.
        capability: Option<String>,
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

/// Alcance explícito de una actualización de investigación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ResearchScope {
    /// Todas las rutas y acciones configuradas.
    All,
    /// Una ruta exacta o alias que se resolverá después.
    Route(String),
    /// Un perfil de acción.
    Action(String),
}

/// Subórdenes de `research`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchCommand {
    /// Crea staging; nunca activa.
    Update {
        /// Qué investigar.
        scope: ResearchScope,
    },
    /// Muestra activo y staging.
    Status,
    /// Aplica una propuesta sólo con confirmación visible.
    Apply {
        /// Identificador de propuesta.
        proposal: String,
        /// Confirmación explícita.
        confirm: bool,
    },
}

/// Subórdenes de `catalog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogCommand {
    /// Normaliza un documento de descubrimiento DSH y crea staging.
    Import {
        /// Fichero JSON producido por la API de descubrimiento.
        file: Option<String>,
    },
    /// Muestra hash activo, rutas y propuestas.
    Status,
    /// Activa una propuesta sólo con confirmación explícita.
    Apply {
        /// Identificador de propuesta.
        proposal: String,
        /// Presencia visible de `--confirm`.
        confirm: bool,
    },
}

/// Las órdenes que hay. El error de orden desconocida las enumera (R8).
pub const COMMANDS: &[&str] = &[
    "grant",
    "run",
    "executor",
    "route",
    "catalog",
    "research",
    "tui",
    "mcp",
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
pub const CANARY_FLAGS: &[&str] = &["--provider", "--model", "--capability"];

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
    batuta grant create --file <grant.json> --confirm
    batuta grant status <id>
    batuta grant revoke <id> --confirm
    batuta run [--file <request.json>]
    batuta run status <id>
    batuta run resume <id>
    batuta executor profile import --file <profile.json>
    batuta executor profile status
    batuta executor profile apply <propuesta> --expected-hash <hash> --confirm
    batuta route [--json <documento> | --file <ruta>]
    batuta catalog import --file <dsh-discovery.json>
    batuta catalog status
    batuta catalog apply <propuesta> --confirm
    batuta research update [--all | --route <ruta> | --action <acción>]
    batuta research status
    batuta research apply <propuesta> --confirm
    batuta tui [--route <fichero-json>]
    batuta mcp
    batuta canary --provider <id> [--model <id>] [--capability <capacidad>]
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
    grant     Crea, consulta y revoca autorizaciones durables y selladas.
    run       Ejecuta o continúa una corrida desde su estado durable.
    executor  Gestiona el perfil operativo mediante staging, CAS y confirmación.
    route     Simula y explica una ruta. Sin bandera de entrada lee JSON por stdin.
    catalog   Importa DSH a staging, muestra estado o aplica con confirmación.
    research  Actualiza staging, muestra estado o aplica una propuesta confirmada.
    tui       Abre la interfaz terminal local, sin servidor; puede explicar un routing.
    mcp       Atiende JSON-RPC 2.0 por stdin/stdout, una petición por línea.
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
    --capability <c>  Ejecuta el escenario declarado para read, write, tools o
                      web_research y sólo lo demuestra si observa su uso real.
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
        "grant" => operational::parse_grant(&args[1..]),
        "run" => operational::parse_run(&args[1..]),
        "executor" => operational::parse_executor(&args[1..]),
        "route" => routing::parsear_route(&args[1..]),
        "catalog" => routing::parsear_catalog(&args[1..]),
        "research" => routing::parsear_research(&args[1..]),
        "tui" => routing::parsear_tui(&args[1..]),
        "mcp" => routing::sin_argumentos(&args[1..], "mcp", Command::Mcp),
        "canary" => legacy::parsear_canary(&args[1..]),
        "panel" => legacy::parsear_panel(&args[1..]),
        "enable" => legacy::parsear_referencia(&args[1..], "enable")
            .map(|model_ref| Command::Enable { model_ref }),
        "disable" => legacy::parsear_referencia(&args[1..], "disable")
            .map(|model_ref| Command::Disable { model_ref }),
        "effort" => legacy::parsear_effort(&args[1..]),
        "nuevo-proveedor" => legacy::parsear_nuevo_proveedor(&args[1..]),
        "nuevo-modelo" => legacy::parsear_nuevo_modelo(&args[1..]),
        "quitar-modelo" => legacy::parsear_referencia(&args[1..], "quitar-modelo")
            .map(|model_ref| Command::QuitarModelo { model_ref }),
        otra => Err(CliError::UnknownCommand {
            given: otra.to_string(),
            available: COMMANDS.to_vec(),
        }),
    }
}
