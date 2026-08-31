//! Parseo de las órdenes operativas K4.

use crate::error::CliError;

use super::Command;

/// Subórdenes de `grant`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantCommand {
    /// Valida y guarda un grant sellado.
    Create {
        /// Borrador JSON.
        file: String,
        /// Confirmación explícita.
        confirm: bool,
    },
    /// Consulta grant y revocación sin alterar historia.
    Status {
        /// Identificador exacto.
        id: String,
    },
    /// Añade una revocación append-only.
    Revoke {
        /// Identificador exacto.
        id: String,
        /// Confirmación explícita.
        confirm: bool,
    },
}

/// Subórdenes de `run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCommand {
    /// Inicia desde fichero o stdin.
    Start {
        /// Fichero JSON; `None` significa stdin.
        file: Option<String>,
    },
    /// Lee el estado durable.
    Status {
        /// Identificador exacto.
        id: String,
    },
    /// Continúa exclusivamente desde estado durable.
    Resume {
        /// Identificador exacto.
        id: String,
    },
}

/// Subórdenes de `executor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorCommand {
    /// Perfil operativo compartido.
    Profile {
        /// Operación sobre el perfil.
        command: ExecutionProfileCommand,
    },
}

/// Operaciones transaccionales del perfil del ejecutor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionProfileCommand {
    /// Crea una propuesta, nunca activa directamente.
    Import {
        /// Borrador JSON.
        file: String,
    },
    /// Muestra activo, hash y propuestas.
    Status,
    /// Activa por CAS y confirmación.
    Apply {
        /// Identificador de propuesta.
        proposal: String,
        /// Hash del perfil activo sobre el que se hizo staging.
        expected_hash: String,
        /// Confirmación explícita.
        confirm: bool,
    },
}

pub(super) fn parse_grant(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(missing("grant", "<create|status|revoke>"));
    };
    let command = match subcommand.as_str() {
        "create" => parse_grant_create(&args[1..])?,
        "status" => GrantCommand::Status {
            id: one_argument(&args[1..], "grant status", "<id>")?,
        },
        "revoke" => parse_grant_revoke(&args[1..])?,
        _ => return Err(unexpected("grant", subcommand)),
    };
    Ok(Command::Grant { command })
}

fn parse_grant_create(args: &[String]) -> Result<GrantCommand, CliError> {
    let mut file = None;
    let mut confirm = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--file" => {
                file = Some(flag_value(args, &mut index, "--file")?);
            }
            "--confirm" => confirm = true,
            other => return Err(unknown_flag(other, &["--file", "--confirm"])),
        }
        index += 1;
    }
    Ok(GrantCommand::Create {
        file: file.ok_or(CliError::MissingFlag { flag: "--file" })?,
        confirm,
    })
}

fn parse_grant_revoke(args: &[String]) -> Result<GrantCommand, CliError> {
    let Some(id) = args.first() else {
        return Err(missing("grant revoke", "<id>"));
    };
    match &args[1..] {
        [] => Ok(GrantCommand::Revoke {
            id: id.clone(),
            confirm: false,
        }),
        [flag] if flag == "--confirm" => Ok(GrantCommand::Revoke {
            id: id.clone(),
            confirm: true,
        }),
        [flag, ..] if flag.starts_with('-') => Err(unknown_flag(flag, &["--confirm"])),
        [extra, ..] => Err(unexpected("grant revoke", extra)),
    }
}

pub(super) fn parse_run(args: &[String]) -> Result<Command, CliError> {
    let command = match args {
        [] => RunCommand::Start { file: None },
        [flag, file] if flag == "--file" => RunCommand::Start {
            file: Some(file.clone()),
        },
        [flag] if flag == "--file" => {
            return Err(CliError::MissingValue { flag: flag.clone() });
        }
        [subcommand, id] if subcommand == "status" => RunCommand::Status { id: id.clone() },
        [subcommand, id] if subcommand == "resume" => RunCommand::Resume { id: id.clone() },
        [flag, ..] if flag.starts_with('-') => return Err(unknown_flag(flag, &["--file"])),
        [subcommand] if subcommand == "status" || subcommand == "resume" => {
            return Err(missing("run", "<id>"));
        }
        [extra, ..] => return Err(unexpected("run", extra)),
    };
    Ok(Command::Run { command })
}

pub(super) fn parse_executor(args: &[String]) -> Result<Command, CliError> {
    let Some(namespace) = args.first() else {
        return Err(missing("executor", "profile"));
    };
    if namespace != "profile" {
        return Err(unexpected("executor", namespace));
    }
    let command = parse_profile(&args[1..])?;
    Ok(Command::Executor {
        command: ExecutorCommand::Profile { command },
    })
}

fn parse_profile(args: &[String]) -> Result<ExecutionProfileCommand, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(missing("executor profile", "<import|status|apply>"));
    };
    match subcommand.as_str() {
        "status" if args.len() == 1 => Ok(ExecutionProfileCommand::Status),
        "status" => Err(unexpected("executor profile status", &args[1])),
        "import" => parse_profile_import(&args[1..]),
        "apply" => parse_profile_apply(&args[1..]),
        _ => Err(unexpected("executor profile", subcommand)),
    }
}

fn parse_profile_import(args: &[String]) -> Result<ExecutionProfileCommand, CliError> {
    match args {
        [flag, file] if flag == "--file" => {
            Ok(ExecutionProfileCommand::Import { file: file.clone() })
        }
        [flag] if flag == "--file" => Err(CliError::MissingValue { flag: flag.clone() }),
        [flag, ..] if flag.starts_with('-') => Err(unknown_flag(flag, &["--file"])),
        [] => Err(CliError::MissingFlag { flag: "--file" }),
        [extra, ..] => Err(unexpected("executor profile import", extra)),
    }
}

fn parse_profile_apply(args: &[String]) -> Result<ExecutionProfileCommand, CliError> {
    let Some(proposal) = args.first() else {
        return Err(missing("executor profile apply", "<proposal-id>"));
    };
    let mut expected_hash = None;
    let mut confirm = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--expected-hash" => {
                expected_hash = Some(flag_value(args, &mut index, "--expected-hash")?);
            }
            "--confirm" => confirm = true,
            other => return Err(unknown_flag(other, &["--expected-hash", "--confirm"])),
        }
        index += 1;
    }
    Ok(ExecutionProfileCommand::Apply {
        proposal: proposal.clone(),
        expected_hash: expected_hash.ok_or(CliError::MissingFlag {
            flag: "--expected-hash",
        })?,
        confirm,
    })
}

fn flag_value(args: &[String], index: &mut usize, flag: &'static str) -> Result<String, CliError> {
    *index += 1;
    let Some(value) = args.get(*index) else {
        return Err(CliError::MissingValue {
            flag: flag.to_string(),
        });
    };
    if value.starts_with('-') {
        return Err(CliError::MissingValue {
            flag: flag.to_string(),
        });
    }
    Ok(value.clone())
}
fn one_argument(
    args: &[String],
    command: &'static str,
    name: &'static str,
) -> Result<String, CliError> {
    match args {
        [value] => Ok(value.clone()),
        [] => Err(missing(command, name)),
        [_, extra, ..] => Err(unexpected(command, extra)),
    }
}

fn missing(command: &'static str, argument: &'static str) -> CliError {
    CliError::MissingArgument { command, argument }
}

fn unexpected(command: &'static str, value: &str) -> CliError {
    CliError::UnexpectedArgument {
        command,
        given: value.to_string(),
    }
}

fn unknown_flag(given: &str, available: &[&'static str]) -> CliError {
    CliError::UnknownFlag {
        given: given.to_string(),
        available: available.to_vec(),
    }
}
