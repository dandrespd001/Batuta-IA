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

use std::process::ExitCode;

use batuta_cli::{Command, ExecutorCommand, USAGE, parse};

mod main_handlers;

use main_handlers::{
    ejecutar_canario, ejecutar_catalog, ejecutar_disable, ejecutar_effort, ejecutar_enable,
    ejecutar_grant, ejecutar_mcp, ejecutar_nuevo_modelo, ejecutar_nuevo_proveedor, ejecutar_panel,
    ejecutar_profile, ejecutar_quitar_modelo, ejecutar_research, ejecutar_route, ejecutar_run,
    ejecutar_tui,
};

fn main() -> ExitCode {
    let argumentos: Vec<String> = std::env::args().skip(1).collect();

    match parse(&argumentos) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Grant { command }) => ejecutar_grant(&command),
        Ok(Command::Run { command }) => ejecutar_run(&command),
        Ok(Command::Executor {
            command: ExecutorCommand::Profile { command },
        }) => ejecutar_profile(&command),
        Ok(Command::Route { json, file }) => ejecutar_route(json.as_deref(), file.as_deref()),
        Ok(Command::Catalog { command }) => ejecutar_catalog(&command),
        Ok(Command::Research { command }) => ejecutar_research(&command),
        Ok(Command::Tui { route_file }) => ejecutar_tui(route_file.as_deref()),
        Ok(Command::Mcp) => ejecutar_mcp(),
        Ok(Command::Canary {
            provider,
            model,
            all,
            capability,
        }) => ejecutar_canario(&provider, model.as_deref(), all, capability.as_deref()),
        Ok(Command::Panel { provider, html }) => {
            ejecutar_panel(provider.as_deref(), html.as_deref())
        }
        Ok(Command::Enable { model_ref }) => ejecutar_enable(&model_ref),
        Ok(Command::Disable { model_ref }) => ejecutar_disable(&model_ref),
        Ok(Command::Effort { model_ref, level }) => ejecutar_effort(&model_ref, &level),
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
