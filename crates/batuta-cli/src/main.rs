//! El binario `batuta`.
//!
//! Tres códigos de salida, y la distinción entre el segundo y el tercero es la
//! misma que el recibo hace entre «no lo pude leer» y «este proveedor no lo
//! ofrece»: un canario rojo **es** una respuesta, y no haber llegado a preguntar
//! no lo es. Colapsarlos en un solo `1` obliga a leer la salida para saber cuál
//! de las dos cosas pasó.
//!
//! * `0` — el canario salió verde.
//! * `1` — el canario salió rojo. El motivo va por `stderr`.
//! * `2` — no llegó a haber veredicto. El motivo va por `stderr`.

use std::path::PathBuf;
use std::process::ExitCode;

use batuta_cli::{
    CanaryOutcome, Command, Layout, USAGE, canary, canary_all, disable, effort, enable, filas,
    nuevo_modelo, nuevo_proveedor, parse, quitar_modelo_de, tabla,
};

fn main() -> ExitCode {
    let argumentos: Vec<String> = std::env::args().skip(1).collect();

    match parse(&argumentos) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Canary {
            provider,
            model,
            all,
        }) => ejecutar_canario(&provider, model.as_deref(), all),
        Ok(Command::Panel { provider }) => ejecutar_panel(provider.as_deref()),
        Ok(Command::Enable { model_ref }) => {
            ejecutar_eleccion("enable", &model_ref, |p, l| enable(p, l, &model_ref))
        }
        Ok(Command::Disable { model_ref }) => {
            ejecutar_eleccion("disable", &model_ref, |p, l| disable(p, l, &model_ref))
        }
        Ok(Command::Effort { model_ref, level }) => {
            ejecutar_eleccion("effort", &model_ref, |p, l| {
                effort(p, l, &model_ref, &level)
            })
        }
        Ok(Command::NuevoProveedor { id }) => ejecutar_nuevo_proveedor(&id),
        Ok(Command::NuevoModelo {
            provider,
            id,
            route_model,
        }) => ejecutar_nuevo_modelo(&provider, &id, &route_model),
        Ok(Command::QuitarModelo { model_ref }) => ejecutar_quitar_modelo(&model_ref),
        Err(e) => {
            eprintln!("batuta: {e}");
            eprintln!("\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Dónde están los manifiestos.
///
/// Van junto al ejecutable en el repositorio; se pueden mover con
/// `BATUTA_PROVIDERS`, que es lo que usan las pruebas de integración y lo que
/// hará falta cuando batuta se instale fuera de su propio árbol.
fn providers_dir() -> PathBuf {
    std::env::var_os("BATUTA_PROVIDERS").map_or_else(|| PathBuf::from("providers"), Into::into)
}

/// El estado y el directorio de manifiestos, resueltos una vez.
fn entorno() -> Result<(Layout, PathBuf), ExitCode> {
    let disposicion = Layout::from_env().map_err(|e| {
        eprintln!("batuta: no hay dónde guardar el estado: {e}");
        ExitCode::from(2)
    })?;
    Ok((disposicion, providers_dir()))
}

fn ejecutar_canario(proveedor: &str, modelo: Option<&str>, todos: bool) -> ExitCode {
    let (disposicion, proveedores) = match entorno() {
        Ok(v) => v,
        Err(codigo) => return codigo,
    };
    let dsh_home =
        std::env::var_os("DSH_HOME").map_or_else(|| disposicion.root().join("dsh"), PathBuf::from);

    let salidas = if todos {
        canary_all(proveedor, &proveedores, &disposicion, &dsh_home)
    } else {
        canary(proveedor, modelo, &proveedores, &disposicion, &dsh_home).map(|una| vec![una])
    };

    match salidas {
        Ok(salidas) => informar(&salidas),
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// `batuta panel`: imprime la tabla y sale con 0, salvo que no se pudiera
/// siquiera construirla.
fn ejecutar_panel(proveedor: Option<&str>) -> ExitCode {
    let (disposicion, proveedores) = match entorno() {
        Ok(v) => v,
        Err(codigo) => return codigo,
    };

    match filas(&proveedores, &disposicion, proveedor) {
        Ok(filas) => {
            print!("{}", tabla(&filas));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// `enable`, `disable` y `effort`: leen la política, la cambian, la guardan.
/// Cada llamador trae su propia acción ya cerrada sobre `model_ref` (y, para
/// `effort`, sobre el nivel): aquí sólo se resuelve el entorno una vez y se
/// informa igual para las tres.
fn ejecutar_eleccion(
    orden: &str,
    model_ref: &str,
    accion: impl FnOnce(&std::path::Path, &Layout) -> Result<(), batuta_cli::CliError>,
) -> ExitCode {
    let (disposicion, proveedores) = match entorno() {
        Ok(v) => v,
        Err(codigo) => return codigo,
    };

    match accion(&proveedores, &disposicion) {
        Ok(()) => {
            println!("{orden} {model_ref}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// `nuevo-proveedor`: escribe la plantilla y confirma dónde quedó.
///
/// No necesita `Layout`: `nuevo-proveedor` sólo toca `providers/`, nunca el
/// estado (§1 — Declaración y Elección no se mezclan).
fn ejecutar_nuevo_proveedor(id: &str) -> ExitCode {
    match nuevo_proveedor(&providers_dir(), id) {
        Ok(destino) => {
            println!("nuevo-proveedor {id} → {}", destino.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// `nuevo-modelo`: añade el modelo y confirma.
fn ejecutar_nuevo_modelo(provider: &str, id: &str, route_model: &str) -> ExitCode {
    match nuevo_modelo(&providers_dir(), provider, id, route_model) {
        Ok(()) => {
            println!("nuevo-modelo {provider}/{id}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// `quitar-modelo`: imprime el bloque que borró. El checklist de T6 lo exige
/// literalmente — es la única forma honesta de borrar de un fichero cuyos
/// comentarios llevan mediciones reales.
fn ejecutar_quitar_modelo(model_ref: &str) -> ExitCode {
    match quitar_modelo_de(&providers_dir(), model_ref) {
        Ok(bloque) => {
            println!("{bloque}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}

/// Una línea por canario, y el código de salida del conjunto.
///
/// Con `--all`, **rojo si alguno lo es**. Un lote no es verde «en general»: o
/// todos sus modelos están permitidos o no lo están, y decir «casi» no sirve
/// para enrutar nada.
fn informar(salidas: &[CanaryOutcome]) -> ExitCode {
    let mut rojos = 0;
    for salida in salidas {
        let recibo = &salida.receipt;
        if recibo.verdict().is_green() {
            println!(
                "verde  {}  modelo {}  → {}",
                recibo.model_requested(),
                if recibo.model_confirmed() {
                    "confirmado"
                } else {
                    "sin confirmar"
                },
                salida.receipt_path.display()
            );
        } else {
            rojos += 1;
            eprintln!(
                "ROJO   {}  {:?}  → {}",
                recibo.model_requested(),
                recibo.verdict(),
                salida.receipt_path.display()
            );
        }
    }

    if rojos == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{rojos} de {} en rojo", salidas.len());
        ExitCode::from(1)
    }
}
