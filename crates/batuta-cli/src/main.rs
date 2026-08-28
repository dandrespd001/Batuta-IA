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

use batuta_cli::{CanaryOutcome, Command, Layout, USAGE, canary, canary_all, parse};

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
        Err(e) => {
            eprintln!("batuta: {e}");
            eprintln!("\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn ejecutar_canario(proveedor: &str, modelo: Option<&str>, todos: bool) -> ExitCode {
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
