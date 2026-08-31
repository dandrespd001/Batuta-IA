//! La decisión pública fija una única generación y recibos exactos.

use std::collections::BTreeSet;
use std::str::FromStr as _;

use batuta_contract::{Capability, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    AuthorizationDecision, DecisionSealV2, RouteCandidate, RouteClass, RouteRequest,
    SelectionAuthorizations, SelectionMargin, select_sealed,
};

fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn decision_sella_manifest_componentes_y_recibos_en_orden() {
    let route = RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap();
    let candidate = RouteCandidate {
        route: route.clone(),
        alias: None,
        enabled: true,
        class: RouteClass::Production,
        capabilities: BTreeSet::from([Capability::Write]),
        max_sensitivity: Sensitivity::Internal,
        context_window: 100_000,
        supported_efforts: BTreeSet::new(),
        quality: QualityProjection {
            route,
            action: "implementation".to_string(),
            researched_score: Some(90.0),
            effective_score: Some(90.0),
            coverage: 100,
            contributing_range: None,
            verified: true,
            contributions: vec![],
            exclusions: vec![],
            override_history: vec![],
            active_override: None,
            evidence_hash: hash('c'),
        },
        relative_cost: 1.0,
        handoff_penalty: 0.0,
        recent_success_rate: 1.0,
        latency_p95_ms: 10,
        cooldown_until: None,
        approved_fallback: true,
    };
    let request = RouteRequest {
        schema_version: 2,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 1,
        effort: None,
        minimum_quality: 80.0,
        selection_margin: SelectionMargin::new(5.0).unwrap(),
        predicted_tokens: 100,
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
        now: 100,
    };
    let seal = DecisionSealV2 {
        manifest_hash: hash('a'),
        catalog_hash: hash('b'),
        policy_hash: hash('d'),
        evidence_hash: hash('c'),
        health_hash: hash('e'),
        capabilities_hash: hash('f'),
        capability_receipt_hashes: BTreeSet::from([hash('2'), hash('1')]),
    };
    let decision = select_sealed(&request, &[candidate], &seal).unwrap();

    assert_eq!(decision.manifest_hash, hash('a'));
    assert_eq!(decision.catalog_hash, hash('b'));
    assert_eq!(
        decision.capability_receipt_hashes,
        vec![hash('1'), hash('2')]
    );
    let first = serde_json::to_vec(&decision).unwrap();
    let second = serde_json::to_vec(&decision).unwrap();
    assert_eq!(first, second);
}
