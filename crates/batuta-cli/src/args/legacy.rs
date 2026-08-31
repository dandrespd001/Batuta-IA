//! Parseo de las superficies de declaración, política y canarios.

use crate::error::CliError;

use super::{CANARY_FLAGS, CANARY_SWITCHES, Command, PANEL_FLAGS};

/// Interpreta las banderas de `canary` sin elegir valores implícitos.
pub(super) fn parsear_canary(args: &[String]) -> Result<Command, CliError> {
    let mut provider = None;
    let mut model = None;
    let mut capability = None;
    let mut all = false;

    let mut indice = 0;
    while indice < args.len() {
        let argumento = &args[indice];
        if CANARY_SWITCHES.contains(&argumento.as_str()) {
            all = true;
            indice += 1;
        } else if CANARY_FLAGS.contains(&argumento.as_str()) {
            let valor = valor_de_bandera(args, indice, argumento)?;
            match argumento.as_str() {
                "--provider" => provider = Some(valor.clone()),
                "--capability" => capability = Some(valor.clone()),
                _ => model = Some(valor.clone()),
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
        capability,
    })
}

fn valor_de_bandera<'a>(
    args: &'a [String],
    indice: usize,
    bandera: &str,
) -> Result<&'a String, CliError> {
    let Some(valor) = args.get(indice + 1) else {
        return Err(CliError::MissingValue {
            flag: bandera.to_owned(),
        });
    };
    if valor.starts_with("--") {
        return Err(CliError::MissingValue {
            flag: bandera.to_owned(),
        });
    }
    Ok(valor)
}

/// Interpreta las dos banderas opcionales de `panel` en cualquier orden.
pub(super) fn parsear_panel(args: &[String]) -> Result<Command, CliError> {
    let mut provider = None;
    let mut html = None;

    let mut indice = 0;
    while indice < args.len() {
        let argumento = &args[indice];
        if !PANEL_FLAGS.contains(&argumento.as_str()) {
            return Err(CliError::UnknownFlag {
                given: argumento.clone(),
                available: PANEL_FLAGS.to_vec(),
            });
        }
        let valor = valor_de_bandera(args, indice, argumento)?;
        if argumento == "--provider" {
            provider = Some(valor.clone());
        } else {
            html = Some(valor.clone());
        }
        indice += 2;
    }

    Ok(Command::Panel { provider, html })
}

pub(super) fn parsear_referencia(
    args: &[String],
    comando: &'static str,
) -> Result<String, CliError> {
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

pub(super) fn parsear_effort(args: &[String]) -> Result<Command, CliError> {
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

pub(super) fn parsear_nuevo_proveedor(args: &[String]) -> Result<Command, CliError> {
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

pub(super) fn parsear_nuevo_modelo(args: &[String]) -> Result<Command, CliError> {
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
