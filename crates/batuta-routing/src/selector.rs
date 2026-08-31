use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::{OverrideEvent, QualityProjection};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Ámbito en el que una ruta puede usarse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteClass {
    /// Trabajo real sujeto a todas las puertas de evidencia.
    Production,
    /// Prueba o canario; no promueve la ruta por sí solo.
    ProbeTest,
}

/// Tolerancia de calidad solicitada; no es incertidumbre del benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SelectionMargin(f64);

impl SelectionMargin {
    /// Construye un margen finito dentro de `0..100`.
    ///
    /// # Errors
    ///
    /// Si el valor no es finito o cae fuera del rango.
    pub fn new(value: f64) -> Result<Self, SelectError> {
        if value.is_finite() && (0.0..=100.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SelectError::invalid(format!(
                "selection_margin must be finite 0..=100; received {value}"
            )))
        }
    }

    /// Valor porcentual.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Una ruta con los datos que el selector necesita, ya leídos de sus fuentes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCandidate {
    /// Ruta exacta.
    pub route: RouteRef,
    /// Alias que se mostrará en la decisión.
    pub alias: Option<String>,
    /// Elección persistida.
    pub enabled: bool,
    /// Producción o pruebas.
    pub class: RouteClass,
    /// Capacidades demostradas y vigentes.
    pub capabilities: BTreeSet<Capability>,
    /// Techo de sensibilidad.
    pub max_sensitivity: Sensitivity,
    /// Ventana de contexto utilizable.
    pub context_window: u64,
    /// Esfuerzos que esta ruta puede honrar.
    pub supported_efforts: BTreeSet<ReasoningEffort>,
    /// Proyección de la acción solicitada.
    pub quality: QualityProjection,
    /// Peso relativo por token.
    pub relative_cost: f64,
    /// Coste estimado de tener que relevarla.
    pub handoff_penalty: f64,
    /// Tasa reciente `0..1`.
    pub recent_success_rate: f64,
    /// Latencia p95 usada como desempate.
    pub latency_p95_ms: u64,
    /// Hasta cuándo no debe seleccionarse.
    pub cooldown_until: Option<u64>,
    /// Si está en la lista de fallbacks aprobados.
    pub approved_fallback: bool,
}

/// Petición resuelta; los valores omitidos ya se tomaron del perfil de acción.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequest {
    /// Versión del documento.
    pub schema_version: u16,
    /// Acción.
    pub action: String,
    /// Capacidades necesarias.
    pub required_capabilities: BTreeSet<Capability>,
    /// Sensibilidad del material.
    pub sensitivity: Sensitivity,
    /// Contexto mínimo.
    pub required_context: u64,
    /// Esfuerzo exigido, si existe.
    pub effort: Option<ReasoningEffort>,
    /// Umbral efectivo.
    pub minimum_quality: f64,
    /// Cercanía admitida respecto de `Qmax`.
    pub selection_margin: SelectionMargin,
    /// Tokens previstos.
    pub predicted_tokens: u64,
    /// Solicitudes externas y permisos efectivos ya verificados.
    pub authorizations: SelectionAuthorizations,
    /// Si esta selección ocurre tras un fallo.
    pub fallback: bool,
    /// Producción o sonda.
    pub class: RouteClass,
    /// Instante determinista de la decisión, en segundos Unix.
    pub now: u64,
}

/// Valores de routing que pertenecen al perfil de una acción.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingActionProfile {
    /// Acción exacta a la que se aplica.
    pub action: String,
    /// Calidad mínima predeterminada.
    pub minimum_quality: f64,
    /// Margen predeterminado.
    pub selection_margin: SelectionMargin,
    /// Autorización persistente de fallbacks no listados.
    pub allow_any_eligible: bool,
    /// Autorización persistente de calidad no verificada.
    pub allow_unverified_quality: bool,
}

/// Petición externa: sólo las perillas del perfil pueden omitirse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequestDraft {
    /// Versión del documento.
    pub schema_version: u16,
    /// Acción solicitada.
    pub action: String,
    /// Capacidades necesarias.
    pub required_capabilities: BTreeSet<Capability>,
    /// Sensibilidad del material.
    pub sensitivity: Sensitivity,
    /// Contexto mínimo.
    pub required_context: u64,
    /// Esfuerzo pedido.
    pub effort: Option<ReasoningEffort>,
    /// Override del umbral del perfil.
    pub minimum_quality: Option<f64>,
    /// Override del margen del perfil.
    pub selection_margin: Option<SelectionMargin>,
    /// Tokens previstos.
    pub predicted_tokens: u64,
    /// Override por petición de fallback.
    pub allow_any_eligible: Option<bool>,
    /// Override por petición de verificación.
    pub allow_unverified_quality: Option<bool>,
    /// Si se está eligiendo fallback.
    pub fallback: bool,
    /// Producción o sonda.
    pub class: RouteClass,
    /// Instante de decisión.
    pub now: u64,
}

impl RouteRequestDraft {
    /// Resuelve únicamente los campos omitibles contra un perfil de la misma acción.
    ///
    /// # Errors
    ///
    /// Si la acción no coincide o el resultado no cumple el esquema.
    pub fn resolve(self, profile: &RoutingActionProfile) -> Result<RouteRequest, SelectError> {
        if self.action != profile.action {
            return Err(SelectError::invalid(format!(
                "request action '{}' does not match profile action '{}'",
                self.action, profile.action
            )));
        }
        let resolved = RouteRequest {
            schema_version: self.schema_version,
            action: self.action,
            required_capabilities: self.required_capabilities,
            sensitivity: self.sensitivity,
            required_context: self.required_context,
            effort: self.effort,
            minimum_quality: self.minimum_quality.unwrap_or(profile.minimum_quality),
            selection_margin: self.selection_margin.unwrap_or(profile.selection_margin),
            predicted_tokens: self.predicted_tokens,
            authorizations: SelectionAuthorizations {
                allow_any_eligible: AuthorizationDecision {
                    requested: self.allow_any_eligible.unwrap_or(false),
                    permitted: self.allow_any_eligible.unwrap_or(false)
                        && profile.allow_any_eligible,
                },
                allow_unverified_quality: AuthorizationDecision {
                    requested: self.allow_unverified_quality.unwrap_or(false),
                    permitted: self.allow_unverified_quality.unwrap_or(false)
                        && profile.allow_unverified_quality,
                },
            },
            fallback: self.fallback,
            class: self.class,
            now: self.now,
        };
        validate_request(&resolved)?;
        Ok(resolved)
    }
}

/// Motivo puro por el que una ruta no ganó.
#[derive(Debug, Clone, PartialEq)]
pub enum DiscardReason {
    /// La política la apagó.
    Disabled,
    /// Clase producción/prueba incompatible.
    ClassMismatch,
    /// Falta una capacidad.
    MissingCapability {
        /// Capacidad ausente.
        capability: Capability,
    },
    /// El techo de sensibilidad no alcanza.
    SensitivityTooHigh,
    /// La ventana de contexto no alcanza.
    ContextTooSmall {
        /// Contexto utilizable.
        available: u64,
        /// Contexto pedido.
        required: u64,
    },
    /// La ruta no puede honrar el esfuerzo.
    UnsupportedEffort {
        /// Esfuerzo pedido.
        effort: ReasoningEffort,
    },
    /// La ruta sigue enfriándose.
    Cooldown {
        /// Fin del cooldown.
        until: u64,
    },
    /// La proyección corresponde a otra acción o ruta.
    ProjectionMismatch,
    /// No hay puntaje efectivo.
    NoQualityScore,
    /// Hay puntaje, pero no evidencia suficiente para producción.
    UnverifiedQuality,
    /// No llega al umbral absoluto.
    BelowMinimum {
        /// Puntaje.
        score: f64,
        /// Umbral.
        minimum: f64,
    },
    /// No está aprobada como fallback.
    UnapprovedFallback,
    /// Quedó fuera de `Qmax - margin`.
    OutsideSelectionMargin {
        /// Puntaje.
        score: f64,
        /// Piso calculado.
        floor: f64,
    },
    /// Era elegible, pero otra ruta tuvo menor coste esperado.
    HigherExpectedCost,
    /// Costes o tasa de éxito no cumplen el contrato.
    InvalidEconomics,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicDiscardReason {
    code: String,
    field: String,
    message: String,
    details: BTreeMap<String, serde_json::Value>,
}

impl DiscardReason {
    fn public_parts(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Disabled => (
                "disabled",
                "policy.routes.enabled",
                "route is disabled by policy",
            ),
            Self::ClassMismatch => (
                "class_mismatch",
                "request.class",
                "route class is incompatible with the request",
            ),
            Self::MissingCapability { .. } => (
                "missing_capability",
                "task.required_capabilities",
                "route does not provide a required capability",
            ),
            Self::SensitivityTooHigh => (
                "sensitivity_too_high",
                "task.sensitivity",
                "route sensitivity ceiling is too low",
            ),
            Self::ContextTooSmall { .. } => (
                "context_too_small",
                "task.minimum_context",
                "route context window is too small",
            ),
            Self::UnsupportedEffort { .. } => (
                "unsupported_effort",
                "task.reasoning_effort",
                "route does not support the requested reasoning effort",
            ),
            Self::Cooldown { .. } => ("cooldown", "health.cooldown_until", "route is cooling down"),
            Self::ProjectionMismatch => (
                "projection_mismatch",
                "evidence.projections",
                "quality projection does not match the route and action",
            ),
            Self::NoQualityScore => (
                "no_quality_score",
                "evidence.projections.effective_score",
                "route has no effective quality score",
            ),
            Self::UnverifiedQuality => (
                "unverified_quality",
                "evidence.projections.coverage",
                "route quality is not independently verified",
            ),
            Self::BelowMinimum { .. } => (
                "below_minimum",
                "policy.profiles.minimum_quality",
                "route quality is below the absolute minimum",
            ),
            Self::UnapprovedFallback => (
                "unapproved_fallback",
                "policy.routes.approved_fallback",
                "route is not approved as fallback",
            ),
            Self::OutsideSelectionMargin { .. } => (
                "outside_selection_margin",
                "policy.profiles.selection_margin",
                "route is outside the quality selection margin",
            ),
            Self::HigherExpectedCost => (
                "higher_expected_cost",
                "catalog.routes.costs",
                "another eligible route has lower expected cost",
            ),
            Self::InvalidEconomics => (
                "invalid_economics",
                "catalog.routes.costs",
                "route economics are unknown, non-finite, or invalid",
            ),
        }
    }

    fn public_details(&self) -> BTreeMap<String, serde_json::Value> {
        let mut details = BTreeMap::new();
        match self {
            Self::MissingCapability { capability } => {
                details.insert("capability".to_string(), serde_json::json!(capability));
            }
            Self::ContextTooSmall {
                available,
                required,
            } => {
                details.insert("available".to_string(), serde_json::json!(available));
                details.insert("required".to_string(), serde_json::json!(required));
            }
            Self::UnsupportedEffort { effort } => {
                details.insert("effort".to_string(), serde_json::json!(effort));
            }
            Self::Cooldown { until } => {
                details.insert("until".to_string(), serde_json::json!(until));
            }
            Self::BelowMinimum { score, minimum } => {
                details.insert("minimum".to_string(), serde_json::json!(minimum));
                details.insert("score".to_string(), serde_json::json!(score));
            }
            Self::OutsideSelectionMargin { score, floor } => {
                details.insert("floor".to_string(), serde_json::json!(floor));
                details.insert("score".to_string(), serde_json::json!(score));
            }
            Self::Disabled
            | Self::ClassMismatch
            | Self::SensitivityTooHigh
            | Self::ProjectionMismatch
            | Self::NoQualityScore
            | Self::UnverifiedQuality
            | Self::UnapprovedFallback
            | Self::HigherExpectedCost
            | Self::InvalidEconomics => {}
        }
        details
    }

    fn from_public(mut public: PublicDiscardReason) -> Result<Self, String> {
        let reason = match public.code.as_str() {
            "disabled" => Self::Disabled,
            "class_mismatch" => Self::ClassMismatch,
            "missing_capability" => Self::MissingCapability {
                capability: take_detail(&mut public.details, "capability")?,
            },
            "sensitivity_too_high" => Self::SensitivityTooHigh,
            "context_too_small" => Self::ContextTooSmall {
                available: take_detail(&mut public.details, "available")?,
                required: take_detail(&mut public.details, "required")?,
            },
            "unsupported_effort" => Self::UnsupportedEffort {
                effort: take_detail(&mut public.details, "effort")?,
            },
            "cooldown" => Self::Cooldown {
                until: take_detail(&mut public.details, "until")?,
            },
            "projection_mismatch" => Self::ProjectionMismatch,
            "no_quality_score" => Self::NoQualityScore,
            "unverified_quality" => Self::UnverifiedQuality,
            "below_minimum" => Self::BelowMinimum {
                score: take_detail(&mut public.details, "score")?,
                minimum: take_detail(&mut public.details, "minimum")?,
            },
            "unapproved_fallback" => Self::UnapprovedFallback,
            "outside_selection_margin" => Self::OutsideSelectionMargin {
                score: take_detail(&mut public.details, "score")?,
                floor: take_detail(&mut public.details, "floor")?,
            },
            "higher_expected_cost" => Self::HigherExpectedCost,
            "invalid_economics" => Self::InvalidEconomics,
            code => return Err(format!("unknown discard code: {code}")),
        };
        if !public.details.is_empty() {
            return Err("unknown discard detail".to_string());
        }
        let (_, field, message) = reason.public_parts();
        if public.field != field || public.message != message {
            return Err("discard field or message does not match code".to_string());
        }
        Ok(reason)
    }
}

impl Serialize for DiscardReason {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (code, field, message) = self.public_parts();
        PublicDiscardReason {
            code: code.to_string(),
            field: field.to_string(),
            message: message.to_string(),
            details: self.public_details(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiscardReason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let public = PublicDiscardReason::deserialize(deserializer)?;
        Self::from_public(public).map_err(serde::de::Error::custom)
    }
}

fn take_detail<T: serde::de::DeserializeOwned>(
    details: &mut BTreeMap<String, serde_json::Value>,
    field: &str,
) -> Result<T, String> {
    let value = details
        .remove(field)
        .ok_or_else(|| format!("missing discard detail: {field}"))?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}

/// Ruta descartada y todas sus razones independientes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscardedRoute {
    /// Ruta evaluada.
    pub route: RouteRef,
    /// Razones acumuladas.
    pub reasons: Vec<DiscardReason>,
}

/// Una autorización solicitada y su decisión confiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// El cliente pidió la excepción.
    pub requested: bool,
    /// La política permitió la solicitud.
    pub permitted: bool,
}

/// Autorizaciones extraordinarias conservadas en el recibo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionAuthorizations {
    /// Cualquier fallback elegible.
    pub allow_any_eligible: AuthorizationDecision,
    /// Salto de la puerta de verificación.
    pub allow_unverified_quality: AuthorizationDecision,
}

/// Hashes confiables de la única generación usada por una selección.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSealV2 {
    /// Hash del manifest leído una sola vez.
    pub manifest_hash: String,
    /// Hash del catálogo.
    pub catalog_hash: String,
    /// Hash de política.
    pub policy_hash: String,
    /// Hash de evidencia activa.
    pub evidence_hash: String,
    /// Hash de salud.
    pub health_hash: String,
    /// Hash del índice de capacidades.
    pub capabilities_hash: String,
    /// Recibos exactos utilizados, ordenados por hash.
    pub capability_receipt_hashes: BTreeSet<String>,
}

impl DecisionSealV2 {
    pub(crate) fn legacy(policy_hash: &str) -> Self {
        let empty = format!("sha256:{}", "0".repeat(64));
        Self {
            manifest_hash: empty.clone(),
            catalog_hash: empty.clone(),
            policy_hash: policy_hash.to_string(),
            evidence_hash: empty.clone(),
            health_hash: empty.clone(),
            capabilities_hash: empty,
            capability_receipt_hashes: BTreeSet::new(),
        }
    }
}

/// Resultado único y reproducible del selector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteDecision {
    /// Versión del documento.
    pub schema_version: u16,
    /// Ruta escogida.
    pub route: RouteRef,
    /// Alias humano, si existía.
    pub alias: Option<String>,
    /// Puntaje derivado de evidencia antes de cualquier override.
    pub researched_score: Option<f64>,
    /// Puntaje efectivo.
    pub effective_score: f64,
    /// Override auditado, si produjo el puntaje efectivo.
    pub manual_override: Option<OverrideEvent>,
    /// Cobertura investigada.
    pub coverage: u8,
    /// Estado de verificación.
    pub verified: bool,
    /// Coste esperado usado en el orden.
    pub expected_cost: f64,
    /// Hash de la evidencia usada.
    pub evidence_hash: String,
    /// Hash de la política usada.
    pub policy_hash: String,
    /// Hash del manifest activo.
    pub manifest_hash: String,
    /// Hash del catálogo activo.
    pub catalog_hash: String,
    /// Hash de salud activa.
    pub health_hash: String,
    /// Hash del índice de capacidades.
    pub capabilities_hash: String,
    /// Hashes ordenados de recibos de capacidad usados.
    pub capability_receipt_hashes: Vec<String>,
    /// Rutas no elegidas y explicación.
    pub discarded: Vec<DiscardedRoute>,
    /// Autorizaciones extraordinarias.
    pub authorizations: SelectionAuthorizations,
}

/// Ninguna ruta pasó todas las puertas o la petición era inválida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectError {
    /// Código estable.
    pub code: String,
    /// Explicación.
    pub message: String,
    /// Descartes, también cuando no queda ninguna.
    pub discarded: Vec<DiscardedRoute>,
}

impl SelectError {
    pub(crate) fn invalid(message: String) -> Self {
        Self {
            code: "invalid_route_request".to_string(),
            message,
            discarded: Vec::new(),
        }
    }
}

impl fmt::Display for SelectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SelectError {}

struct Eligible<'a> {
    candidate: &'a RouteCandidate,
    score: f64,
    expected_cost: f64,
}

/// Aplica todas las puertas, el margen y el coste esperado.
///
/// # Errors
///
/// Si la petición es inválida o ninguna ruta queda elegible.
pub fn select(
    request: &RouteRequest,
    candidates: &[RouteCandidate],
    policy_hash: &str,
) -> Result<RouteDecision, SelectError> {
    select_sealed(request, candidates, &DecisionSealV2::legacy(policy_hash))
}

/// Selecciona y sella todos los componentes de la generación usada.
///
/// # Errors
///
/// Si la petición o los hashes son inválidos, o ninguna ruta queda elegible.
pub fn select_sealed(
    request: &RouteRequest,
    candidates: &[RouteCandidate],
    seal: &DecisionSealV2,
) -> Result<RouteDecision, SelectError> {
    validate_request(request)?;
    validate_seal(seal)?;
    let mut discarded = Vec::new();
    let mut eligible = Vec::new();

    for candidate in candidates {
        let mut reasons = filter_reasons(request, candidate);
        let score = candidate.quality.effective_score;
        if reasons.is_empty()
            && let Some(score) = score
        {
            let expected_cost = expected_cost(request, candidate);
            if expected_cost.is_finite() {
                eligible.push(Eligible {
                    candidate,
                    score,
                    expected_cost,
                });
            } else {
                reasons.push(DiscardReason::InvalidEconomics);
            }
        }
        if !reasons.is_empty() {
            discarded.push(DiscardedRoute {
                route: candidate.route.clone(),
                reasons,
            });
        }
    }

    if eligible.is_empty() {
        return Err(SelectError {
            code: "no_eligible_route".to_string(),
            message: "no route passed capability, evidence, policy and health filters".to_string(),
            discarded,
        });
    }

    let qmax = eligible
        .iter()
        .skip(1)
        .fold(eligible[0].score, |maximum, item| maximum.max(item.score));
    let floor = request
        .minimum_quality
        .max(qmax - request.selection_margin.get());
    let mut finalists = Vec::new();
    for item in eligible {
        if item.score < floor {
            discarded.push(DiscardedRoute {
                route: item.candidate.route.clone(),
                reasons: vec![DiscardReason::OutsideSelectionMargin {
                    score: item.score,
                    floor,
                }],
            });
        } else {
            finalists.push(item);
        }
    }
    finalists.sort_by(|left, right| {
        left.expected_cost
            .total_cmp(&right.expected_cost)
            .then_with(|| {
                left.candidate
                    .latency_p95_ms
                    .cmp(&right.candidate.latency_p95_ms)
            })
            .then_with(|| left.candidate.route.cmp(&right.candidate.route))
    });
    let selected = finalists.remove(0);
    for item in finalists {
        discarded.push(DiscardedRoute {
            route: item.candidate.route.clone(),
            reasons: vec![DiscardReason::HigherExpectedCost],
        });
    }
    discarded.sort_by(|left, right| left.route.cmp(&right.route));

    Ok(RouteDecision {
        schema_version: request.schema_version,
        route: selected.candidate.route.clone(),
        alias: selected.candidate.alias.clone(),
        researched_score: selected.candidate.quality.researched_score,
        effective_score: selected.score,
        manual_override: selected.candidate.quality.active_override.clone(),
        coverage: selected.candidate.quality.coverage,
        verified: selected.candidate.quality.verified,
        expected_cost: selected.expected_cost,
        evidence_hash: selected.candidate.quality.evidence_hash.clone(),
        policy_hash: seal.policy_hash.clone(),
        manifest_hash: seal.manifest_hash.clone(),
        catalog_hash: seal.catalog_hash.clone(),
        health_hash: seal.health_hash.clone(),
        capabilities_hash: seal.capabilities_hash.clone(),
        capability_receipt_hashes: seal.capability_receipt_hashes.iter().cloned().collect(),
        discarded,
        authorizations: request.authorizations,
    })
}

fn validate_seal(seal: &DecisionSealV2) -> Result<(), SelectError> {
    let legacy_empty = format!("sha256:{}", "0".repeat(64));
    if seal.manifest_hash == legacy_empty
        && seal.catalog_hash == legacy_empty
        && seal.evidence_hash == legacy_empty
        && seal.health_hash == legacy_empty
        && seal.capabilities_hash == legacy_empty
        && seal.capability_receipt_hashes.is_empty()
        && !seal.policy_hash.trim().is_empty()
    {
        return Ok(());
    }
    for (field, hash) in [
        ("manifest_hash", &seal.manifest_hash),
        ("catalog_hash", &seal.catalog_hash),
        ("policy_hash", &seal.policy_hash),
        ("evidence_hash", &seal.evidence_hash),
        ("health_hash", &seal.health_hash),
        ("capabilities_hash", &seal.capabilities_hash),
    ] {
        let Some(hex) = hash.strip_prefix("sha256:") else {
            return Err(SelectError::invalid(format!(
                "{field} must be sha256:<hex>"
            )));
        };
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SelectError::invalid(format!(
                "{field} must be sha256:<hex>"
            )));
        }
    }
    for hash in &seal.capability_receipt_hashes {
        let Some(hex) = hash.strip_prefix("sha256:") else {
            return Err(SelectError::invalid(
                "capability receipt hash must be sha256:<hex>".to_string(),
            ));
        };
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SelectError::invalid(
                "capability receipt hash must be sha256:<hex>".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_request(request: &RouteRequest) -> Result<(), SelectError> {
    if !matches!(request.schema_version, 1 | 2) {
        return Err(SelectError::invalid(format!(
            "schema_version {} is unsupported; supported: 1, 2",
            request.schema_version
        )));
    }
    if request.action.trim().is_empty() {
        return Err(SelectError::invalid("action cannot be empty".to_string()));
    }
    if request.predicted_tokens > u64::from(u32::MAX) {
        return Err(SelectError::invalid(format!(
            "predicted_tokens must fit u32 for deterministic costing; received {}",
            request.predicted_tokens
        )));
    }
    if !request.minimum_quality.is_finite() || !(0.0..=100.0).contains(&request.minimum_quality) {
        return Err(SelectError::invalid(format!(
            "minimum_quality must be finite 0..=100; received {}",
            request.minimum_quality
        )));
    }
    let margin = request.selection_margin.get();
    if !margin.is_finite() || !(0.0..=100.0).contains(&margin) {
        return Err(SelectError::invalid(format!(
            "selection_margin must be finite 0..=100; received {margin}"
        )));
    }
    Ok(())
}

fn filter_reasons(request: &RouteRequest, candidate: &RouteCandidate) -> Vec<DiscardReason> {
    let mut reasons = Vec::new();
    if !candidate.enabled {
        reasons.push(DiscardReason::Disabled);
    }
    if candidate.class != request.class {
        reasons.push(DiscardReason::ClassMismatch);
    }
    for capability in &request.required_capabilities {
        if !candidate.capabilities.contains(capability) {
            reasons.push(DiscardReason::MissingCapability {
                capability: *capability,
            });
        }
    }
    if !request.sensitivity.fits_within(candidate.max_sensitivity) {
        reasons.push(DiscardReason::SensitivityTooHigh);
    }
    if candidate.context_window < request.required_context {
        reasons.push(DiscardReason::ContextTooSmall {
            available: candidate.context_window,
            required: request.required_context,
        });
    }
    if let Some(effort) = request.effort
        && !candidate.supported_efforts.contains(&effort)
    {
        reasons.push(DiscardReason::UnsupportedEffort { effort });
    }
    if let Some(until) = candidate.cooldown_until
        && until > request.now
    {
        reasons.push(DiscardReason::Cooldown { until });
    }
    if candidate.quality.route != candidate.route || candidate.quality.action != request.action {
        reasons.push(DiscardReason::ProjectionMismatch);
    }
    match candidate.quality.effective_score {
        None => reasons.push(DiscardReason::NoQualityScore),
        Some(score) if score < request.minimum_quality => {
            reasons.push(DiscardReason::BelowMinimum {
                score,
                minimum: request.minimum_quality,
            });
        }
        Some(_) => {}
    }
    if !candidate.quality.verified && !request.authorizations.allow_unverified_quality.permitted {
        reasons.push(DiscardReason::UnverifiedQuality);
    }
    if request.fallback
        && !candidate.approved_fallback
        && !request.authorizations.allow_any_eligible.permitted
    {
        reasons.push(DiscardReason::UnapprovedFallback);
    }
    if !candidate.relative_cost.is_finite()
        || candidate.relative_cost < 0.0
        || !candidate.handoff_penalty.is_finite()
        || candidate.handoff_penalty < 0.0
        || !candidate.recent_success_rate.is_finite()
        || !(0.0..=1.0).contains(&candidate.recent_success_rate)
    {
        reasons.push(DiscardReason::InvalidEconomics);
    }
    reasons
}

fn expected_cost(request: &RouteRequest, candidate: &RouteCandidate) -> f64 {
    let tokens = u32::try_from(request.predicted_tokens).map_or(f64::INFINITY, f64::from);
    let base = tokens * candidate.relative_cost + candidate.handoff_penalty;
    base / candidate.recent_success_rate.max(0.01)
}
