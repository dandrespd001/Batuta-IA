//! Adaptación de fichero/stdin y ejecutor confiable para las órdenes K4.

use batuta_exec::ManifestHarnessExecutor;
use batuta_routing::ExecutionProfileStore;
use serde::Serialize;

use crate::{
    ApiErrorV2, ExecutionProfileCommand, GrantCommand, Layout, OperationalApi, RunCommand,
    run_json, run_resume, run_status,
};

/// Ejecuta una suborden `grant` y devuelve exclusivamente JSON contractual.
///
/// # Errors
///
/// Si no puede leerse la entrada o la operación de grant falla.
pub fn execute_grant_command(
    layout: &Layout,
    command: &GrantCommand,
) -> Result<String, ApiErrorV2> {
    let api = OperationalApi::new(layout, now_secs());
    match command {
        GrantCommand::Create { file, confirm } => {
            let input = read_file(file)?;
            serialize(&api.grant_create_json(&input, *confirm)?)
        }
        GrantCommand::Status { id } => serialize(&api.grant_status(id)?),
        GrantCommand::Revoke { id, confirm } => serialize(&api.grant_revoke(id, *confirm)?),
    }
}

/// Ejecuta `executor profile` sin permitir configurar ubicaciones ni comandos.
///
/// # Errors
///
/// Si no puede leerse la entrada o staging, CAS o publicación fallan.
pub fn execute_profile_command(
    layout: &Layout,
    command: &ExecutionProfileCommand,
) -> Result<String, ApiErrorV2> {
    let api = OperationalApi::new(layout, now_secs());
    match command {
        ExecutionProfileCommand::Import { file } => {
            let input = read_file(file)?;
            serialize(&api.profile_import_json(&input)?)
        }
        ExecutionProfileCommand::Status => serialize(&api.profile_status()?),
        ExecutionProfileCommand::Apply {
            proposal,
            expected_hash,
            confirm,
        } => serialize(&api.profile_apply(proposal, expected_hash, *confirm)?),
    }
}

/// Ejecuta una suborden `run`; sólo `Start` sin fichero consume `input`.
///
/// # Errors
///
/// Si faltan perfil o manifests confiables, la entrada falla o el coordinador
/// no puede iniciar, consultar o continuar la corrida.
pub fn execute_run_command(
    layout: &Layout,
    command: &RunCommand,
    input: &mut dyn std::io::Read,
) -> Result<String, ApiErrorV2> {
    match command {
        RunCommand::Status { id } => serialize(&run_status(layout, id)?),
        RunCommand::Start { file } => {
            let request = match file {
                Some(file) => read_file(file)?,
                None => read_input(input)?,
            };
            let executor = open_executor(layout)?;
            serialize(&run_json(layout, &executor, &request)?)
        }
        RunCommand::Resume { id } => {
            let executor = open_executor(layout)?;
            serialize(&run_resume(layout, &executor, id)?)
        }
    }
}

fn open_executor(layout: &Layout) -> Result<ManifestHarnessExecutor, ApiErrorV2> {
    let profile = ExecutionProfileStore::open(layout.execution_profile(), layout.leases())
        .status()
        .map_err(|error| {
            ApiErrorV2::new(
                "execution_profile_error",
                "profile",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?
        .active
        .ok_or_else(|| {
            ApiErrorV2::new(
                "execution_profile_required",
                "profile",
                "an active confirmed ExecutionProfileV1 is required",
                serde_json::Value::Null,
            )
        })?;
    let manifests = layout.trusted_manifests().map_err(|error| {
        ApiErrorV2::new(
            "manifest_layout_error",
            "state.manifests",
            error.to_string(),
            serde_json::Value::Null,
        )
    })?;
    ManifestHarnessExecutor::open(&manifests, profile, layout.invocations()).map_err(|error| {
        ApiErrorV2::new(
            "manifest_executor_error",
            "state.manifests",
            error.to_string(),
            serde_json::Value::Null,
        )
    })
}

fn read_file(path: &str) -> Result<String, ApiErrorV2> {
    std::fs::read_to_string(path).map_err(|error| {
        ApiErrorV2::new(
            "io_error",
            "file",
            error.to_string(),
            serde_json::json!({"path": path}),
        )
    })
}

fn read_input(input: &mut dyn std::io::Read) -> Result<String, ApiErrorV2> {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer).map_err(|error| {
        ApiErrorV2::new(
            "io_error",
            "stdin",
            error.to_string(),
            serde_json::Value::Null,
        )
    })?;
    Ok(buffer)
}

fn serialize<T: Serialize>(response: &T) -> Result<String, ApiErrorV2> {
    serde_json::to_string(response).map_err(|error| {
        ApiErrorV2::new(
            "serialization_error",
            "response",
            error.to_string(),
            serde_json::Value::Null,
        )
    })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
