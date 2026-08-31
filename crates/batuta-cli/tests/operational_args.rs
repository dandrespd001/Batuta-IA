//! Contrato de parseo para grants, corridas y perfil del ejecutor.

use batuta_cli::{
    Command, ExecutionProfileCommand, ExecutorCommand, GrantCommand, RunCommand, parse,
};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn grant_create_status_y_revoke_son_ordenes_cerradas() {
    assert_eq!(
        parse(&args(&[
            "grant",
            "create",
            "--file",
            "grant.json",
            "--confirm",
        ]))
        .unwrap(),
        Command::Grant {
            command: GrantCommand::Create {
                file: "grant.json".to_string(),
                confirm: true,
            },
        }
    );
    assert_eq!(
        parse(&args(&["grant", "status", "grant-1"])).unwrap(),
        Command::Grant {
            command: GrantCommand::Status {
                id: "grant-1".to_string(),
            },
        }
    );
    assert_eq!(
        parse(&args(&["grant", "revoke", "grant-1", "--confirm"])).unwrap(),
        Command::Grant {
            command: GrantCommand::Revoke {
                id: "grant-1".to_string(),
                confirm: true,
            },
        }
    );
}

#[test]
fn run_lee_stdin_o_fichero_y_expone_status_y_resume() {
    assert_eq!(
        parse(&args(&["run"])).unwrap(),
        Command::Run {
            command: RunCommand::Start { file: None },
        }
    );
    assert_eq!(
        parse(&args(&["run", "--file", "run.json"])).unwrap(),
        Command::Run {
            command: RunCommand::Start {
                file: Some("run.json".to_string()),
            },
        }
    );
    assert_eq!(
        parse(&args(&["run", "status", "run-1"])).unwrap(),
        Command::Run {
            command: RunCommand::Status {
                id: "run-1".to_string(),
            },
        }
    );
    assert_eq!(
        parse(&args(&["run", "resume", "run-1"])).unwrap(),
        Command::Run {
            command: RunCommand::Resume {
                id: "run-1".to_string(),
            },
        }
    );
}

#[test]
fn executor_profile_import_status_y_apply_exigen_la_jerarquia_publica() {
    assert_eq!(
        parse(&args(&[
            "executor",
            "profile",
            "import",
            "--file",
            "profile.json",
        ]))
        .unwrap(),
        Command::Executor {
            command: ExecutorCommand::Profile {
                command: ExecutionProfileCommand::Import {
                    file: "profile.json".to_string(),
                },
            },
        }
    );
    assert_eq!(
        parse(&args(&["executor", "profile", "status"])).unwrap(),
        Command::Executor {
            command: ExecutorCommand::Profile {
                command: ExecutionProfileCommand::Status,
            },
        }
    );
    assert_eq!(
        parse(&args(&[
            "executor",
            "profile",
            "apply",
            "proposal-1",
            "--expected-hash",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--confirm",
        ]))
        .unwrap(),
        Command::Executor {
            command: ExecutorCommand::Profile {
                command: ExecutionProfileCommand::Apply {
                    proposal: "proposal-1".to_string(),
                    expected_hash:
                        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    confirm: true,
                },
            },
        }
    );
}

#[test]
fn banderas_operativas_desconocidas_y_valores_ausentes_fallan() {
    assert!(parse(&args(&["grant", "create", "--file"])).is_err());
    assert!(parse(&args(&["run", "--json", "{}"])).is_err());
    assert!(
        parse(&args(&[
            "executor",
            "profile",
            "apply",
            "proposal-1",
            "--confirm",
        ]))
        .is_err()
    );
}
