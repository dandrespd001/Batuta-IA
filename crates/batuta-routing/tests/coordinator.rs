//! Coordinación durable: grant, reserva, journal, una llamada y crash ambiguo.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity, TaskSpec};
use batuta_exec::{
    ExecutorError, FakeHarnessExecutor, HarnessExecutor, InvocationRequestV2,
    NormalizedInvocationResult, TokenUsage,
};
use batuta_quality::QualityProjection;
use batuta_routing::{
    BudgetAmount, CapabilityIndexEntryV2, CapabilityIndexV2, CatalogRouteStateV2, CatalogStateV2,
    EvidenceStateV2, ExecutionGrantV1, ExecutionPolicyV2, GrantLimits, GrantOperation, GrantStore,
    HealthStateV2, PolicyRouteStateV2, PolicyStateV2, RouteClass, RouteHealth,
    RouteRequestEnvelopeV2, RouteRequestV2, RoutingActionProfile, RunClock, RunCoordinator,
    RunJournalKindV2, RunPhaseV2, RunRequestV2, RunSleeper, SelectionMargin, StateComponentsV2,
    StateStore,
};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-coordinator-{nonce}"))
}

fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn route() -> RouteRef {
    RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap()
}

fn install_state(root: &std::path::Path) {
    let route = route();
    StateStore::open(root.join("state-v2"))
        .commit(&StateComponentsV2 {
            catalog: CatalogStateV2 {
                schema_version: 2,
                routes: BTreeMap::from([(
                    route.clone(),
                    CatalogRouteStateV2 {
                        class: RouteClass::Production,
                        max_sensitivity: Sensitivity::Internal,
                        context_window: 10_000,
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
                    evidence_hash: hash('9'),
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
                        receipt_hashes: BTreeSet::from([hash('1')]),
                        expires_at: 10_000,
                    },
                )]),
            },
        })
        .unwrap();
}

struct TestRuntime(AtomicU64);

impl TestRuntime {
    fn new(now_ms: u64) -> Self {
        Self(AtomicU64::new(now_ms))
    }

    fn set(&self, now_ms: u64) {
        self.0.store(now_ms, Ordering::SeqCst);
    }
}

impl RunClock for TestRuntime {
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

impl RunSleeper for TestRuntime {
    fn sleep_millis(&self, _millis: u64) {
        panic!("these coordinator tests must not sleep");
    }
}

fn request(id: &str) -> RunRequestV2 {
    let task: TaskSpec = serde_json::from_str(
        r#"{
            "role":"implementation",
            "sensitivity":"internal",
            "output_contract":"unified_diff",
            "write_mode":"validated_patch",
            "allowed_write_paths":["src"],
            "required_capabilities":["write"],
            "gate_profile":"standard",
            "timeout_seconds":10,
            "max_repairs":0
        }"#,
    )
    .unwrap();
    RunRequestV2 {
        schema_version: 2,
        id: id.to_string(),
        objective: "implement".to_string(),
        task,
        routing: RouteRequestEnvelopeV2 {
            schema_version: 2,
            request: RouteRequestV2 {
                schema_version: 2,
                action: "implementation".to_string(),
                required_capabilities: BTreeSet::from([Capability::Write]),
                sensitivity: Sensitivity::Internal,
                required_context: 1,
                effort: None,
                minimum_quality: None,
                selection_margin: None,
                predicted_tokens: 100,
                allow_any_eligible: None,
                allow_unverified_quality: None,
            },
        },
        grant_id: "grant-1".to_string(),
    }
}

fn install_grant(root: &std::path::Path) {
    let grant = ExecutionGrantV1::new(
        "grant-1".to_string(),
        100,
        200,
        hash('0'),
        BTreeSet::from([route()]),
        BTreeSet::from(["implementation".to_string()]),
        BTreeSet::from([GrantOperation::Run]),
        GrantLimits {
            requests: 2,
            input_tokens: 1_000,
            output_tokens: 1_000,
            wall_time_ms: 20_000,
        },
    )
    .unwrap();
    GrantStore::open(root.join("grants"))
        .append(&grant)
        .unwrap();
}

#[test]
fn ninguna_invocacion_empieza_sin_grant_reserva_y_journal_sincronizado() {
    let root = root();
    install_state(&root);
    install_grant(&root);
    let fake = FakeHarnessExecutor::new(NormalizedInvocationResult {
        output: "done".to_string(),
        usage: TokenUsage {
            input_tokens: 50,
            output_tokens: 20,
        },
        latency_ms: 5,
        provenance: Some("fake".to_string()),
        manifest_hash: None,
        failure: None,
    });
    let runtime = TestRuntime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &fake, &runtime, &runtime);
    let status = coordinator.execute(request("run-1")).unwrap();

    assert_eq!(fake.invocation_count(), 1);
    assert_eq!(status.phase, RunPhaseV2::Completed);
    assert_eq!(
        status.consumed,
        BudgetAmount {
            requests: 1,
            input_tokens: 50,
            output_tokens: 20,
            wall_time_ms: 5,
        }
    );
    assert_eq!(
        status
            .journal
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        vec![
            RunJournalKindV2::Planned,
            RunJournalKindV2::Reserved,
            RunJournalKindV2::InvocationStarted,
            RunJournalKindV2::InvocationSucceeded,
        ]
    );
    std::fs::remove_dir_all(root).unwrap();
}

struct PanicExecutor;

impl HarnessExecutor for PanicExecutor {
    fn invoke(
        &self,
        _request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        panic!("simulated crash after invocation_started")
    }
}

#[test]
fn reinicio_tras_inicio_ambiguo_no_reenvia() {
    let root = root();
    install_state(&root);
    install_grant(&root);
    let runtime = TestRuntime::new(120_000);
    let crashing = RunCoordinator::with_runtime(root.clone(), &PanicExecutor, &runtime, &runtime);
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = crashing.execute(request("run-crash"));
    }));
    assert!(crashed.is_err());

    let fake = FakeHarnessExecutor::new(NormalizedInvocationResult {
        output: "must-not-run".to_string(),
        usage: TokenUsage::default(),
        latency_ms: 0,
        provenance: None,
        manifest_hash: None,
        failure: None,
    });
    runtime.set(121_000);
    let resumed = RunCoordinator::with_runtime(root.clone(), &fake, &runtime, &runtime);
    let status = resumed.resume("run-crash").unwrap();

    assert_eq!(fake.invocation_count(), 0);
    assert_eq!(status.phase, RunPhaseV2::OutcomeUnknown);
    assert!(status.outcome_unknown);
    std::fs::remove_dir_all(root).unwrap();
}
