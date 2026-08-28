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

use batuta_cli::{Command, Layout, USAGE, canary, parse};

fn main() -> ExitCode {
    let argumentos: Vec<String> = std::env::args().skip(1).collect();

    match parse(&argumentos) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Canary { provider, model }) => ejecutar_canario(&provider, model.as_deref()),
        Err(e) => {
            eprintln!("batuta: {e}");
            eprintln!("\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn ejecutar_canario(proveedor: &str, modelo: Option<&str>) -> ExitCode {
    let disposicion = match Layout::from_env() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("batuta: no hay dónde guardar el estado: {e}");
            return ExitCode::from(2);
        }
    };

    // Los manifiestos van junto al ejecutable en el repositorio; se pueden mover
    // con `BATUTA_PROVIDERS`, que es lo que usan las pruebas de integración y lo
    // que hará falta cuando batuta se instale fuera de su propio árbol.
    let proveedores = std::env::var_os("BATUTA_PROVIDERS")
        .map_or_else(|| std::path::PathBuf::from("providers"), Into::into);
    let dsh_home = std::env::var_os("DSH_HOME")
        .map_or_else(|| disposicion.root().join("dsh"), std::path::PathBuf::from);

    match canary(proveedor, modelo, &proveedores, &disposicion, &dsh_home) {
        Ok(salida) => {
            println!("recibo: {}", salida.receipt_path.display());
            if salida.receipt.verdict().is_green() {
                println!(
                    "verde — modelo {}",
                    if salida.receipt.model_confirmed() {
                        "confirmado"
                    } else {
                        "sin confirmar: el proveedor no deja registro legible"
                    }
                );
                ExitCode::SUCCESS
            } else {
                eprintln!("rojo — {:?}", salida.receipt.verdict());
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("batuta: {e}");
            ExitCode::from(2)
        }
    }
}
