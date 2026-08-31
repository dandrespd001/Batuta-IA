//! Contrato JSON compartido por CLI, MCP y pruebas de superficie.

use std::fmt;

use batuta_routing::{ApplicationService, RouteDecision, RouteRequestEnvelopeV2, SelectError};
use serde::{Deserialize, Serialize};

/// Error JSON estable y cerrado de todas las superficies v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiErrorV2 {
    /// Versión del contrato de error.
    pub schema_version: u16,
    /// Código para máquinas.
    pub code: String,
    /// Campo responsable; cadena vacía cuando no corresponde a una entrada.
    pub field: String,
    /// Mensaje para personas.
    pub message: String,
    /// Detalle estructurado.
    pub details: serde_json::Value,
}

/// Alias conservado para las superficies v2 anteriores.
pub type ApiError = ApiErrorV2;

/// Sobre uniforme de éxito para todas las superficies v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiResponseV2<T> {
    /// Versión del sobre, independiente de la versión del documento interno.
    pub schema_version: u16,
    /// Documento de respuesta.
    pub data: T,
}

impl ApiErrorV2 {
    /// Construye un error con todos sus campos contractuales.
    pub fn new(
        code: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            schema_version: 2,
            code: code.into(),
            field: field.into(),
            message: message.into(),
            details,
        }
    }

    /// Construye usando el campo estable asociado a un código heredado.
    pub fn for_code(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        let code = code.into();
        let field = error_field(&code);
        Self::new(code, field, message, details)
    }
}

fn error_field(code: &str) -> &'static str {
    match code {
        "invalid_json" | "invalid_json_rpc" => "document",
        "invalid_route_request" | "invalid_request" => "request",
        "routing_state_required" => "state.manifest",
        "serialization_error" => "response",
        "unknown_tool" => "params.name",
        _ => "",
    }
}

impl fmt::Display for ApiErrorV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiErrorV2 {}

impl From<SelectError> for ApiErrorV2 {
    fn from(error: SelectError) -> Self {
        let details = serde_json::to_value(&error.discarded).unwrap_or(serde_json::Value::Null);
        Self::new(error.code, "request", error.message, details)
    }
}

/// Ejecuta el mismo selector que MCP y devuelve la decisión en el sobre v2.
///
/// # Errors
///
/// Devuelve un error versionado cuando el JSON, el esquema o la selección no son válidos.
pub fn route_json(service: &ApplicationService, input: &str) -> Result<String, ApiError> {
    let envelope: RouteRequestEnvelopeV2 = serde_json::from_str(input).map_err(|error| {
        ApiError::for_code(
            "invalid_json",
            error.to_string(),
            serde_json::json!({"document": "routing_envelope"}),
        )
    })?;
    let decision = route(service, envelope)?;
    serde_json::to_string(&ApiResponseV2 {
        schema_version: 2,
        data: decision,
    })
    .map_err(|error| {
        ApiError::for_code(
            "serialization_error",
            error.to_string(),
            serde_json::Value::Null,
        )
    })
}

pub(crate) fn route(
    service: &ApplicationService,
    envelope: RouteRequestEnvelopeV2,
) -> Result<RouteDecision, ApiError> {
    service.route(envelope).map_err(Into::into)
}

/// Render textual determinista de la misma decisión serializada por CLI y MCP.
pub fn decision_table(decision: &RouteDecision) -> String {
    let researched = decision
        .researched_score
        .map_or_else(|| "sin puntaje".to_string(), |score| score.to_string());
    let manual = decision.manual_override.as_ref().map_or_else(
        || "sin override".to_string(),
        |value| {
            format!(
                "{} ({}, {})",
                value
                    .score
                    .map_or_else(|| "clear".to_string(), |score| score.to_string()),
                value.author,
                value.reason
            )
        },
    );
    format!(
        "ruta | investigado | override | efectivo | cobertura | verificada | evidence_hash | policy_hash\n{} | {} | {} | {} | {} | {} | {} | {}",
        decision.route,
        researched,
        manual,
        decision.effective_score,
        decision.coverage,
        decision.verified,
        decision.evidence_hash,
        decision.policy_hash
    )
}

/// Render HTML autocontenido de los mismos campos de [`decision_table`].
pub fn decision_html(decision: &RouteDecision) -> String {
    let table = decision_table(decision);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Batuta routing</title></head><body><p>Decisión de sólo lectura</p><pre>{}</pre></body></html>",
        escape_html(&table)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
