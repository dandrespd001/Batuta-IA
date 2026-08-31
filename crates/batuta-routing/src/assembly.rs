//! Ensamblado tipado de una generación de estado en una caché de routing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use serde::{Deserialize, Serialize};

use crate::{
    DecisionSealV2, ExecutionPolicyV2, RouteCandidate, RouteClass, RouteHealth,
    RoutingActionProfile, RoutingSnapshot, SelectError, StateSnapshotV2, StateStoreError,
};

/// Metadatos estables descubiertos para una ruta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRouteStateV2 {
    /// Clase actualmente publicada.
    pub class: RouteClass,
    /// Sensibilidad máxima.
    pub max_sensitivity: Sensitivity,
    /// Contexto utilizable.
    pub context_window: u64,
    /// Esfuerzos exactos disponibles.
    pub supported_efforts: BTreeSet<ReasoningEffort>,
}

/// Componente de catálogo v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogStateV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Rutas exactas.
    pub routes: BTreeMap<RouteRef, CatalogRouteStateV2>,
}

/// Perillas de política para una ruta exacta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRouteStateV2 {
    /// Alias humano.
    pub alias: Option<String>,
    /// Habilitación explícita.
    pub enabled: bool,
    /// Coste relativo conocido.
    pub relative_cost: f64,
    /// Penalización esperada de relevo.
    pub handoff_penalty: f64,
    /// Miembro de fallbacks aprobados.
    pub approved_fallback: bool,
}

/// Política consolidada v2 usada por el ensamblador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyStateV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Límites explícitos de retry y relevo.
    pub execution: ExecutionPolicyV2,
    /// Perfiles por acción.
    pub profiles: BTreeMap<String, RoutingActionProfile>,
    /// Configuración por ruta.
    pub routes: BTreeMap<RouteRef, PolicyRouteStateV2>,
}

/// Proyecciones de evidencia ya calculadas y selladas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceStateV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Exactamente una proyección por ruta y acción disponible.
    pub projections: Vec<QualityProjection>,
}

/// Ventanas de salud por ruta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthStateV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Salud exacta.
    pub routes: BTreeMap<RouteRef, RouteHealth>,
}

/// Capacidades demostradas por recibos exactos con una vigencia común.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityIndexEntryV2 {
    /// Capacidades positivas.
    pub capabilities: BTreeSet<Capability>,
    /// Recibos que sustentan la entrada.
    pub receipt_hashes: BTreeSet<String>,
    /// Caducidad exclusiva.
    pub expires_at: u64,
}

/// Índice de capacidades v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityIndexV2 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Entrada por ruta exacta y revisión incluida en `RouteRef`.
    pub routes: BTreeMap<RouteRef, CapabilityIndexEntryV2>,
}

/// Configuración incompleta que no produjo candidato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyDiscard {
    /// Ruta exacta.
    pub route: RouteRef,
    /// Acción, cuando ya se conocía.
    pub action: Option<String>,
    /// Código estable.
    pub code: String,
    /// Campo ausente o inválido.
    pub field: String,
    /// Mensaje estable.
    pub message: String,
    /// Valores relevantes ordenados.
    pub details: BTreeMap<String, String>,
}

/// Caché derivada y descartes estructurados.
#[derive(Debug, Clone)]
pub struct AssemblyReport {
    /// Snapshot que puede usar el servicio.
    pub snapshot: RoutingSnapshot,
    /// Configuraciones que no se convirtieron en candidato.
    pub discarded: Vec<AssemblyDiscard>,
}

/// Ensambla exactamente un candidato por ruta y acción desde una generación.
///
/// # Errors
///
/// Si una versión, perfil, duplicado o sello incumple el contrato.
#[allow(clippy::too_many_lines)] // Mantiene visible el orden único de validación y ensamblado.
pub fn assemble_snapshot(
    state: &StateSnapshotV2,
    now: u64,
) -> Result<AssemblyReport, AssemblyError> {
    validate_versions(state)?;
    let manifest_hash = state
        .manifest
        .manifest_hash()
        .map_err(AssemblyError::State)?;
    let mut projections = BTreeMap::new();
    for projection in &state.components.evidence.projections {
        let key = (projection.route.clone(), projection.action.clone());
        if projections.insert(key, projection).is_some() {
            return Err(AssemblyError::Invalid(
                "duplicate evidence projection for route and action".to_string(),
            ));
        }
    }

    let mut candidates = Vec::new();
    let mut discarded = Vec::new();
    let mut receipt_hashes = BTreeSet::new();
    for (route, catalog) in &state.components.catalog.routes {
        let Some(policy) = state.components.policy.routes.get(route) else {
            discarded.push(discard(
                route,
                None,
                "missing_policy_route",
                "policy.routes",
            ));
            continue;
        };
        let Some(health) = state.components.health.routes.get(route) else {
            discarded.push(discard(
                route,
                None,
                "missing_health_route",
                "health.routes",
            ));
            continue;
        };
        let Some(capability) = state.components.capabilities.routes.get(route) else {
            discarded.push(discard(
                route,
                None,
                "missing_capability_index",
                "capabilities.routes",
            ));
            continue;
        };
        validate_policy_route(route, policy)?;
        let (capabilities, receipts) = if capability.expires_at > now {
            for hash in &capability.receipt_hashes {
                validate_hash(hash)?;
            }
            (
                capability.capabilities.clone(),
                capability.receipt_hashes.clone(),
            )
        } else {
            (BTreeSet::new(), BTreeSet::new())
        };
        receipt_hashes.extend(receipts);
        for action in state.components.policy.profiles.keys() {
            let Some(quality) = projections.get(&(route.clone(), action.clone())) else {
                discarded.push(discard(
                    route,
                    Some(action.clone()),
                    "missing_quality_projection",
                    "evidence.projections",
                ));
                continue;
            };
            candidates.push(RouteCandidate {
                route: route.clone(),
                alias: policy.alias.clone(),
                enabled: policy.enabled,
                class: catalog.class,
                capabilities: capabilities.clone(),
                max_sensitivity: catalog.max_sensitivity,
                context_window: catalog.context_window,
                supported_efforts: catalog.supported_efforts.clone(),
                quality: (*quality).clone(),
                relative_cost: policy.relative_cost,
                handoff_penalty: policy.handoff_penalty,
                recent_success_rate: health.recent_success_rate,
                latency_p95_ms: health.latency_p95_ms,
                cooldown_until: health.cooldown_until,
                approved_fallback: policy.approved_fallback,
            });
        }
    }
    candidates.sort_by(|left, right| {
        (&left.route, &left.quality.action).cmp(&(&right.route, &right.quality.action))
    });
    discarded.sort_by(|left, right| {
        (&left.route, &left.action, &left.code).cmp(&(&right.route, &right.action, &right.code))
    });
    let seal = DecisionSealV2 {
        manifest_hash,
        catalog_hash: state.manifest.catalog_hash.clone(),
        policy_hash: state.manifest.policy_hash.clone(),
        evidence_hash: state.manifest.evidence_hash.clone(),
        health_hash: state.manifest.health_hash.clone(),
        capabilities_hash: state.manifest.capabilities_hash.clone(),
        capability_receipt_hashes: receipt_hashes,
    };
    let snapshot = RoutingSnapshot::new_sealed(
        seal,
        state.components.policy.execution,
        state.components.policy.profiles.clone(),
        candidates,
    )
    .map_err(AssemblyError::Select)?;
    Ok(AssemblyReport {
        snapshot,
        discarded,
    })
}

fn validate_versions(state: &StateSnapshotV2) -> Result<(), AssemblyError> {
    for (name, version) in [
        ("catalog", state.components.catalog.schema_version),
        ("policy", state.components.policy.schema_version),
        ("evidence", state.components.evidence.schema_version),
        ("health", state.components.health.schema_version),
        ("capabilities", state.components.capabilities.schema_version),
    ] {
        if version != 2 {
            return Err(AssemblyError::Invalid(format!(
                "{name}.schema_version must be 2"
            )));
        }
    }
    for (action, profile) in &state.components.policy.profiles {
        if action != &profile.action {
            return Err(AssemblyError::Invalid(format!(
                "profile key '{action}' does not match action '{}'",
                profile.action
            )));
        }
    }
    state
        .components
        .policy
        .execution
        .validate()
        .map_err(|error| AssemblyError::Invalid(error.to_string()))?;
    Ok(())
}

fn validate_policy_route(
    route: &RouteRef,
    policy: &PolicyRouteStateV2,
) -> Result<(), AssemblyError> {
    if !policy.relative_cost.is_finite()
        || policy.relative_cost < 0.0
        || !policy.handoff_penalty.is_finite()
        || policy.handoff_penalty < 0.0
    {
        return Err(AssemblyError::Invalid(format!(
            "invalid economics for route '{route}'"
        )));
    }
    Ok(())
}

fn discard(route: &RouteRef, action: Option<String>, code: &str, field: &str) -> AssemblyDiscard {
    AssemblyDiscard {
        route: route.clone(),
        action,
        code: code.to_string(),
        field: field.to_string(),
        message: format!("route '{route}' has incomplete {field}"),
        details: BTreeMap::new(),
    }
}

fn validate_hash(hash: &str) -> Result<(), AssemblyError> {
    let valid = hash.len() == 71
        && hash.starts_with("sha256:")
        && hash[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(AssemblyError::Invalid(format!(
            "invalid capability receipt hash '{hash}'"
        )))
    }
}

/// Error de carga o ensamblado de una generación.
#[derive(Debug)]
pub enum AssemblyError {
    /// Estado inválido.
    Invalid(String),
    /// Almacén.
    State(StateStoreError),
    /// Selector/caché.
    Select(SelectError),
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::State(error) => write!(f, "{error}"),
            Self::Select(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AssemblyError {}
