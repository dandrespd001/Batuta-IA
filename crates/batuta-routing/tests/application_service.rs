//! La frontera pública no confía en candidatos, perfiles ni hashes del cliente.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    ApplicationService, ExecutionPolicyV2, RouteCandidate, RouteClass, RouteRequestEnvelopeV2,
    RouteRequestV2, RoutingActionProfile, RoutingSnapshot, SelectionMargin,
};

fn snapshot() -> RoutingSnapshot {
    let route = RouteRef::from_str("dsh/deepseek/deepseek-v4/2026-08").unwrap();
    let fallback = RouteRef::from_str("dsh/fallback/model-v1").unwrap();
    RoutingSnapshot::new(
        "sha256:policy".to_string(),
        ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
        BTreeMap::from([(
            "implementation".to_string(),
            RoutingActionProfile {
                action: "implementation".to_string(),
                minimum_quality: 75.0,
                selection_margin: SelectionMargin::new(5.0).unwrap(),
                allow_any_eligible: false,
                allow_unverified_quality: false,
            },
        )]),
        vec![
            RouteCandidate {
                route: route.clone(),
                alias: Some("deepseekV4-Flash".to_string()),
                enabled: true,
                class: RouteClass::Production,
                capabilities: BTreeSet::from([Capability::Write]),
                max_sensitivity: Sensitivity::Internal,
                context_window: 100_000,
                supported_efforts: BTreeSet::from([ReasoningEffort::High]),
                quality: QualityProjection {
                    route,
                    action: "implementation".to_string(),
                    researched_score: Some(82.0),
                    effective_score: Some(82.0),
                    coverage: 100,
                    contributing_range: None,
                    verified: true,
                    contributions: vec![],
                    exclusions: vec![],
                    override_history: vec![],
                    active_override: None,
                    evidence_hash: "sha256:evidence".to_string(),
                },
                relative_cost: 1.0,
                handoff_penalty: 0.0,
                recent_success_rate: 1.0,
                latency_p95_ms: 100,
                cooldown_until: None,
                approved_fallback: true,
            },
            RouteCandidate {
                route: fallback.clone(),
                alias: Some("fallback".to_string()),
                enabled: true,
                class: RouteClass::Production,
                capabilities: BTreeSet::from([Capability::Write]),
                max_sensitivity: Sensitivity::Internal,
                context_window: 100_000,
                supported_efforts: BTreeSet::from([ReasoningEffort::High]),
                quality: QualityProjection {
                    route: fallback,
                    action: "implementation".to_string(),
                    researched_score: Some(80.0),
                    effective_score: Some(80.0),
                    coverage: 100,
                    contributing_range: None,
                    verified: true,
                    contributions: vec![],
                    exclusions: vec![],
                    override_history: vec![],
                    active_override: None,
                    evidence_hash: "sha256:evidence-fallback".to_string(),
                },
                relative_cost: 2.0,
                handoff_penalty: 0.0,
                recent_success_rate: 1.0,
                latency_p95_ms: 100,
                cooldown_until: None,
                approved_fallback: true,
            },
        ],
    )
    .unwrap()
}

#[test]
fn selecciona_sobre_interseccion_del_grant_y_rutas_vigentes() {
    let service =
        ApplicationService::with_context(snapshot(), 1_000, RouteClass::Production, false);
    let fallback = RouteRef::from_str("dsh/fallback/model-v1").unwrap();
    let allowed = BTreeSet::from([
        fallback.clone(),
        RouteRef::from_str("dsh/new/model").unwrap(),
    ]);
    let decision = service
        .route_with_allowed_routes(
            RouteRequestEnvelopeV2 {
                schema_version: 2,
                request: request(),
            },
            &allowed,
            &BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(decision.route, fallback);

    let withdrawn = BTreeSet::from([RouteRef::from_str("dsh/withdrawn/model").unwrap()]);
    assert!(
        service
            .route_with_allowed_routes(
                RouteRequestEnvelopeV2 {
                    schema_version: 2,
                    request: request(),
                },
                &withdrawn,
                &BTreeSet::new(),
            )
            .is_err()
    );
}

fn request() -> RouteRequestV2 {
    RouteRequestV2 {
        schema_version: 2,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 10_000,
        effort: Some(ReasoningEffort::High),
        minimum_quality: None,
        selection_margin: None,
        predicted_tokens: 5_000,
        allow_any_eligible: None,
        allow_unverified_quality: None,
    }
}

#[test]
fn el_servicio_ensambla_candidatos_y_perfil_desde_una_foto_local() {
    let service =
        ApplicationService::with_context(snapshot(), 1_000, RouteClass::Production, false);
    let decision = service
        .route(RouteRequestEnvelopeV2 {
            schema_version: 2,
            request: request(),
        })
        .unwrap();

    assert_eq!(
        decision.route.to_string(),
        "dsh/deepseek/deepseek-v4/2026-08"
    );
    assert_eq!(decision.policy_hash, "sha256:policy");
    assert_eq!(service.execution_policy().unwrap().max_attempts, 3);
}

#[test]
fn el_json_publico_rechaza_candidatos_perfiles_y_hashes_inyectados() {
    for field in ["candidates", "profile", "policy_hash", "evidence_hash"] {
        let mut value = serde_json::to_value(RouteRequestEnvelopeV2 {
            schema_version: 2,
            request: request(),
        })
        .unwrap();
        value[field] = serde_json::json!([]);

        let error = serde_json::from_value::<RouteRequestEnvelopeV2>(value).unwrap_err();
        assert!(
            error.to_string().contains("unknown field"),
            "{field}: {error}"
        );
    }
}

#[test]
fn el_json_publico_rechaza_reloj_clase_y_fallback_inyectados() {
    for (field, injected) in [
        ("now", serde_json::json!(1_000)),
        ("class", serde_json::json!("production")),
        ("fallback", serde_json::json!(false)),
    ] {
        let mut value = serde_json::to_value(request()).unwrap();
        value[field] = injected;
        let error = serde_json::from_value::<RouteRequestV2>(value).unwrap_err();
        assert!(
            error.to_string().contains("unknown field"),
            "{field}: {error}"
        );
    }
}
