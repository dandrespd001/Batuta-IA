//! API operativa compartida: respuestas v2, grants, perfil y corridas.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_cli::{Layout, OperationalApi, run_json, run_status};
use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_exec::{FakeHarnessExecutor, NormalizedInvocationResult, TokenUsage};
use batuta_quality::QualityProjection;
use batuta_routing::{
    CapabilityIndexEntryV2, CapabilityIndexV2, CatalogRouteStateV2, CatalogStateV2,
    EvidenceStateV2, ExecutionPolicyV2, HealthStateV2, PolicyRouteStateV2, PolicyStateV2,
    RouteClass, RouteHealth, RoutingActionProfile, RunPhaseV2, SelectionMargin, StateComponentsV2,
    StateStore,
};

fn root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-operational-api-{label}-{nonce}"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn hash(marker: char) -> String {
    format!("sha256:{}", marker.to_string().repeat(64))
}

fn route() -> RouteRef {
    RouteRef::from_str("dsh/fake/model-v1/revision-1").unwrap()
}

fn install_state(root: &std::path::Path, expires_at: u64) -> String {
    let route = route();
    let manifest = StateStore::open(root.join("state-v2"))
        .commit(&StateComponentsV2 {
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
                execution: ExecutionPolicyV2::new(2, 2_000, 0).unwrap(),
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
                        alias: None,
                        enabled: true,
                        relative_cost: 1.0,
                        handoff_penalty: 0.0,
                        approved_fallback: true,
                    },
                )]),
            },
            evidence: EvidenceStateV2 {
                schema_version: 2,
                projections: vec![QualityProjection {
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
                }],
            },
            health: HealthStateV2 {
                schema_version: 2,
                routes: BTreeMap::from([(route.clone(), RouteHealth::healthy())]),
            },
            capabilities: CapabilityIndexV2 {
                schema_version: 2,
                routes: BTreeMap::from([(
                    route,
                    CapabilityIndexEntryV2 {
                        capabilities: BTreeSet::from([Capability::Write]),
                        receipt_hashes: BTreeSet::from([hash('c')]),
                        expires_at,
                    },
                )]),
            },
        })
        .unwrap();
    manifest.manifest_hash().unwrap()
}

fn grant_json(id: &str, manifest_hash: &str, now: u64, routes: &[RouteRef]) -> String {
    serde_json::json!({
        "schema_version": 1,
        "id": id,
        "issued_at": now.saturating_sub(1),
        "expires_at": now + 3_600,
        "manifest_hash": manifest_hash,
        "routes": routes,
        "actions": ["implementation"],
        "operations": ["run"],
        "limits": {
            "requests": 3,
            "input_tokens": 10_000,
            "output_tokens": 10_000,
            "wall_time_ms": 60_000
        }
    })
    .to_string()
}

fn run_request() -> String {
    serde_json::json!({
        "schema_version": 2,
        "id": "run-1",
        "objective": "implement the requested change",
        "task": {
            "role": "implementation",
            "sensitivity": "internal",
            "output_contract": "unified_diff",
            "write_mode": "validated_patch",
            "allowed_write_paths": ["src"],
            "required_capabilities": ["write"],
            "gate_profile": "standard",
            "timeout_seconds": 10,
            "max_repairs": 0
        },
        "routing": {
            "schema_version": 2,
            "request": {
                "schema_version": 2,
                "action": "implementation",
                "required_capabilities": ["write"],
                "sensitivity": "internal",
                "required_context": 1,
                "predicted_tokens": 100
            }
        },
        "grant_id": "grant-1"
    })
    .to_string()
}

#[test]
fn grant_exige_confirmacion_manifest_vigente_y_rutas_ya_presentes() {
    let root = root("grant");
    let layout = Layout::under(root.clone());
    let now = now_secs();
    let manifest_hash = install_state(&root, now + 3_600);
    let api = OperationalApi::new(&layout, now);
    let draft = grant_json("grant-1", &manifest_hash, now, &[route()]);

    let error = api.grant_create_json(&draft, false).unwrap_err();
    assert_eq!(error.schema_version, 2);
    assert_eq!(error.code, "confirmation_required");
    assert_eq!(error.field, "confirm");

    let created = api.grant_create_json(&draft, true).unwrap();
    assert_eq!(created.schema_version, 2);
    assert!(created.data.grant_hash.starts_with("sha256:"));
    assert_eq!(
        api.grant_status("grant-1").unwrap().data.grant,
        created.data
    );

    let future_route = RouteRef::from_str("dsh/future/model-v2/revision-2").unwrap();
    let future = grant_json("grant-future", &manifest_hash, now, &[future_route]);
    assert_eq!(
        api.grant_create_json(&future, true).unwrap_err().code,
        "grant_route_not_current"
    );

    assert_eq!(
        api.grant_revoke("grant-1", false).unwrap_err().code,
        "confirmation_required"
    );
    assert!(
        api.grant_revoke("grant-1", true)
            .unwrap()
            .data
            .revocation
            .is_some()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn perfil_solo_se_activa_despues_de_staging_cas_y_confirmacion() {
    let root = root("profile");
    let workdir = root.join("workdir");
    std::fs::create_dir_all(&workdir).unwrap();
    let layout = Layout::under(root.clone());
    let api = OperationalApi::new(&layout, now_secs());
    let draft = serde_json::json!({
        "schema_version": 1,
        "workdir": workdir,
        "max_stdout_bytes": 4096,
        "max_stderr_bytes": 2048,
        "termination_grace_ms": 100
    })
    .to_string();

    let proposal = api.profile_import_json(&draft).unwrap().data;
    assert!(api.profile_status().unwrap().data.active.is_none());
    assert_eq!(
        api.profile_apply(&proposal.id, &proposal.expected_active_hash, false)
            .unwrap_err()
            .field,
        "confirm"
    );
    let active = api
        .profile_apply(&proposal.id, &proposal.expected_active_hash, true)
        .unwrap()
        .data;
    assert_eq!(active.profile_hash(), proposal.proposed_profile_hash);
    assert_eq!(api.profile_status().unwrap().data.active.unwrap(), active);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn run_y_status_comparten_sobre_y_estado_durable_con_un_fake() {
    let root = root("run");
    let layout = Layout::under(root.clone());
    let now = now_secs();
    let manifest_hash = install_state(&root, now + 3_600);
    OperationalApi::new(&layout, now)
        .grant_create_json(
            &grant_json("grant-1", &manifest_hash, now, &[route()]),
            true,
        )
        .unwrap();
    let executor = FakeHarnessExecutor::new(NormalizedInvocationResult {
        output: "done".to_string(),
        usage: TokenUsage {
            input_tokens: 50,
            output_tokens: 20,
        },
        latency_ms: 5,
        provenance: Some("fake".to_string()),
        manifest_hash: Some(manifest_hash),
        failure: None,
    });

    let started = run_json(&layout, &executor, &run_request()).unwrap();
    assert_eq!(started.schema_version, 2);
    assert_eq!(started.data.phase, RunPhaseV2::Completed);
    assert_eq!(executor.invocation_count(), 1);
    assert_eq!(run_status(&layout, "run-1").unwrap().data, started.data);

    let error = run_json(&layout, &executor, "{").unwrap_err();
    assert_eq!(error.schema_version, 2);
    assert_eq!(error.field, "request");
    std::fs::remove_dir_all(root).unwrap();
}
