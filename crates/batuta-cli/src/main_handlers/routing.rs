//! Adaptadores de catálogo, investigación, routing, TUI y MCP.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use batuta_cli::{
    ApplicationService, CatalogCommand, Layout, McpServer, ResearchCommand, apply_research,
    queue_research_update, research_status_json, route_json, run_tui,
};
use batuta_routing::{CatalogStore, DshCatalogBridge, DshSidecarClient, StateStore};

use super::legacy::entorno;

pub(crate) fn ejecutar_catalog(command: &CatalogCommand) -> ExitCode {
    let layout = match Layout::from_env() {
        Ok(layout) => layout,
        Err(error) => {
            eprintln!("batuta: {error}");
            return ExitCode::from(2);
        }
    };
    let store = CatalogStore::open(layout.catalog());
    let result = match command {
        CatalogCommand::Import { file } => import_catalog(file.as_deref()).and_then(|report| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());
            let id = format!("catalog-{now}-{}", std::process::id());
            store
                .stage(&id, now, report.catalog)
                .map(|proposal| {
                    serde_json::json!({
                        "schema_version": 2,
                        "proposal": proposal.id,
                        "expected_active_hash": proposal.expected_active_hash,
                        "proposed_catalog_hash": proposal.proposed_catalog_hash,
                        "rejected": report.rejected,
                        "active_changed": false
                    })
                    .to_string()
                })
                .map_err(|error| error.to_string())
        }),
        CatalogCommand::Status => store
            .status()
            .and_then(|status| {
                serde_json::to_string(&status).map_err(batuta_routing::CatalogStoreError::Json)
            })
            .map_err(|error| error.to_string()),
        CatalogCommand::Apply { proposal, confirm } => store
            .apply(proposal, *confirm)
            .and_then(|_| store.status())
            .and_then(|status| {
                serde_json::to_string(&status).map_err(batuta_routing::CatalogStoreError::Json)
            })
            .map_err(|error| error.to_string()),
    };
    print_result(result)
}

fn import_catalog(file: Option<&str>) -> Result<batuta_routing::CatalogImportReport, String> {
    if let Some(file) = file {
        return std::fs::read_to_string(file)
            .map_err(|error| error.to_string())
            .and_then(|document| {
                DshCatalogBridge::import_json(&document).map_err(|error| error.to_string())
            });
    }
    let module = std::env::var("BATUTA_DSH_CATALOG_MODULE")
        .map_err(|_| "BATUTA_DSH_CATALOG_MODULE is required for sidecar import".to_string())?;
    let script = std::env::var_os("BATUTA_DSH_CATALOG_SIDECAR")
        .map_or_else(|| PathBuf::from("sidecar/dsh_catalog.mjs"), PathBuf::from);
    let path = std::env::var("PATH").unwrap_or_default();
    DshSidecarClient::new(
        PathBuf::from("node"),
        vec![script.display().to_string()],
        std::collections::BTreeMap::from([
            ("PATH".to_string(), path),
            ("BATUTA_DSH_CATALOG_MODULE".to_string(), module),
        ]),
        std::time::Duration::from_secs(10),
        1024 * 1024,
        64 * 1024,
    )
    .map_err(|error| error.to_string())?
    .catalog_snapshot(&format!("catalog-{}", std::process::id()))
    .map_err(|error| error.to_string())
}

pub(crate) fn ejecutar_route(inline: Option<&str>, file: Option<&str>) -> ExitCode {
    let input = match route_input(inline, file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({
                    "schema_version": 2,
                    "code": "input_error",
                    "field": "request",
                    "message": error.to_string(),
                    "details": null
                })
            );
            return ExitCode::from(2);
        }
    };
    let layout = match Layout::from_env() {
        Ok(layout) => layout,
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"schema_version": 2, "code": "state_error", "field": "state", "message": error.to_string(), "details": null})
            );
            return ExitCode::from(2);
        }
    };
    let service = match routing_service(&layout) {
        Ok(service) => service,
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"schema_version": 2, "code": "routing_state_error", "field": "state_manifest", "message": error, "details": null})
            );
            return ExitCode::from(2);
        }
    };
    match route_json(&service, &input) {
        Ok(decision) => {
            println!("{decision}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string(&error).unwrap_or_else(|_| error.to_string())
            );
            ExitCode::from(2)
        }
    }
}

fn route_input(inline: Option<&str>, file: Option<&str>) -> std::io::Result<String> {
    if let Some(json) = inline {
        return Ok(json.to_string());
    }
    if let Some(path) = file {
        return std::fs::read_to_string(path);
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

pub(crate) fn ejecutar_research(command: &ResearchCommand) -> ExitCode {
    let (layout, _) = match entorno() {
        Ok(environment) => environment,
        Err(code) => return code,
    };
    let result = match command {
        ResearchCommand::Update { scope } => queue_research_update(&layout, scope).map(|id| {
            serde_json::json!({
                "schema_version": 1,
                "request": id,
                "status": "staged_request",
                "active_changed": false
            })
            .to_string()
        }),
        ResearchCommand::Status => research_status_json(&layout),
        ResearchCommand::Apply { proposal, confirm } => apply_research(&layout, proposal, *confirm)
            .map(|hash| {
                serde_json::json!({
                    "schema_version": 1,
                    "proposal": proposal,
                    "active_hash": hash
                })
                .to_string()
            }),
    };
    print_result(result)
}

pub(crate) fn ejecutar_tui(route_file: Option<&str>) -> ExitCode {
    let (layout, _) = match entorno() {
        Ok(environment) => environment,
        Err(code) => return code,
    };
    let route_input = match route_file.map(std::fs::read_to_string).transpose() {
        Ok(input) => input,
        Err(error) => {
            eprintln!("batuta: no se pudo leer el routing de la TUI: {error}");
            return ExitCode::from(2);
        }
    };
    let service = match routing_service(&layout) {
        Ok(service) => service,
        Err(error) => {
            eprintln!("batuta: {error}");
            return ExitCode::from(2);
        }
    };
    match run_tui(&layout, &service, route_input.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("batuta: no se pudo abrir la TUI: {error}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn ejecutar_mcp() -> ExitCode {
    let result = Layout::from_env()
        .map_err(|error| error.to_string())
        .and_then(|layout| routing_service(&layout))
        .and_then(|service| McpServer::serve_stdio(&service).map_err(|error| error.to_string()));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}

fn routing_service(layout: &Layout) -> Result<ApplicationService, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    ApplicationService::from_state_store(&StateStore::open(layout.state()), now, false)
        .map_err(|error| error.to_string())
}

fn print_result(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("batuta: {error}");
            ExitCode::from(2)
        }
    }
}
