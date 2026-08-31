//! Parseo de catálogo, investigación, routing y TUI.

use crate::error::CliError;

use super::{CatalogCommand, Command, ResearchCommand, ResearchScope};

pub(super) fn parsear_catalog(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(CliError::MissingArgument {
            command: "catalog",
            argument: "<import|status|apply>",
        });
    };
    match subcommand.as_str() {
        "status" => sin_argumentos(
            &args[1..],
            "catalog status",
            Command::Catalog {
                command: CatalogCommand::Status,
            },
        ),
        "import" => parsear_import_catalog(&args[1..]),
        "apply" => parsear_apply_catalog(&args[1..]),
        given => Err(CliError::UnknownFlag {
            given: given.to_string(),
            available: vec!["import", "status", "apply"],
        }),
    }
}

fn parsear_import_catalog(args: &[String]) -> Result<Command, CliError> {
    match args {
        [flag, file] if flag == "--file" => Ok(Command::Catalog {
            command: CatalogCommand::Import {
                file: Some(file.clone()),
            },
        }),
        [flag] if flag == "--file" => Err(CliError::MissingValue { flag: flag.clone() }),
        [] => Ok(Command::Catalog {
            command: CatalogCommand::Import { file: None },
        }),
        [given, ..] if given.starts_with("--") => Err(CliError::UnknownFlag {
            given: given.clone(),
            available: vec!["--file"],
        }),
        [given, ..] => Err(CliError::UnexpectedArgument {
            command: "catalog import",
            given: given.clone(),
        }),
    }
}

fn parsear_apply_catalog(args: &[String]) -> Result<Command, CliError> {
    match args {
        [proposal, flag] if flag == "--confirm" => Ok(Command::Catalog {
            command: CatalogCommand::Apply {
                proposal: proposal.clone(),
                confirm: true,
            },
        }),
        [proposal] => Ok(Command::Catalog {
            command: CatalogCommand::Apply {
                proposal: proposal.clone(),
                confirm: false,
            },
        }),
        [] => Err(CliError::MissingArgument {
            command: "catalog apply",
            argument: "<propuesta>",
        }),
        [_, given, ..] => Err(CliError::UnexpectedArgument {
            command: "catalog apply",
            given: given.clone(),
        }),
    }
}

pub(super) fn parsear_tui(args: &[String]) -> Result<Command, CliError> {
    match args {
        [] => Ok(Command::Tui { route_file: None }),
        [flag, value] if flag == "--route" => Ok(Command::Tui {
            route_file: Some(value.clone()),
        }),
        [flag] if flag == "--route" => Err(CliError::MissingValue { flag: flag.clone() }),
        [given, ..] if given.starts_with("--") => Err(CliError::UnknownFlag {
            given: given.clone(),
            available: vec!["--route"],
        }),
        [given, ..] => Err(CliError::UnexpectedArgument {
            command: "tui",
            given: given.clone(),
        }),
    }
}

pub(super) fn parsear_route(args: &[String]) -> Result<Command, CliError> {
    match args {
        [] => Ok(Command::Route {
            json: None,
            file: None,
        }),
        [flag, value] if flag == "--json" => Ok(Command::Route {
            json: Some(value.clone()),
            file: None,
        }),
        [flag, value] if flag == "--file" => Ok(Command::Route {
            json: None,
            file: Some(value.clone()),
        }),
        [flag] if flag == "--json" || flag == "--file" => {
            Err(CliError::MissingValue { flag: flag.clone() })
        }
        [given, ..] if given.starts_with("--") => Err(CliError::UnknownFlag {
            given: given.clone(),
            available: vec!["--json", "--file"],
        }),
        [given, ..] => Err(CliError::UnexpectedArgument {
            command: "route",
            given: given.clone(),
        }),
    }
}

pub(super) fn parsear_research(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(CliError::MissingArgument {
            command: "research",
            argument: "<update|status|apply>",
        });
    };
    match subcommand.as_str() {
        "status" => sin_argumentos(
            &args[1..],
            "research status",
            Command::Research {
                command: ResearchCommand::Status,
            },
        ),
        "update" => parsear_research_update(&args[1..]),
        "apply" => parsear_research_apply(&args[1..]),
        given => Err(CliError::UnknownFlag {
            given: given.to_string(),
            available: vec!["update", "status", "apply"],
        }),
    }
}

fn parsear_research_update(args: &[String]) -> Result<Command, CliError> {
    let scope = match args {
        [flag] if flag == "--all" => ResearchScope::All,
        [flag, value] if flag == "--route" => ResearchScope::Route(value.clone()),
        [flag, value] if flag == "--action" => ResearchScope::Action(value.clone()),
        [] => {
            return Err(CliError::MissingArgument {
                command: "research update",
                argument: "<--all|--route <ruta>|--action <acción>>",
            });
        }
        [flag] if flag == "--route" || flag == "--action" => {
            return Err(CliError::MissingValue { flag: flag.clone() });
        }
        [given, ..] if given.starts_with("--") => {
            return Err(CliError::UnknownFlag {
                given: given.clone(),
                available: vec!["--all", "--route", "--action"],
            });
        }
        [given, ..] => {
            return Err(CliError::UnexpectedArgument {
                command: "research update",
                given: given.clone(),
            });
        }
    };
    Ok(Command::Research {
        command: ResearchCommand::Update { scope },
    })
}

fn parsear_research_apply(args: &[String]) -> Result<Command, CliError> {
    match args {
        [proposal, flag] if flag == "--confirm" => Ok(Command::Research {
            command: ResearchCommand::Apply {
                proposal: proposal.clone(),
                confirm: true,
            },
        }),
        [proposal] => Ok(Command::Research {
            command: ResearchCommand::Apply {
                proposal: proposal.clone(),
                confirm: false,
            },
        }),
        [] => Err(CliError::MissingArgument {
            command: "research apply",
            argument: "<propuesta>",
        }),
        [_, given, ..] => Err(CliError::UnexpectedArgument {
            command: "research apply",
            given: given.clone(),
        }),
    }
}

pub(super) fn sin_argumentos(
    args: &[String],
    command: &'static str,
    parsed: Command,
) -> Result<Command, CliError> {
    match args {
        [] => Ok(parsed),
        [given, ..] => Err(CliError::UnexpectedArgument {
            command,
            given: given.clone(),
        }),
    }
}
