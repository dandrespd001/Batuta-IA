//! La foto compartida se reemplaza de forma durable y completa.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    ExecutionPolicyV2, RouteCandidate, RouteClass, RoutingActionProfile, RoutingSnapshot,
    RoutingSnapshotStore, SelectionMargin,
};

fn snapshot(hash: &str) -> RoutingSnapshot {
    let route = RouteRef::from_str("dsh/opencode/nemotron-3-ultra-free").unwrap();
    RoutingSnapshot::new(
        hash.to_string(),
        ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
        BTreeMap::from([(
            "implementation".to_string(),
            RoutingActionProfile {
                action: "implementation".to_string(),
                minimum_quality: 0.0,
                selection_margin: SelectionMargin::new(0.0).unwrap(),
                allow_any_eligible: false,
                allow_unverified_quality: true,
            },
        )]),
        vec![RouteCandidate {
            route: route.clone(),
            alias: None,
            enabled: false,
            class: RouteClass::ProbeTest,
            capabilities: BTreeSet::from([Capability::Read]),
            max_sensitivity: Sensitivity::Public,
            context_window: 1,
            supported_efforts: BTreeSet::from([ReasoningEffort::Low]),
            quality: QualityProjection {
                route,
                action: "implementation".to_string(),
                researched_score: None,
                effective_score: None,
                coverage: 0,
                contributing_range: None,
                verified: false,
                contributions: vec![],
                exclusions: vec![],
                override_history: vec![],
                active_override: None,
                evidence_hash: "sha256:empty".to_string(),
            },
            relative_cost: 0.0,
            handoff_penalty: 0.0,
            recent_success_rate: 1.0,
            latency_p95_ms: 0,
            cooldown_until: None,
            approved_fallback: false,
        }],
    )
    .unwrap()
}

#[test]
fn guardar_y_volver_a_leer_da_una_foto_completa_equivalente() {
    let root = std::env::temp_dir().join(format!(
        "batuta-snapshot-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let store = RoutingSnapshotStore::open(root.join("snapshot.json"));

    store.save(&snapshot("sha256:first")).unwrap();
    assert_eq!(store.load().unwrap().policy_hash(), "sha256:first");
    store.save(&snapshot("sha256:second")).unwrap();
    assert_eq!(store.load().unwrap().policy_hash(), "sha256:second");

    std::fs::remove_dir_all(root).unwrap();
}
