//! Contrato del selector balanceado y determinista.

use std::collections::BTreeSet;
use std::str::FromStr;

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    AuthorizationDecision, DiscardReason, RouteCandidate, RouteClass, RouteRequest,
    SelectionAuthorizations, SelectionMargin, select,
};

fn route(value: &str) -> RouteRef {
    RouteRef::from_str(value).unwrap()
}

fn quality(route: &RouteRef, score: f64, verified: bool) -> QualityProjection {
    QualityProjection {
        route: route.clone(),
        action: "implementation".to_string(),
        researched_score: Some(score),
        effective_score: Some(score),
        coverage: 100,
        contributing_range: None,
        verified,
        contributions: vec![],
        exclusions: vec![],
        override_history: vec![],
        active_override: None,
        evidence_hash: format!("sha256:{score}"),
    }
}

fn candidate(reference: &str, score: f64, cost: f64) -> RouteCandidate {
    let reference = route(reference);
    RouteCandidate {
        route: reference.clone(),
        alias: None,
        enabled: true,
        class: RouteClass::Production,
        capabilities: BTreeSet::from([Capability::Read, Capability::Write]),
        max_sensitivity: Sensitivity::Internal,
        context_window: 200_000,
        supported_efforts: BTreeSet::from([ReasoningEffort::High]),
        quality: quality(&reference, score, true),
        relative_cost: cost,
        handoff_penalty: 100.0,
        recent_success_rate: 1.0,
        latency_p95_ms: 1_000,
        cooldown_until: None,
        approved_fallback: true,
    }
}

fn request() -> RouteRequest {
    RouteRequest {
        schema_version: 2,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 100_000,
        effort: Some(ReasoningEffort::High),
        minimum_quality: 75.0,
        selection_margin: SelectionMargin::new(5.0).unwrap(),
        predicted_tokens: 10_000,
        authorizations: SelectionAuthorizations {
            allow_any_eligible: AuthorizationDecision {
                requested: false,
                permitted: false,
            },
            allow_unverified_quality: AuthorizationDecision {
                requested: false,
                permitted: false,
            },
        },
        fallback: false,
        class: RouteClass::Production,
        now: 1_000,
    }
}

#[test]
fn elige_la_mas_barata_dentro_del_margen_de_calidad() {
    let expensive = candidate("codex/openai/gpt-5.6", 90.0, 3.0);
    let cheap = candidate("dsh/deepseek/deepseek-v4", 86.0, 1.0);
    let too_far = candidate("dsh/minimax/minimax-m2.5", 84.0, 0.1);

    let decision = select(&request(), &[expensive, cheap, too_far], "sha256:policy").unwrap();

    assert_eq!(decision.route.to_string(), "dsh/deepseek/deepseek-v4");
    let rejected = decision
        .discarded
        .iter()
        .find(|item| item.route.to_string() == "dsh/minimax/minimax-m2.5")
        .unwrap();
    assert!(
        rejected
            .reasons
            .contains(&DiscardReason::OutsideSelectionMargin {
                score: 84.0,
                floor: 85.0,
            })
    );
}

#[test]
fn fallbacks_no_aprobados_solo_entran_con_autorizacion_visible() {
    let mut cheap = candidate("dsh/minimax/minimax-m2.5", 90.0, 0.1);
    cheap.approved_fallback = false;
    let approved = candidate("codex/openai/gpt-5.6", 90.0, 3.0);
    let mut fallback_request = request();
    fallback_request.fallback = true;

    let normal = select(
        &fallback_request,
        &[cheap.clone(), approved.clone()],
        "sha256:policy",
    )
    .unwrap();
    assert_eq!(normal.route, approved.route);

    fallback_request.authorizations.allow_any_eligible = AuthorizationDecision {
        requested: true,
        permitted: true,
    };
    let authorized = select(&fallback_request, &[cheap, approved], "sha256:policy").unwrap();
    assert_eq!(authorized.route.to_string(), "dsh/minimax/minimax-m2.5");
    assert!(authorized.authorizations.allow_any_eligible.permitted);
}

#[test]
fn calidad_no_verificada_necesita_otra_autorizacion_separada() {
    let mut only = candidate("dsh/minimax/minimax-m2.5", 90.0, 0.1);
    only.quality.verified = false;

    let error = select(&request(), &[only.clone()], "sha256:policy").unwrap_err();
    assert!(
        error.discarded[0]
            .reasons
            .contains(&DiscardReason::UnverifiedQuality)
    );

    let mut authorized = request();
    authorized.authorizations.allow_unverified_quality = AuthorizationDecision {
        requested: true,
        permitted: true,
    };
    let decision = select(&authorized, &[only], "sha256:policy").unwrap();
    assert!(decision.authorizations.allow_unverified_quality.permitted);
}

#[test]
fn enumera_capacidad_cooldown_y_desempata_por_identificador() {
    let mut unavailable = candidate("abacus/abacus/glm-5.3", 90.0, 1.0);
    unavailable.capabilities.clear();
    unavailable.cooldown_until = Some(2_000);
    let z = candidate("dsh/zeta/model", 90.0, 1.0);
    let a = candidate("dsh/alpha/model", 90.0, 1.0);

    let decision = select(&request(), &[unavailable, z, a], "sha256:policy").unwrap();

    assert_eq!(decision.route.to_string(), "dsh/alpha/model");
    let reasons = &decision
        .discarded
        .iter()
        .find(|item| item.route.to_string() == "abacus/abacus/glm-5.3")
        .unwrap()
        .reasons;
    assert!(reasons.iter().any(|reason| matches!(
        reason,
        DiscardReason::MissingCapability {
            capability: Capability::Write
        }
    )));
    assert!(reasons.contains(&DiscardReason::Cooldown { until: 2_000 }));
}
