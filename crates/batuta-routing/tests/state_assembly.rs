//! Ensamblado único desde los cinco componentes del manifest activo.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    ApplicationService, CapabilityIndexEntryV2, CapabilityIndexV2, CatalogRouteStateV2,
    CatalogStateV2, EvidenceStateV2, ExecutionPolicyV2, HealthStateV2, PolicyRouteStateV2,
    PolicyStateV2, RouteClass, RouteHealth, RouteRequestEnvelopeV2, RouteRequestV2,
    RoutingActionProfile, SelectionMargin, StateComponentsV2, StateStore,
};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-assembly-{nonce}"))
}

fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn servicio_abre_manifest_una_vez_y_sella_sus_cinco_componentes() {
    let route = RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap();
    let projection = QualityProjection {
        route: route.clone(),
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
        evidence_hash: hash('e'),
    };
    let components = StateComponentsV2 {
        catalog: CatalogStateV2 {
            schema_version: 2,
            routes: BTreeMap::from([(
                route.clone(),
                CatalogRouteStateV2 {
                    class: RouteClass::Production,
                    max_sensitivity: Sensitivity::Internal,
                    context_window: 100_000,
                    supported_efforts: BTreeSet::from([ReasoningEffort::High]),
                },
            )]),
        },
        policy: PolicyStateV2 {
            schema_version: 2,
            execution: ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
            profiles: BTreeMap::from([(
                "implementation".to_string(),
                RoutingActionProfile {
                    action: "implementation".to_string(),
                    minimum_quality: 80.0,
                    selection_margin: SelectionMargin::new(5.0).unwrap(),
                    allow_any_eligible: false,
                    allow_unverified_quality: false,
                },
            )]),
            routes: BTreeMap::from([(
                route.clone(),
                PolicyRouteStateV2 {
                    alias: Some("minimax".to_string()),
                    enabled: true,
                    relative_cost: 1.0,
                    handoff_penalty: 0.0,
                    approved_fallback: true,
                },
            )]),
        },
        evidence: EvidenceStateV2 {
            schema_version: 2,
            projections: vec![projection],
        },
        health: HealthStateV2 {
            schema_version: 2,
            routes: BTreeMap::from([(route.clone(), RouteHealth::healthy())]),
        },
        capabilities: CapabilityIndexV2 {
            schema_version: 2,
            routes: BTreeMap::from([(
                route.clone(),
                CapabilityIndexEntryV2 {
                    capabilities: BTreeSet::from([Capability::Write]),
                    receipt_hashes: BTreeSet::from([hash('1')]),
                    expires_at: 2_000,
                },
            )]),
        },
    };
    let root = root();
    let store = StateStore::open(root.clone());
    let manifest = store.commit(&components).unwrap();
    let service = ApplicationService::from_state_store(&store, 1_000, false).unwrap();
    let decision = service
        .route(RouteRequestEnvelopeV2 {
            schema_version: 2,
            request: RouteRequestV2 {
                schema_version: 2,
                action: "implementation".to_string(),
                required_capabilities: BTreeSet::from([Capability::Write]),
                sensitivity: Sensitivity::Internal,
                required_context: 1_000,
                effort: Some(ReasoningEffort::High),
                minimum_quality: None,
                selection_margin: None,
                predicted_tokens: 1_000,
                allow_any_eligible: None,
                allow_unverified_quality: None,
            },
        })
        .unwrap();

    assert_eq!(decision.manifest_hash, manifest.manifest_hash().unwrap());
    assert_eq!(decision.catalog_hash, manifest.catalog_hash);
    assert_eq!(decision.capability_receipt_hashes, vec![hash('1')]);
    assert_eq!(service.execution_policy().unwrap().max_handoffs, 2);
    std::fs::remove_dir_all(root).unwrap();
}
