//! Adaptadores de salida para grants, perfiles y runs K4.

use std::process::ExitCode;

use batuta_cli::{
    ApiErrorV2, ExecutionProfileCommand, GrantCommand, Layout, RunCommand, execute_grant_command,
    execute_profile_command, execute_run_command,
};

pub(crate) fn ejecutar_grant(command: &GrantCommand) -> ExitCode {
    execute_with_layout(|layout| execute_grant_command(layout, command))
}

pub(crate) fn ejecutar_profile(command: &ExecutionProfileCommand) -> ExitCode {
    execute_with_layout(|layout| execute_profile_command(layout, command))
}

pub(crate) fn ejecutar_run(command: &RunCommand) -> ExitCode {
    execute_with_layout(|layout| {
        let mut stdin = std::io::stdin().lock();
        execute_run_command(layout, command, &mut stdin)
    })
}

fn execute_with_layout(operation: impl FnOnce(&Layout) -> Result<String, ApiErrorV2>) -> ExitCode {
    let result = Layout::from_env()
        .map_err(|error| {
            ApiErrorV2::new(
                "layout_error",
                "state",
                error.to_string(),
                serde_json::Value::Null,
            )
        })
        .and_then(|layout| operation(&layout));
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let output = serde_json::to_string(&error).unwrap_or_else(|_| {
                r#"{"schema_version":2,"code":"serialization_error","field":"response","message":"failed to serialize ApiErrorV2","details":null}"#.to_string()
            });
            eprintln!("{output}");
            ExitCode::from(2)
        }
    }
}
