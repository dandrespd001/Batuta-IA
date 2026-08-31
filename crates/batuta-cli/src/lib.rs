//! La línea de órdenes de batuta.
//!
//! La lógica vive aquí y no en `main.rs` para que se pueda probar desde fuera
//! sin lanzar un proceso: un test que sólo mira el código de salida de un binario
//! no puede decir **por qué** falló, y «falló» no es un diagnóstico.

pub mod api;
pub mod args;
pub mod command;
pub mod declaracion;
pub mod eleccion;
pub mod error;
pub mod mcp;
pub mod operational;
pub mod operational_cli;
pub mod panel;
pub mod paths;
pub mod research;
pub mod tui;

pub use api::{ApiError, ApiErrorV2, ApiResponseV2, decision_html, decision_table, route_json};
pub use args::{
    CANARY_FLAGS, CANARY_SWITCHES, COMMANDS, CatalogCommand, Command, ExecutionProfileCommand,
    ExecutorCommand, GrantCommand, PANEL_FLAGS, ResearchCommand, ResearchScope, RunCommand, USAGE,
    parse,
};
pub use batuta_routing::{ApplicationService, RouteRequestEnvelopeV2, RouteRequestV2};
pub use command::{CanaryOutcome, canary, canary_all, canary_capability, canary_capability_all};
pub use declaracion::{
    anexar_modelo, nuevo_modelo, nuevo_proveedor, plantilla_proveedor, quitar_modelo,
    quitar_modelo_de,
};
pub use eleccion::{disable, effort, enable};
pub use error::CliError;
pub use mcp::McpServer;
pub use operational::{OperationalApi, run_json, run_resume, run_status};
pub use operational_cli::{execute_grant_command, execute_profile_command, execute_run_command};
pub use panel::{Fila, escribir_html, filas, tabla, tabla_html};
pub use paths::Layout;
pub use research::{apply_research, queue_research_update, research_status_json};
pub use tui::{
    RunPreviewV2, TuiApp, TuiExecutionJob, TuiExecutionSection, TuiExecutionWorker, TuiInputAction,
    TuiView, run_tui,
};
