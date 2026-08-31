//! Adaptadores de canarios, panel, política y declaración.

use std::path::PathBuf;
use std::process::ExitCode;

use batuta_cli::{
    CanaryOutcome, Layout, canary, canary_all, canary_capability, canary_capability_all, disable,
    effort, enable, escribir_html, filas, nuevo_modelo, nuevo_proveedor, quitar_modelo_de, tabla,
};

fn providers_dir() -> PathBuf {
    std::env::var_os("BATUTA_PROVIDERS").map_or_else(|| PathBuf::from("providers"), Into::into)
}

pub(super) fn entorno() -> Result<(Layout, PathBuf), ExitCode> {
    let disposicion = Layout::from_env().map_err(|error| {
        eprintln!("batuta: no hay dónde guardar el estado: {error}");
        ExitCode::from(2)
    })?;
    Ok((disposicion, providers_dir()))
}

pub(crate) fn ejecutar_canario(
    proveedor: &str,
    modelo: Option<&str>,
    todos: bool,
    capability: Option<&str>,
) -> ExitCode {
    let (disposicion, proveedores) = match entorno() {
        Ok(environment) => environment,
        Err(code) => return code,
    };
    let dsh_home =
        std::env::var_os("DSH_HOME").map_or_else(|| disposicion.root().join("dsh"), PathBuf::from);

    let capability = match capability.map(str::parse::<batuta_contract::Capability>) {
        Some(Ok(capability)) => Some(capability),
        Some(Err(error)) => {
            eprintln!("batuta: {error}");
            return ExitCode::from(2);
        }
        None => None,
    };

    let outcomes = match (todos, capability) {
        (true, Some(capability)) => {
            canary_capability_all(proveedor, capability, &proveedores, &disposicion, &dsh_home)
        }
        (true, None) => canary_all(proveedor, &proveedores, &disposicion, &dsh_home),
        (false, Some(capability)) => canary_capability(
            proveedor,
            modelo,
            capability,
            &proveedores,
            &disposicion,
            &dsh_home,
        )
        .map(|outcome| vec![outcome]),
        (false, None) => canary(proveedor, modelo, &proveedores, &disposicion, &dsh_home)
            .map(|outcome| vec![outcome]),
    };

    match outcomes {
        Ok(outcomes) => informar(&outcomes),
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_panel(proveedor: Option<&str>, html: Option<&str>) -> ExitCode {
    let (layout, providers) = match entorno() {
        Ok(environment) => environment,
        Err(code) => return code,
    };

    match filas(&providers, &layout, proveedor) {
        Ok(rows) => print_panel(&rows, html),
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

fn print_panel(rows: &[batuta_cli::Fila], html: Option<&str>) -> ExitCode {
    let Some(path) = html else {
        print!("{}", tabla(rows));
        return ExitCode::SUCCESS;
    };
    let path = std::path::Path::new(path);
    match escribir_html(path, rows) {
        Ok(()) => {
            println!("panel --html → {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_enable(model_ref: &str) -> ExitCode {
    ejecutar_eleccion("enable", model_ref, |providers, layout| {
        enable(providers, layout, model_ref)
    })
}

pub(crate) fn ejecutar_disable(model_ref: &str) -> ExitCode {
    ejecutar_eleccion("disable", model_ref, |providers, layout| {
        disable(providers, layout, model_ref)
    })
}

pub(crate) fn ejecutar_effort(model_ref: &str, level: &str) -> ExitCode {
    ejecutar_eleccion("effort", model_ref, |providers, layout| {
        effort(providers, layout, model_ref, level)
    })
}

fn ejecutar_eleccion(
    orden: &str,
    model_ref: &str,
    accion: impl FnOnce(&std::path::Path, &Layout) -> Result<(), batuta_cli::CliError>,
) -> ExitCode {
    let (layout, providers) = match entorno() {
        Ok(environment) => environment,
        Err(code) => return code,
    };

    match accion(&providers, &layout) {
        Ok(()) => {
            println!("{orden} {model_ref}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_nuevo_proveedor(id: &str) -> ExitCode {
    match nuevo_proveedor(&providers_dir(), id) {
        Ok(destination) => {
            println!("nuevo-proveedor {id} → {}", destination.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_nuevo_modelo(provider: &str, id: &str, route_model: &str) -> ExitCode {
    match nuevo_modelo(&providers_dir(), provider, id, route_model) {
        Ok(()) => {
            println!("nuevo-modelo {provider}/{id}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_quitar_modelo(model_ref: &str) -> ExitCode {
    match quitar_modelo_de(&providers_dir(), model_ref) {
        Ok(block) => {
            println!("{block}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

fn informar(outcomes: &[CanaryOutcome]) -> ExitCode {
    let mut red = 0;
    for outcome in outcomes {
        let receipt = &outcome.receipt;
        if receipt.verdict().is_green() {
            println!(
                "verde  {}  modelo {}  → {}",
                receipt.model_requested(),
                if receipt.model_confirmed() {
                    "confirmado"
                } else {
                    "sin confirmar"
                },
                outcome.receipt_path.display()
            );
        } else {
            red += 1;
            eprintln!(
                "ROJO   {}  {:?}  → {}",
                receipt.model_requested(),
                receipt.verdict(),
                outcome.receipt_path.display()
            );
        }
    }

    if red == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{red} de {} en rojo", outcomes.len());
        ExitCode::from(1)
    }
}
