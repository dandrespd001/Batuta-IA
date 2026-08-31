//! Infraestructura falsa compartida por las pruebas del coordinador K4.

#![allow(dead_code)] // Cada binario de integración usa un subconjunto distinto.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::str::FromStr as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity, TaskSpec};
use batuta_exec::{
    ExecutorError, HarnessExecutor, InvocationFailure, InvocationRequestV2,
    NormalizedInvocationResult, TokenUsage,
};
use batuta_quality::QualityProjection;
use batuta_routing::{
    CapabilityIndexEntryV2, CapabilityIndexV2, CatalogRouteStateV2, CatalogStateV2,
    EvidenceStateV2, ExecutionGrantV1, ExecutionPolicyV2, GrantLimits, GrantOperation, GrantStore,
    HealthStateV2, PolicyRouteStateV2, PolicyStateV2, RouteClass, RouteHealth,
    RouteRequestEnvelopeV2, RouteRequestV2, RoutingActionProfile, RunClock, RunRequestV2,
    RunSleeper, SelectionMargin, StateComponentsV2, StateStore,
};

pub(crate) fn root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "batuta-coordinator-v2-{label}-{}-{nonce}",
        std::process::id()
    ))
}

pub(crate) fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub(crate) fn route_one() -> RouteRef {
    RouteRef::from_str("dsh/one/model-v1/r1").unwrap()
}

pub(crate) fn route_two() -> RouteRef {
    RouteRef::from_str("dsh/two/model-v2/r2").unwrap()
}

pub(crate) fn route_new() -> RouteRef {
    RouteRef::from_str("dsh/new/not-in-manifest/r1").unwrap()
}

fn task() -> TaskSpec {
    serde_json::from_str(
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
    .unwrap()
}

pub(crate) fn request(id: &str) -> RunRequestV2 {
    RunRequestV2 {
        schema_version: 2,
        id: id.to_string(),
        objective: "implement objective only".to_string(),
        task: task(),
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

fn route_components(cost: f64) -> (CatalogRouteStateV2, PolicyRouteStateV2) {
    (
        CatalogRouteStateV2 {
            class: RouteClass::Production,
            max_sensitivity: Sensitivity::Internal,
            context_window: 100_000,
            supported_efforts: BTreeSet::from([ReasoningEffort::High]),
        },
        PolicyRouteStateV2 {
            alias: None,
            enabled: true,
            relative_cost: cost,
            handoff_penalty: 0.0,
            approved_fallback: true,
        },
    )
}

fn projection(route: RouteRef, score: f64, marker: char) -> QualityProjection {
    QualityProjection {
        route,
        action: "implementation".to_string(),
        researched_score: Some(score),
        effective_score: Some(score),
        coverage: 100,
        contributing_range: None,
        verified: true,
        contributions: vec![],
        exclusions: vec![],
        override_history: vec![],
        active_override: None,
        evidence_hash: hash(marker),
    }
}

pub(crate) fn install_state(
    root: &std::path::Path,
    routes: &[RouteRef],
    policy: ExecutionPolicyV2,
) {
    let mut catalog = BTreeMap::new();
    let mut policy_routes = BTreeMap::new();
    let mut evidence = Vec::new();
    let mut health = BTreeMap::new();
    let mut capabilities = BTreeMap::new();
    for (index, route) in routes.iter().enumerate() {
        let index = u32::try_from(index).unwrap();
        let (catalog_route, policy_route) = route_components(f64::from(index + 1));
        catalog.insert(route.clone(), catalog_route);
        policy_routes.insert(route.clone(), policy_route);
        evidence.push(projection(route.clone(), 95.0 - f64::from(index), 'a'));
        health.insert(route.clone(), RouteHealth::healthy());
        capabilities.insert(
            route.clone(),
            CapabilityIndexEntryV2 {
                capabilities: BTreeSet::from([Capability::Write]),
                receipt_hashes: BTreeSet::from([hash('1')]),
                expires_at: 10_000,
            },
        );
    }
    StateStore::open(root.join("state-v2"))
        .commit(&StateComponentsV2 {
            catalog: CatalogStateV2 {
                schema_version: 2,
                routes: catalog,
            },
            policy: PolicyStateV2 {
                schema_version: 2,
                execution: policy,
                profiles: BTreeMap::from([(
                    "implementation".to_string(),
                    RoutingActionProfile {
                        action: "implementation".to_string(),
                        minimum_quality: 80.0,
                        selection_margin: SelectionMargin::new(20.0).unwrap(),
                        allow_any_eligible: false,
                        allow_unverified_quality: false,
                    },
                )]),
                routes: policy_routes,
            },
            evidence: EvidenceStateV2 {
                schema_version: 2,
                projections: evidence,
            },
            health: HealthStateV2 {
                schema_version: 2,
                routes: health,
            },
            capabilities: CapabilityIndexV2 {
                schema_version: 2,
                routes: capabilities,
            },
        })
        .unwrap();
}

pub(crate) fn install_grant(root: &std::path::Path, routes: BTreeSet<RouteRef>) {
    install_grant_with_limits(
        root,
        routes,
        100,
        1_000,
        GrantLimits {
            requests: 8,
            input_tokens: 2_000,
            output_tokens: 2_000,
            wall_time_ms: 100_000,
        },
    );
}

pub(crate) fn install_grant_with_limits(
    root: &std::path::Path,
    routes: BTreeSet<RouteRef>,
    issued_at: u64,
    expires_at: u64,
    limits: GrantLimits,
) {
    let grant = ExecutionGrantV1::new(
        "grant-1".to_string(),
        issued_at,
        expires_at,
        hash('0'),
        routes,
        BTreeSet::from(["implementation".to_string()]),
        BTreeSet::from([GrantOperation::Run]),
        limits,
    )
    .unwrap();
    GrantStore::open(root.join("grants"))
        .append(&grant)
        .unwrap();
}

#[derive(Debug)]
pub(crate) enum Step {
    Known(NormalizedInvocationResult),
    Error,
}

#[derive(Debug)]
pub(crate) struct ScriptedExecutor {
    steps: Mutex<VecDeque<Step>>,
    requests: Mutex<Vec<InvocationRequestV2>>,
}

impl ScriptedExecutor {
    pub(crate) fn new(steps: Vec<Step>) -> Self {
        Self {
            steps: Mutex::new(steps.into()),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn requests(&self) -> Vec<InvocationRequestV2> {
        self.requests.lock().unwrap().clone()
    }
}

impl HarnessExecutor for ScriptedExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        self.requests.lock().unwrap().push(request.clone());
        match self.steps.lock().unwrap().pop_front().unwrap() {
            Step::Known(result) => Ok(result),
            Step::Error => Err(ExecutorError::Configuration("observed error".to_string())),
        }
    }
}

pub(crate) fn result(failure: Option<InvocationFailure>, output: &str, latency_ms: u64) -> Step {
    Step::Known(NormalizedInvocationResult {
        output: output.to_string(),
        usage: TokenUsage {
            input_tokens: 3,
            output_tokens: 2,
        },
        latency_ms,
        provenance: Some("fake/model/r1".to_string()),
        manifest_hash: Some(hash('f')),
        failure,
    })
}

#[derive(Debug)]
pub(crate) struct Runtime {
    now_ms: AtomicU64,
    sleeps: Mutex<Vec<u64>>,
    panic_on_sleep: bool,
}

impl Runtime {
    pub(crate) fn new(now_ms: u64) -> Self {
        Self {
            now_ms: AtomicU64::new(now_ms),
            sleeps: Mutex::new(Vec::new()),
            panic_on_sleep: false,
        }
    }

    pub(crate) fn panicking(now_ms: u64) -> Self {
        Self {
            panic_on_sleep: true,
            ..Self::new(now_ms)
        }
    }

    pub(crate) fn advance(&self, millis: u64) {
        self.now_ms.fetch_add(millis, Ordering::AcqRel);
    }

    pub(crate) fn sleeps(&self) -> Vec<u64> {
        self.sleeps.lock().unwrap().clone()
    }
}

impl RunClock for Runtime {
    fn now_millis(&self) -> u64 {
        self.now_ms.load(Ordering::Acquire)
    }
}

impl RunSleeper for Runtime {
    fn sleep_millis(&self, millis: u64) {
        self.sleeps.lock().unwrap().push(millis);
        assert!(!self.panic_on_sleep, "crash during durable wait");
        self.advance(millis);
    }
}
