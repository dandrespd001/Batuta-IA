//! Los recibos de routing son append-only y congelan hashes y transiciones.

use std::collections::BTreeSet;
use std::str::FromStr as _;

use batuta_contract::{Capability, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    AuthorizationDecision, FailureCategory, HandoffDraft, RouteClass, RouteDecision, RouteRequest,
    RoutingReceipt, RoutingReceiptStore, RoutingTransition, SelectionAuthorizations,
    SelectionMargin,
};

fn no_authorizations() -> SelectionAuthorizations {
    SelectionAuthorizations {
        allow_any_eligible: AuthorizationDecision {
            requested: false,
            permitted: false,
        },
        allow_unverified_quality: AuthorizationDecision {
            requested: false,
            permitted: false,
        },
    }
}

#[test]
fn reiniciar_conserva_el_recibo_y_un_id_no_se_sobrescribe() {
    let route = RouteRef::from_str("dsh/minimax/minimax-m2.5/r7").unwrap();
    let request = RouteRequest {
        schema_version: 2,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 1,
        effort: None,
        minimum_quality: 70.0,
        selection_margin: SelectionMargin::new(5.0).unwrap(),
        predicted_tokens: 10,
        authorizations: no_authorizations(),
        fallback: false,
        class: RouteClass::Production,
        now: 100,
    };
    let projection = QualityProjection {
        route: route.clone(),
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
        evidence_hash: "sha256:evidence-old".to_string(),
    };
    let decision = RouteDecision {
        schema_version: 2,
        route: route.clone(),
        alias: None,
        researched_score: Some(80.0),
        effective_score: 80.0,
        manual_override: None,
        coverage: 100,
        verified: true,
        expected_cost: 10.0,
        evidence_hash: "sha256:evidence-old".to_string(),
        policy_hash: "sha256:policy-old".to_string(),
        manifest_hash: "sha256:manifest-old".to_string(),
        catalog_hash: "sha256:catalog-old".to_string(),
        health_hash: "sha256:health-old".to_string(),
        capabilities_hash: "sha256:capabilities-old".to_string(),
        capability_receipt_hashes: vec![],
        discarded: vec![],
        authorizations: no_authorizations(),
    };
    let checkpoint = HandoffDraft {
        schema_version: 1,
        objective: "terminar".to_string(),
        constraints: vec!["sin red".to_string()],
        decisions: vec!["ruta exacta".to_string()],
        files: vec!["src/lib.rs".to_string()],
        diff_summary: "cambio acotado".to_string(),
        tests: vec![],
        failure: FailureCategory::QuotaExhausted,
        failure_message: "quota".to_string(),
        next_step: "usar fallback".to_string(),
        remaining_tokens: 9,
        remaining_wall_seconds: 60,
    }
    .try_into()
    .unwrap();
    let receipt = RoutingReceipt::new(
        "receipt-1".to_string(),
        100,
        request,
        vec![projection],
        decision,
        vec![RoutingTransition::new(
            100,
            "planned",
            "running",
            Some(route),
        )],
        Some(checkpoint),
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!("batuta-routing-receipt-{}", std::process::id()));
    let store = RoutingReceiptStore::open(root.clone());

    store.append(&receipt).unwrap();
    let loaded = RoutingReceiptStore::open(root.clone())
        .load("receipt-1")
        .unwrap();
    assert_eq!(loaded.policy_hash, "sha256:policy-old");
    assert_eq!(loaded.evidence_hash, "sha256:evidence-old");
    assert_eq!(loaded.transitions.len(), 1);
    assert!(store.append(&receipt).is_err());

    std::fs::remove_dir_all(root).unwrap();
}
