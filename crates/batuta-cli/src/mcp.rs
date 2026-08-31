//! MCP JSON-RPC 2.0 por stdio.

use std::io::{BufRead as _, Write as _};

use serde::Deserialize;

use batuta_routing::{ApplicationService, RouteRequestEnvelopeV2};

use crate::api::{ApiError, ApiResponseV2, route};

/// Servidor sin sockets ni estado conversacional.
pub struct McpServer;

#[derive(Deserialize)]
struct Request {
    jsonrpc: String,
    #[serde(default)]
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

impl McpServer {
    /// Atiende una petición JSON-RPC completa.
    ///
    /// # Errors
    ///
    /// Devuelve un error de API si la entrada no es JSON válido o no puede serializarse.
    pub fn handle_line(line: &str) -> Result<String, ApiError> {
        Self::handle_line_inner(line, None)
    }

    /// Atiende una petición con una foto local ya fijada por el proceso.
    ///
    /// # Errors
    ///
    /// Devuelve un error de API si la entrada no es válida.
    pub fn handle_line_with_service(
        line: &str,
        service: &ApplicationService,
    ) -> Result<String, ApiError> {
        Self::handle_line_inner(line, Some(service))
    }

    fn handle_line_inner(
        line: &str,
        service: Option<&ApplicationService>,
    ) -> Result<String, ApiError> {
        let request: Request = serde_json::from_str(line).map_err(|error| {
            ApiError::for_code(
                "invalid_json_rpc",
                error.to_string(),
                serde_json::Value::Null,
            )
        })?;
        if request.jsonrpc != "2.0" {
            return Ok(error_response(
                &request.id,
                -32_600,
                "jsonrpc must be '2.0'",
                &serde_json::Value::Null,
            ));
        }
        let result = match request.method.as_str() {
            "initialize" => serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "batuta", "version": env!("CARGO_PKG_VERSION")}
            }),
            "ping" => serde_json::json!({}),
            "tools/list" => tools_list(),
            "tools/call" => match tool_call(&request.params, service) {
                Ok(result) => result,
                Err(error) => {
                    return Ok(error_response(
                        &request.id,
                        -32_602,
                        &error.message,
                        &serde_json::to_value(&error).unwrap_or(serde_json::Value::Null),
                    ));
                }
            },
            method => {
                return Ok(error_response(
                    &request.id,
                    -32_601,
                    &format!("unknown method '{method}'"),
                    &serde_json::Value::Null,
                ));
            }
        };
        serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": request.id,
            "result": result
        }))
        .map_err(|error| serialization_api_error(&error))
    }

    /// Bucle stdio: una petición y una respuesta por línea.
    ///
    /// # Errors
    ///
    /// Devuelve un error si falla stdio o una petición no puede procesarse.
    pub fn serve_stdio(service: &ApplicationService) -> Result<(), ApiError> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line.map_err(|error| io_api_error(&error))?;
            let response = Self::handle_line_with_service(&line, service)?;
            writeln!(stdout, "{response}").map_err(|error| io_api_error(&error))?;
            stdout.flush().map_err(|error| io_api_error(&error))?;
        }
        Ok(())
    }
}

fn tools_list() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "batuta.route",
                "description": "Simula routing sin ejecutar ni gastar.",
                "inputSchema": {"type": "object"}
            },
        ]
    })
}

fn tool_call(
    params: &serde_json::Value,
    service: Option<&ApplicationService>,
) -> Result<serde_json::Value, ApiError> {
    let name = params
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ApiError::new(
                "invalid_tool_call",
                "params.name",
                "tools/call requires params.name",
                params.clone(),
            )
        })?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let structured = match name {
        "batuta.route" => {
            let envelope: RouteRequestEnvelopeV2 =
                serde_json::from_value(arguments).map_err(|error| {
                    ApiError::for_code(
                        "invalid_route_request",
                        error.to_string(),
                        serde_json::Value::Null,
                    )
                })?;
            let service = service.ok_or_else(|| {
                ApiError::for_code(
                    "routing_state_required",
                    "batuta.route requires an active StateManifestV2",
                    serde_json::Value::Null,
                )
            })?;
            serde_json::to_value(ApiResponseV2 {
                schema_version: 2,
                data: route(service, envelope)?,
            })
            .map_err(|error| serialization_api_error(&error))?
        }
        _ => {
            return Err(ApiError::for_code(
                "unknown_tool",
                format!("unknown MCP tool '{name}'"),
                serde_json::Value::Null,
            ));
        }
    };
    let text =
        serde_json::to_string(&structured).map_err(|error| serialization_api_error(&error))?;
    Ok(serde_json::json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false
    }))
}

fn error_response(
    id: &serde_json::Value,
    code: i32,
    message: &str,
    data: &serde_json::Value,
) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message, "data": data}
    })
    .to_string()
}

fn io_api_error(error: &std::io::Error) -> ApiError {
    ApiError::new(
        "stdio_error",
        "stdio",
        error.to_string(),
        serde_json::Value::Null,
    )
}

fn serialization_api_error(error: &serde_json::Error) -> ApiError {
    ApiError::for_code(
        "serialization_error",
        error.to_string(),
        serde_json::Value::Null,
    )
}
