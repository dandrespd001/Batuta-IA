use std::collections::{BTreeMap, BTreeSet};

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

use crate::hash::hash_json;
use crate::{
    ActionProfile, BenchmarkObservation, BenchmarkWeight, OverrideEvent, OverrideOperation,
    QualityError, SourceKind,
};

/// Extremos reales de los resultados que contribuyeron.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreRange {
    /// Menor resultado contribuyente.
    pub min: f64,
    /// Mayor resultado contribuyente.
    pub max: f64,
}

/// Motivo estable por el que una observación no contribuyó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionCode {
    /// La ruta exacta no coincide.
    RouteMismatch,
    /// El benchmark no coincide.
    BenchmarkMismatch,
    /// La versión del benchmark no coincide.
    BenchmarkVersionMismatch,
    /// El escenario no coincide.
    ScenarioMismatch,
    /// La configuración no coincide.
    ConfigurationMismatch,
    /// El scaffold no coincide.
    ScaffoldMismatch,
    /// La métrica no coincide.
    MetricMismatch,
    /// La revisión no coincide con la esperada.
    RevisionMismatch,
    /// Una ruta sin revisión produciría una mezcla de revisiones.
    AmbiguousRevision,
    /// La observación está caducada.
    Expired,
}

/// Observación aceptada con su antigüedad y fuente reproducibles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContribution {
    /// Identificador de observación.
    pub observation: String,
    /// Puntaje que entró en el cálculo.
    pub normalized_score: f64,
    /// Fecha observada.
    pub observed_at: u64,
    /// Edad al proyectar.
    pub age_seconds: u64,
    /// Primer segundo en que deja de ser vigente.
    pub expires_at: u64,
    /// URL primaria.
    pub source_url: String,
    /// Tipo de fuente.
    pub source_kind: SourceKind,
}

/// Observación descartada con un motivo público estable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationExclusion {
    /// Identificador de observación.
    pub observation: String,
    /// Código cerrado.
    pub code: ExclusionCode,
    /// Campo incompatible.
    pub field: String,
    /// Explicación estable.
    pub message: String,
    /// Valores relevantes para auditar el descarte.
    pub details: BTreeMap<String, String>,
    /// Fecha observada.
    pub observed_at: u64,
    /// Edad al proyectar.
    pub age_seconds: u64,
    /// Primer segundo en que deja de ser vigente.
    pub expires_at: u64,
    /// URL primaria.
    pub source_url: String,
    /// Tipo de fuente.
    pub source_kind: SourceKind,
}

/// Puntaje derivado para una ruta y una acción.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityProjection {
    /// Ruta exacta evaluada.
    pub route: RouteRef,
    /// Acción evaluada.
    pub action: String,
    /// Puntaje derivado; `None` cuando no hay cobertura.
    pub researched_score: Option<f64>,
    /// Override o puntaje investigado.
    pub effective_score: Option<f64>,
    /// Cobertura porcentual de la cesta.
    pub coverage: u8,
    /// Rango de observaciones contribuyentes.
    pub contributing_range: Option<ScoreRange>,
    /// Si cumple cobertura y existe fuente independiente o evaluación local.
    pub verified: bool,
    /// Contribuciones con fecha, edad, caducidad y fuente.
    pub contributions: Vec<EvidenceContribution>,
    /// Descartes estructurados y reproducibles.
    pub exclusions: Vec<ObservationExclusion>,
    /// Historial append-only ordenado.
    pub override_history: Vec<OverrideEvent>,
    /// Último `set` no retirado por un `clear`.
    pub active_override: Option<OverrideEvent>,
    /// Hash de ruta, perfil, contribuciones, descartes e historial.
    pub evidence_hash: String,
}

/// Proyecta calidad sin mutar evidencia ni política.
///
/// # Errors
///
/// Si el perfil, una observación o un evento incumplen el esquema, hay ids
/// duplicados o no se puede serializar la evidencia para sellarla.
#[allow(clippy::too_many_lines)]
pub fn project(
    route: &RouteRef,
    profile: &ActionProfile,
    observations: &[BenchmarkObservation],
    override_events: &[OverrideEvent],
    now: u64,
) -> Result<QualityProjection, QualityError> {
    profile.validate()?;
    let mut seen = BTreeSet::new();
    for observation in observations {
        observation.validate()?;
        if !seen.insert(observation.id.as_str()) {
            return Err(QualityError::DuplicateObservation {
                id: observation.id.clone(),
            });
        }
    }

    let mut sorted_observations: Vec<_> = observations.iter().collect();
    sorted_observations.sort_by(|left, right| left.id.cmp(&right.id));

    let mut exclusions = Vec::new();
    let mut contributions = Vec::new();
    let mut weighted_score = 0.0;
    let mut coverage: u16 = 0;
    let mut range: Option<ScoreRange> = None;
    let mut has_verifying_source = false;

    for component in &profile.basket {
        let mut candidates = Vec::new();
        for observation in &sorted_observations {
            if let Some((code, field, expected, actual)) = mismatch(route, component, observation) {
                exclusions.push(exclusion(
                    observation,
                    component,
                    code,
                    field,
                    expected,
                    actual,
                    now,
                    profile.max_age_seconds,
                ));
                continue;
            }
            let age = now.saturating_sub(observation.observed_at);
            if age > profile.max_age_seconds {
                exclusions.push(exclusion(
                    observation,
                    component,
                    ExclusionCode::Expired,
                    "observed_at",
                    format!("age <= {}", profile.max_age_seconds),
                    age.to_string(),
                    now,
                    profile.max_age_seconds,
                ));
                continue;
            }
            candidates.push(*observation);
        }

        let expected_revision = route
            .revision()
            .or(component.expected_model_revision.as_deref());
        if expected_revision.is_none() {
            let revisions: BTreeSet<_> = candidates
                .iter()
                .map(|observation| observation.model_revision.as_str())
                .collect();
            if revisions.len() > 1 {
                let actual = revisions.into_iter().collect::<Vec<_>>().join(",");
                for observation in candidates {
                    exclusions.push(exclusion(
                        observation,
                        component,
                        ExclusionCode::AmbiguousRevision,
                        "model_revision",
                        "one revision for an unpinned route".to_string(),
                        actual.clone(),
                        now,
                        profile.max_age_seconds,
                    ));
                }
                continue;
            }
        }

        let mut scores = Vec::new();
        for observation in candidates {
            if let Some(expected) = expected_revision
                && observation.model_revision != expected
            {
                exclusions.push(exclusion(
                    observation,
                    component,
                    ExclusionCode::RevisionMismatch,
                    "model_revision",
                    expected.to_string(),
                    observation.model_revision.clone(),
                    now,
                    profile.max_age_seconds,
                ));
                continue;
            }
            scores.push(observation.normalized_score);
            let age_seconds = now.saturating_sub(observation.observed_at);
            contributions.push(EvidenceContribution {
                observation: observation.id.clone(),
                normalized_score: observation.normalized_score,
                observed_at: observation.observed_at,
                age_seconds,
                expires_at: observation
                    .observed_at
                    .saturating_add(profile.max_age_seconds),
                source_url: observation.source_url.clone(),
                source_kind: observation.source_kind,
            });
            has_verifying_source |= matches!(
                observation.source_kind,
                SourceKind::Independent | SourceKind::LocalEvaluation
            );
            range = Some(match range {
                None => ScoreRange {
                    min: observation.normalized_score,
                    max: observation.normalized_score,
                },
                Some(current) => ScoreRange {
                    min: current.min.min(observation.normalized_score),
                    max: current.max.max(observation.normalized_score),
                },
            });
        }
        if !scores.is_empty() {
            let count = u32::try_from(scores.len()).map_err(|_| QualityError::InvalidField {
                field: "observations",
                message: "too many observations for one benchmark component".to_string(),
            })?;
            let mean = scores.iter().sum::<f64>() / f64::from(count);
            weighted_score += mean * f64::from(component.weight);
            coverage += u16::from(component.weight);
        }
    }

    contributions.sort_by(|left, right| left.observation.cmp(&right.observation));
    let contributing_ids: BTreeSet<_> = contributions
        .iter()
        .map(|item| item.observation.as_str())
        .collect();
    exclusions.retain(|item| !contributing_ids.contains(item.observation.as_str()));
    let mut best_exclusion = BTreeMap::new();
    for item in exclusions {
        best_exclusion
            .entry(item.observation.clone())
            .and_modify(|current: &mut ObservationExclusion| {
                if exclusion_priority(item.code) > exclusion_priority(current.code) {
                    *current = item.clone();
                }
            })
            .or_insert(item);
    }
    let mut exclusions: Vec<_> = best_exclusion.into_values().collect();
    exclusions.sort_by(|left, right| {
        (&left.observation, left.code, &left.field, &left.details).cmp(&(
            &right.observation,
            right.code,
            &right.field,
            &right.details,
        ))
    });

    let coverage =
        u8::try_from(coverage).map_err(|_| QualityError::InvalidWeights { sum: coverage })?;
    let researched_score = (coverage > 0).then(|| weighted_score / f64::from(coverage));
    let (override_history, active_override) = resolve_overrides(override_events)?;
    let effective_score = active_override
        .as_ref()
        .and_then(|event| event.score)
        .or(researched_score);
    let verified = coverage >= profile.minimum_coverage && has_verifying_source;
    let evidence_hash = hash_json(&(
        route,
        route.revision(),
        profile,
        &contributions,
        &exclusions,
        &override_history,
    ))?;

    Ok(QualityProjection {
        route: route.clone(),
        action: profile.action.clone(),
        researched_score,
        effective_score,
        coverage,
        contributing_range: range,
        verified,
        contributions,
        exclusions,
        override_history,
        active_override,
        evidence_hash,
    })
}

fn mismatch(
    route: &RouteRef,
    component: &BenchmarkWeight,
    observation: &BenchmarkObservation,
) -> Option<(ExclusionCode, &'static str, String, String)> {
    let checks = [
        (
            &observation.route == route,
            ExclusionCode::RouteMismatch,
            "route",
            route.to_string(),
            observation.route.to_string(),
        ),
        (
            observation.benchmark == component.benchmark,
            ExclusionCode::BenchmarkMismatch,
            "benchmark",
            component.benchmark.clone(),
            observation.benchmark.clone(),
        ),
        (
            observation.benchmark_version == component.benchmark_version,
            ExclusionCode::BenchmarkVersionMismatch,
            "benchmark_version",
            component.benchmark_version.clone(),
            observation.benchmark_version.clone(),
        ),
        (
            observation.scenario == component.scenario,
            ExclusionCode::ScenarioMismatch,
            "scenario",
            component.scenario.clone(),
            observation.scenario.clone(),
        ),
        (
            observation.configuration == component.configuration,
            ExclusionCode::ConfigurationMismatch,
            "configuration",
            component.configuration.clone(),
            observation.configuration.clone(),
        ),
        (
            observation.scaffold == component.scaffold,
            ExclusionCode::ScaffoldMismatch,
            "scaffold",
            component.scaffold.clone(),
            observation.scaffold.clone(),
        ),
        (
            observation.metric == component.metric,
            ExclusionCode::MetricMismatch,
            "metric",
            component.metric.clone(),
            observation.metric.clone(),
        ),
    ];
    checks
        .into_iter()
        .find(|(matches, ..)| !matches)
        .map(|(_, code, field, expected, actual)| (code, field, expected, actual))
}

#[allow(clippy::too_many_arguments)]
fn exclusion(
    observation: &BenchmarkObservation,
    component: &BenchmarkWeight,
    code: ExclusionCode,
    field: &'static str,
    expected: String,
    actual: String,
    now: u64,
    max_age_seconds: u64,
) -> ObservationExclusion {
    let mut details = BTreeMap::new();
    details.insert("actual".to_string(), actual);
    details.insert("benchmark".to_string(), component.benchmark.clone());
    details.insert("expected".to_string(), expected);
    details.insert("scenario".to_string(), component.scenario.clone());
    ObservationExclusion {
        observation: observation.id.clone(),
        code,
        field: field.to_string(),
        message: format!("observation excluded: {field} is incompatible"),
        details,
        observed_at: observation.observed_at,
        age_seconds: now.saturating_sub(observation.observed_at),
        expires_at: observation.observed_at.saturating_add(max_age_seconds),
        source_url: observation.source_url.clone(),
        source_kind: observation.source_kind,
    }
}

fn resolve_overrides(
    events: &[OverrideEvent],
) -> Result<(Vec<OverrideEvent>, Option<OverrideEvent>), QualityError> {
    let mut history = events.to_vec();
    history.sort_by(|left, right| {
        (left.recorded_at, left.id.as_str()).cmp(&(right.recorded_at, right.id.as_str()))
    });
    let mut seen = BTreeSet::new();
    let mut active = None;
    for event in &history {
        event.validate()?;
        if !seen.insert(event.id.as_str()) {
            return Err(QualityError::InvalidField {
                field: "override_event.id",
                message: format!("duplicate event id '{}'", event.id),
            });
        }
        match event.operation {
            OverrideOperation::Set => active = Some(event.clone()),
            OverrideOperation::Clear => active = None,
        }
    }
    Ok((history, active))
}

const fn exclusion_priority(code: ExclusionCode) -> u8 {
    match code {
        ExclusionCode::RouteMismatch => 1,
        ExclusionCode::BenchmarkMismatch => 2,
        ExclusionCode::BenchmarkVersionMismatch => 3,
        ExclusionCode::ScenarioMismatch => 4,
        ExclusionCode::ConfigurationMismatch => 5,
        ExclusionCode::ScaffoldMismatch => 6,
        ExclusionCode::MetricMismatch => 7,
        ExclusionCode::RevisionMismatch => 8,
        ExclusionCode::AmbiguousRevision => 9,
        ExclusionCode::Expired => 10,
    }
}
