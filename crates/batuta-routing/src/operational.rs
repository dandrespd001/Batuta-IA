//! Contratos operativos sellados para investigación y canarios de efecto.

#![allow(clippy::missing_errors_doc)]

use std::fmt;

use batuta_contract::RouteRef;
use batuta_quality::{BenchmarkObservation, SourceKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::GrantLimits;

/// Fuente primaria completa que sustenta una observación investigada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSourceV2 {
    /// URL primaria publicada.
    pub source_url: String,
    /// Publicación responsable.
    pub publication: String,
    /// Consulta reproducible usada para encontrar la fuente.
    pub query: String,
    /// Benchmark citado.
    pub benchmark: String,
    /// Versión del benchmark.
    pub benchmark_version: String,
    /// Escenario evaluado.
    pub scenario: String,
    /// Configuración evaluada.
    pub configuration: String,
    /// Ruta exacta evaluada.
    pub route: RouteRef,
    /// Revisión exacta evaluada.
    pub model_revision: String,
    /// Métrica publicada.
    pub metric: String,
    /// Tipo e independencia de la fuente.
    pub source_kind: SourceKind,
}

/// Propuesta de investigación sellada contra estado, evidencia y grant base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchProposalV2 {
    /// Versión cerrada del contrato.
    pub schema_version: u16,
    /// Identificador inmutable de staging.
    pub id: String,
    /// Instante de creación Unix.
    pub created_at: u64,
    /// Ruta exacta que ejecutó la investigación.
    pub researcher_route: RouteRef,
    /// Grant que autorizó la operación `research`.
    pub grant_id: String,
    /// Manifest activo al investigar.
    pub base_manifest_hash: String,
    /// Evidencia activa al investigar.
    pub base_evidence_hash: String,
    /// Observaciones completas propuestas.
    pub observations: Vec<BenchmarkObservation>,
    /// Fuentes primarias completas.
    pub sources: Vec<ResearchSourceV2>,
    /// Hash canónico del resto del documento.
    pub proposal_hash: String,
}

#[derive(Serialize)]
struct ResearchProposalBody<'a> {
    schema_version: u16,
    id: &'a str,
    created_at: u64,
    researcher_route: &'a RouteRef,
    grant_id: &'a str,
    base_manifest_hash: &'a str,
    base_evidence_hash: &'a str,
    observations: &'a [BenchmarkObservation],
    sources: &'a [ResearchSourceV2],
}

impl ResearchProposalV2 {
    /// Construye, valida, ordena y sella una propuesta sin mutar evidencia activa.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        created_at: u64,
        researcher_route: RouteRef,
        grant_id: impl Into<String>,
        base_manifest_hash: String,
        base_evidence_hash: String,
        mut observations: Vec<BenchmarkObservation>,
        mut sources: Vec<ResearchSourceV2>,
    ) -> Result<Self, OperationalError> {
        observations.sort_by(|left, right| left.id.cmp(&right.id));
        sources.sort_by(|left, right| {
            (&left.source_url, &left.route, &left.metric).cmp(&(
                &right.source_url,
                &right.route,
                &right.metric,
            ))
        });
        let mut proposal = Self {
            schema_version: 2,
            id: id.into(),
            created_at,
            researcher_route,
            grant_id: grant_id.into(),
            base_manifest_hash,
            base_evidence_hash,
            observations,
            sources,
            proposal_hash: String::new(),
        };
        proposal.validate_content()?;
        proposal.proposal_hash = proposal.calculate_hash()?;
        Ok(proposal)
    }

    /// Verifica el conflicto triple antes de aplicar: manifest, evidencia y contenido.
    pub fn validate_apply(
        &self,
        current_manifest_hash: &str,
        current_evidence_hash: &str,
    ) -> Result<(), OperationalError> {
        self.validate_content()?;
        if self.base_manifest_hash != current_manifest_hash {
            return Err(OperationalError::Conflict("manifest"));
        }
        if self.base_evidence_hash != current_evidence_hash {
            return Err(OperationalError::Conflict("evidence"));
        }
        if self.proposal_hash != self.calculate_hash()? {
            return Err(OperationalError::Conflict("proposal_hash"));
        }
        Ok(())
    }

    fn validate_content(&self) -> Result<(), OperationalError> {
        validate_text("id", &self.id)?;
        validate_text("grant_id", &self.grant_id)?;
        validate_hash("base_manifest_hash", &self.base_manifest_hash)?;
        validate_hash("base_evidence_hash", &self.base_evidence_hash)?;
        if self.observations.is_empty() || self.sources.is_empty() {
            return Err(OperationalError::Invalid(
                "observations and sources must be non-empty".to_string(),
            ));
        }
        for observation in &self.observations {
            if observation.route == self.researcher_route {
                return Err(OperationalError::Invalid(
                    "researcher_route cannot certify itself".to_string(),
                ));
            }
            if !self.sources.iter().any(|source| {
                source.route == observation.route
                    && source.source_url == observation.source_url
                    && source.benchmark == observation.benchmark
                    && source.benchmark_version == observation.benchmark_version
                    && source.scenario == observation.scenario
                    && source.configuration == observation.configuration
                    && source.model_revision == observation.model_revision
                    && source.metric == observation.metric
                    && source.source_kind == observation.source_kind
            }) {
                return Err(OperationalError::Invalid(format!(
                    "observation {} has no exact source",
                    observation.id
                )));
            }
        }
        for source in &self.sources {
            validate_source(source)?;
        }
        Ok(())
    }

    fn calculate_hash(&self) -> Result<String, OperationalError> {
        canonical_hash(&ResearchProposalBody {
            schema_version: self.schema_version,
            id: &self.id,
            created_at: self.created_at,
            researcher_route: &self.researcher_route,
            grant_id: &self.grant_id,
            base_manifest_hash: &self.base_manifest_hash,
            base_evidence_hash: &self.base_evidence_hash,
            observations: &self.observations,
            sources: &self.sources,
        })
    }
}

/// Evento de herramienta observado; una mención textual nunca crea un evento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolEventV2 {
    /// Herramienta invocada.
    pub tool: String,
    /// Resultado conocido de la invocación.
    pub success: bool,
    /// Resultado textual acotado a 4096 bytes.
    pub result: Option<String>,
    /// Digest del resultado completo.
    pub result_digest: Option<String>,
    /// Artefacto observado, si aplica.
    pub artifact: Option<String>,
    /// URL observada, si aplica.
    pub source_url: Option<String>,
    /// Estado HTTP observado, si aplica.
    pub source_status: Option<u16>,
}

/// Escenario exacto de un canario de capacidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanaryScenarioV2 {
    /// Lee un nonce externo al prompt.
    Read,
    /// Escribe un artefacto exacto en el directorio permitido.
    Write,
    /// Ejecuta una herramienta con resultado verificable.
    Tools,
    /// Observa una URL y su contenido.
    Web,
}

/// Efectos verificados fuera de la respuesta textual del modelo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryEffectsV2 {
    /// Nonce escrito fuera del prompt.
    pub expected_nonce: Option<String>,
    /// Respuesta exacta observada.
    pub response: Option<String>,
    /// Ruta relativa allowlisted esperada.
    pub expected_artifact: Option<String>,
    /// Ruta relativa realmente creada.
    pub observed_artifact: Option<String>,
    /// Digest esperado del contenido.
    pub expected_artifact_digest: Option<String>,
    /// Digest observado del contenido.
    pub observed_artifact_digest: Option<String>,
    /// Escrituras fuera del artefacto permitido.
    pub lateral_writes: Vec<String>,
    /// Fuente web observada.
    pub source_url: Option<String>,
    /// Estado HTTP observado.
    pub source_status: Option<u16>,
    /// Digest del contenido web observado.
    pub source_digest: Option<String>,
}

/// Recibo sellado de un canario con límites y efectos verificables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCanaryReceiptV2 {
    /// Versión cerrada del contrato.
    pub schema_version: u16,
    /// Ruta exacta canariada.
    pub route: RouteRef,
    /// Revisión exacta canariada.
    pub model_revision: String,
    /// Escenario ejecutado.
    pub scenario: CanaryScenarioV2,
    /// Manifest usado.
    pub manifest_hash: String,
    /// Grant consumido.
    pub grant_id: String,
    /// Límites de esa ejecución.
    pub limits: GrantLimits,
    /// Eventos observados y ordenados.
    pub tool_events: Vec<ToolEventV2>,
    /// Efectos observados fuera de la prosa.
    pub effects: CanaryEffectsV2,
    /// Vencimiento del recibo.
    pub expires_at: u64,
    /// Resultado positivo de la validación estructural.
    pub positive: bool,
    /// Hash canónico del recibo.
    pub receipt_hash: String,
}

#[derive(Serialize)]
struct CanaryReceiptBody<'a> {
    schema_version: u16,
    route: &'a RouteRef,
    model_revision: &'a str,
    scenario: CanaryScenarioV2,
    manifest_hash: &'a str,
    grant_id: &'a str,
    limits: GrantLimits,
    tool_events: &'a [ToolEventV2],
    effects: &'a CanaryEffectsV2,
    expires_at: u64,
    positive: bool,
}

impl CapabilityCanaryReceiptV2 {
    /// Valida efectos exactos y sella únicamente un recibo positivo.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        route: RouteRef,
        model_revision: impl Into<String>,
        scenario: CanaryScenarioV2,
        manifest_hash: String,
        grant_id: impl Into<String>,
        limits: GrantLimits,
        mut tool_events: Vec<ToolEventV2>,
        mut effects: CanaryEffectsV2,
        expires_at: u64,
    ) -> Result<Self, OperationalError> {
        tool_events.sort_by(|left, right| {
            (&left.tool, &left.artifact, &left.source_url).cmp(&(
                &right.tool,
                &right.artifact,
                &right.source_url,
            ))
        });
        effects.lateral_writes.sort();
        let mut receipt = Self {
            schema_version: 2,
            route,
            model_revision: model_revision.into(),
            scenario,
            manifest_hash,
            grant_id: grant_id.into(),
            limits,
            tool_events,
            effects,
            expires_at,
            positive: true,
            receipt_hash: String::new(),
        };
        receipt.validate_effects()?;
        receipt.receipt_hash = receipt.calculate_hash()?;
        Ok(receipt)
    }

    /// Indica si el recibo sigue siendo positivo, íntegro y vigente.
    #[must_use]
    pub fn is_positive_at(&self, now: u64) -> bool {
        self.positive
            && now < self.expires_at
            && self.validate_effects().is_ok()
            && self
                .calculate_hash()
                .is_ok_and(|hash| hash == self.receipt_hash)
    }

    fn validate_effects(&self) -> Result<(), OperationalError> {
        validate_text("model_revision", &self.model_revision)?;
        validate_text("grant_id", &self.grant_id)?;
        validate_hash("manifest_hash", &self.manifest_hash)?;
        self.limits
            .validate()
            .map_err(|error| OperationalError::Invalid(error.to_string()))?;
        if self.expires_at == 0 {
            return Err(OperationalError::Invalid(
                "expires_at must be positive".to_string(),
            ));
        }
        let successful = self.tool_events.iter().filter(|event| event.success);
        if successful.clone().next().is_none() {
            return Err(OperationalError::Invalid(
                "a successful tool event is required".to_string(),
            ));
        }
        for event in &self.tool_events {
            validate_text("tool_events.tool", &event.tool)?;
            if event
                .result
                .as_ref()
                .is_some_and(|result| result.len() > 4_096)
            {
                return Err(OperationalError::Invalid(
                    "tool event result exceeds 4096 bytes".to_string(),
                ));
            }
            if let Some(digest) = &event.result_digest {
                validate_hash("tool_events.result_digest", digest)?;
            }
        }
        match self.scenario {
            CanaryScenarioV2::Tools => {
                if !successful
                    .clone()
                    .any(|event| event.result_digest.is_some())
                {
                    return Err(OperationalError::Invalid(
                        "tools requires a successful result digest".to_string(),
                    ));
                }
            }
            CanaryScenarioV2::Read => {
                if self.effects.expected_nonce.is_none()
                    || self.effects.expected_nonce != self.effects.response
                {
                    return Err(OperationalError::Invalid(
                        "read requires the exact external nonce".to_string(),
                    ));
                }
            }
            CanaryScenarioV2::Write => {
                if self.effects.expected_artifact.is_none()
                    || self.effects.expected_artifact != self.effects.observed_artifact
                    || self.effects.expected_artifact_digest.is_none()
                    || self.effects.expected_artifact_digest
                        != self.effects.observed_artifact_digest
                    || !self.effects.lateral_writes.is_empty()
                {
                    return Err(OperationalError::Invalid(
                        "write requires one exact artifact and no lateral writes".to_string(),
                    ));
                }
                validate_hash(
                    "effects.expected_artifact_digest",
                    self.effects
                        .expected_artifact_digest
                        .as_deref()
                        .unwrap_or_default(),
                )?;
            }
            CanaryScenarioV2::Web => {
                let Some(url) = self.effects.source_url.as_deref() else {
                    return Err(OperationalError::Invalid(
                        "web requires a source URL".to_string(),
                    ));
                };
                validate_url(url)?;
                if !matches!(self.effects.source_status, Some(200..=399)) {
                    return Err(OperationalError::Invalid(
                        "web requires a successful source status".to_string(),
                    ));
                }
                validate_hash(
                    "effects.source_digest",
                    self.effects.source_digest.as_deref().unwrap_or_default(),
                )?;
                if !successful.clone().any(|event| {
                    event.source_url.as_deref() == Some(url)
                        && event.source_status == self.effects.source_status
                }) {
                    return Err(OperationalError::Invalid(
                        "web source must come from a successful tool event".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn calculate_hash(&self) -> Result<String, OperationalError> {
        canonical_hash(&CanaryReceiptBody {
            schema_version: self.schema_version,
            route: &self.route,
            model_revision: &self.model_revision,
            scenario: self.scenario,
            manifest_hash: &self.manifest_hash,
            grant_id: &self.grant_id,
            limits: self.limits,
            tool_events: &self.tool_events,
            effects: &self.effects,
            expires_at: self.expires_at,
            positive: self.positive,
        })
    }
}

/// Error estable de validación o conflicto operacional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationalError {
    /// Documento inválido.
    Invalid(String),
    /// Base o contenido cambió antes de aplicar.
    Conflict(&'static str),
    /// No pudo producirse la representación canónica.
    Serialization(String),
}

impl fmt::Display for OperationalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid_operational_document: {message}"),
            Self::Conflict(field) => write!(formatter, "stale_operational_base: {field}"),
            Self::Serialization(message) => {
                write!(formatter, "operational_serialization_failed: {message}")
            }
        }
    }
}

impl std::error::Error for OperationalError {}

fn validate_source(source: &ResearchSourceV2) -> Result<(), OperationalError> {
    validate_url(&source.source_url)?;
    for (field, value) in [
        ("publication", source.publication.as_str()),
        ("query", source.query.as_str()),
        ("benchmark", source.benchmark.as_str()),
        ("benchmark_version", source.benchmark_version.as_str()),
        ("scenario", source.scenario.as_str()),
        ("configuration", source.configuration.as_str()),
        ("model_revision", source.model_revision.as_str()),
        ("metric", source.metric.as_str()),
    ] {
        validate_text(field, value)?;
    }
    Ok(())
}

fn validate_text(field: &str, value: &str) -> Result<(), OperationalError> {
    if value.trim().is_empty() || value.len() > 4_096 {
        return Err(OperationalError::Invalid(format!(
            "{field} must be non-empty and bounded"
        )));
    }
    Ok(())
}

fn validate_url(value: &str) -> Result<(), OperationalError> {
    if !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err(OperationalError::Invalid(
            "source_url must be http(s)".to_string(),
        ));
    }
    Ok(())
}

fn validate_hash(field: &str, value: &str) -> Result<(), OperationalError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(OperationalError::Invalid(format!(
            "{field} must be sha256:<hex>"
        )));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(OperationalError::Invalid(format!(
            "{field} must be sha256:<hex>"
        )));
    }
    Ok(())
}

fn canonical_hash(value: &impl Serialize) -> Result<String, OperationalError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| OperationalError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
