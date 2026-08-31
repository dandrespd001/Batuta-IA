//! Servicio compartido que mantiene la frontera de confianza del selector.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{Capability, ReasoningEffort, Sensitivity};
use serde::{Deserialize, Serialize};

use crate::{
    AssemblyError, DecisionSealV2, ExecutionPolicyV2, RouteCandidate, RouteClass, RouteDecision,
    RouteRequestDraft, RoutingActionProfile, SelectError, SelectionMargin, StateStore,
    assemble_snapshot, select_sealed,
};

/// Petición pública v2: sólo intención y autorizaciones del llamador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequestV2 {
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
    /// Esfuerzo solicitado.
    pub effort: Option<ReasoningEffort>,
    /// Override opcional del umbral.
    pub minimum_quality: Option<f64>,
    /// Override opcional del margen.
    pub selection_margin: Option<SelectionMargin>,
    /// Tokens previstos.
    pub predicted_tokens: u64,
    /// Solicitud de fallback no listado; la política aún debe permitirlo.
    pub allow_any_eligible: Option<bool>,
    /// Solicitud de calidad no verificada; la política aún debe permitirla.
    pub allow_unverified_quality: Option<bool>,
}

/// Sobre público común a CLI y MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequestEnvelopeV2 {
    /// Versión de esta frontera.
    pub schema_version: u16,
    /// Petición sin candidatos, perfiles, puntajes ni hashes.
    pub request: RouteRequestV2,
}

/// Foto consistente y local de política, perfiles y candidatos ensamblados.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingSnapshot {
    schema_version: u16,
    seal: DecisionSealV2,
    execution_policy: ExecutionPolicyV2,
    profiles: BTreeMap<String, RoutingActionProfile>,
    candidates: Vec<RouteCandidate>,
}

impl RoutingSnapshot {
    /// Construye una foto v2 y comprueba identidades internas antes de publicarla.
    ///
    /// # Errors
    ///
    /// Si un perfil no coincide con su clave, falta el hash o se repite una
    /// proyección de la misma ruta y acción.
    #[allow(clippy::needless_pass_by_value)] // API v1 conservada para llamadas existentes.
    pub fn new(
        policy_hash: String,
        execution_policy: ExecutionPolicyV2,
        profiles: BTreeMap<String, RoutingActionProfile>,
        candidates: Vec<RouteCandidate>,
    ) -> Result<Self, SelectError> {
        let seal = DecisionSealV2::legacy(&policy_hash);
        Self::new_sealed(seal, execution_policy, profiles, candidates)
    }

    /// Construye una caché derivada de una generación sellada.
    ///
    /// # Errors
    ///
    /// Si perfiles o candidatos no son internamente coherentes.
    pub fn new_sealed(
        seal: DecisionSealV2,
        execution_policy: ExecutionPolicyV2,
        profiles: BTreeMap<String, RoutingActionProfile>,
        candidates: Vec<RouteCandidate>,
    ) -> Result<Self, SelectError> {
        execution_policy
            .validate()
            .map_err(|error| SelectError::invalid(error.to_string()))?;
        Self::validate_parts(&seal.policy_hash, &profiles, &candidates)?;
        Ok(Self {
            schema_version: 2,
            seal,
            execution_policy,
            profiles,
            candidates,
        })
    }

    pub(crate) fn validate(self) -> Result<Self, SelectError> {
        if self.schema_version != 2 {
            return Err(SelectError::invalid(format!(
                "routing snapshot schema_version {} is unsupported; supported: 2",
                self.schema_version
            )));
        }
        self.execution_policy
            .validate()
            .map_err(|error| SelectError::invalid(error.to_string()))?;
        Self::validate_parts(&self.seal.policy_hash, &self.profiles, &self.candidates)?;
        Ok(self)
    }

    fn validate_parts(
        policy_hash: &str,
        profiles: &BTreeMap<String, RoutingActionProfile>,
        candidates: &[RouteCandidate],
    ) -> Result<(), SelectError> {
        if policy_hash.trim().is_empty() {
            return Err(SelectError::invalid(
                "policy_hash cannot be empty".to_string(),
            ));
        }
        for (action, profile) in profiles {
            if action != &profile.action {
                return Err(SelectError::invalid(format!(
                    "profile key '{action}' does not match action '{}'",
                    profile.action
                )));
            }
        }
        let mut identities = BTreeSet::new();
        for candidate in candidates {
            let identity = (candidate.route.clone(), candidate.quality.action.clone());
            if !identities.insert(identity) {
                return Err(SelectError::invalid(format!(
                    "duplicate candidate projection for route '{}' and action '{}'",
                    candidate.route, candidate.quality.action
                )));
            }
        }
        Ok(())
    }

    /// Versión validada de la foto.
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Hash de la política que originó esta foto.
    pub fn policy_hash(&self) -> &str {
        &self.seal.policy_hash
    }

    /// Límites de recuperación sellados en esta foto.
    pub const fn execution_policy(&self) -> ExecutionPolicyV2 {
        self.execution_policy
    }
}

/// Fachada pura consumida por todas las superficies.
#[derive(Debug, Clone)]
pub struct ApplicationService {
    snapshot: RoutingSnapshot,
    now: u64,
    class: RouteClass,
    fallback: bool,
}

impl ApplicationService {
    /// Abre el manifest activo una vez y ensambla una caché derivada tipada.
    ///
    /// # Errors
    ///
    /// Si el manifest, un componente o el ensamblado incumplen el contrato.
    pub fn from_state_store(
        store: &StateStore,
        now: u64,
        fallback: bool,
    ) -> Result<Self, AssemblyError> {
        let state = store.load().map_err(AssemblyError::State)?;
        let report = assemble_snapshot(&state, now)?;
        Ok(Self::with_context(
            report.snapshot,
            now,
            RouteClass::Production,
            fallback,
        ))
    }

    /// Fija una única foto y captura el reloj interno para esta instancia.
    pub fn new(snapshot: RoutingSnapshot) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        Self::with_context(snapshot, now, RouteClass::Production, false)
    }

    /// Construye el servicio con contexto confiable explícito.
    ///
    /// Esta superficie es para el coordinador y tests; ninguno de estos campos
    /// forma parte del JSON público.
    pub const fn with_context(
        snapshot: RoutingSnapshot,
        now: u64,
        class: RouteClass,
        fallback: bool,
    ) -> Self {
        Self {
            snapshot,
            now,
            class,
            fallback,
        }
    }

    /// Política explícita usada por las ejecuciones que partan de esta foto.
    ///
    /// # Errors
    ///
    /// Si un snapshot deserializado fue alterado después de construirse.
    pub fn execution_policy(&self) -> Result<ExecutionPolicyV2, SelectError> {
        self.snapshot
            .execution_policy()
            .validate()
            .map_err(|error| SelectError::invalid(error.to_string()))
    }

    /// Resuelve perfil y candidatos desde la foto confiable y selecciona.
    ///
    /// # Errors
    ///
    /// Si el sobre no es v2, no existe el perfil o el selector no encuentra ruta.
    pub fn route(&self, envelope: RouteRequestEnvelopeV2) -> Result<RouteDecision, SelectError> {
        self.route_matching(envelope, |_| true)
    }

    /// Selecciona únicamente en la intersección entre rutas todavía presentes y
    /// rutas exactas autorizadas, excluyendo además las ya intentadas.
    ///
    /// # Errors
    ///
    /// Si el sobre es inválido o la intersección no conserva una ruta elegible.
    pub fn route_with_allowed_routes(
        &self,
        envelope: RouteRequestEnvelopeV2,
        allowed: &BTreeSet<batuta_contract::RouteRef>,
        excluded: &BTreeSet<batuta_contract::RouteRef>,
    ) -> Result<RouteDecision, SelectError> {
        self.route_matching(envelope, |candidate| {
            allowed.contains(&candidate.route) && !excluded.contains(&candidate.route)
        })
    }

    fn route_matching(
        &self,
        envelope: RouteRequestEnvelopeV2,
        include: impl Fn(&RouteCandidate) -> bool,
    ) -> Result<RouteDecision, SelectError> {
        if envelope.schema_version != 2 || envelope.request.schema_version != 2 {
            return Err(SelectError::invalid(format!(
                "route request and envelope schema_version must be 2; received {}/{}",
                envelope.schema_version, envelope.request.schema_version
            )));
        }
        if self.snapshot.schema_version != 2 {
            return Err(SelectError::invalid(format!(
                "routing snapshot schema_version {} is unsupported; supported: 2",
                self.snapshot.schema_version
            )));
        }
        let action = envelope.request.action.clone();
        let profile = self.snapshot.profiles.get(&action).ok_or_else(|| {
            SelectError::invalid(format!("no routing profile for action '{action}'"))
        })?;
        let public = envelope.request;
        let request = RouteRequestDraft {
            schema_version: public.schema_version,
            action: public.action,
            required_capabilities: public.required_capabilities,
            sensitivity: public.sensitivity,
            required_context: public.required_context,
            effort: public.effort,
            minimum_quality: public.minimum_quality,
            selection_margin: public.selection_margin,
            predicted_tokens: public.predicted_tokens,
            allow_any_eligible: public.allow_any_eligible,
            allow_unverified_quality: public.allow_unverified_quality,
            fallback: self.fallback,
            class: self.class,
            now: self.now,
        }
        .resolve(profile)?;
        let candidates = self
            .snapshot
            .candidates
            .iter()
            .filter(|candidate| candidate.quality.action == action && include(candidate))
            .cloned()
            .collect::<Vec<_>>();
        select_sealed(&request, &candidates, &self.snapshot.seal)
    }
}
