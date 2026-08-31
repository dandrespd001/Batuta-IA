//! La vista Execution usa contratos cerrados, preview puro y un worker único.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use batuta_cli::{
    Layout, OperationalApi, TuiApp, TuiExecutionJob, TuiExecutionSection, TuiExecutionWorker,
    TuiInputAction, run_json,
};
use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_exec::{
    ExecutorError, HarnessExecutor, InvocationRequestV2, NormalizedInvocationResult, TokenUsage,
};
use batuta_quality::QualityProjection;
use batuta_routing::{
    CapabilityIndexEntryV2, CapabilityIndexV2, CatalogRouteStateV2, CatalogStateV2,
    EvidenceStateV2, ExecutionPolicyV2, HealthStateV2, PolicyRouteStateV2, PolicyStateV2,
    RouteClass, RouteHealth, RoutingActionProfile, RunRequestV2, SelectionMargin,
    StateComponentsV2, StateStore,
};

fn root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-tui-execution-{label}-{nonce}"))
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

fn grant_json(manifest_hash: &str, now: u64) -> String {
    serde_json::json!({
        "schema_version": 1,
        "id": "grant-1",
        "issued_at": now.saturating_sub(1),
        "expires_at": now + 3_600,
        "manifest_hash": manifest_hash,
        "routes": [route()],
        "actions": ["implementation"],
        "operations": ["run"],
        "limits": {
            "requests": 2,
            "input_tokens": 10000,
            "output_tokens": 10000,
            "wall_time_ms": 60000
        }
    })
    .to_string()
}

fn request_json() -> String {
    serde_json::json!({
        "schema_version": 2,
        "id": "run-tui-1",
        "objective": "implement",
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

#[derive(Debug, Default)]
struct FakeExecutor {
    requests: Mutex<Vec<InvocationRequestV2>>,
}

impl FakeExecutor {
    fn requests_json(&self) -> serde_json::Value {
        serde_json::to_value(self.requests.lock().unwrap().clone()).unwrap()
    }
}

impl HarnessExecutor for FakeExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(NormalizedInvocationResult {
            output: "done".to_string(),
            usage: TokenUsage {
                input_tokens: 3,
                output_tokens: 2,
            },
            latency_ms: 5,
            provenance: Some("fake/parity/r1".to_string()),
            manifest_hash: Some(hash('f')),
            failure: None,
        })
    }
}

fn normalized_receipt(root: &std::path::Path) -> serde_json::Value {
    let bytes = std::fs::read(root.join("run-receipts/run-tui-1.json")).unwrap();
    let mut receipt: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let object = receipt.as_object_mut().unwrap();
    object.remove("created_at");
    object.remove("receipt_hash");
    for transition in object
        .get_mut("transitions")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        transition.as_object_mut().unwrap().remove("at");
    }
    receipt
}

fn enter(app: &mut TuiApp, layout: &Layout, worker: &TuiExecutionWorker, now_ms: u64, value: &str) {
    app.replace_execution_input(value);
    app.submit_execution_input(layout, worker, now_ms).unwrap();
}

#[test]
fn perfil_tui_hace_staging_y_exige_escribir_el_id_para_aplicar() {
    let root = root("profile");
    let workdir = root.join("workdir");
    std::fs::create_dir_all(&workdir).unwrap();
    let layout = Layout::under(root.clone());
    let mut app = TuiApp::new();
    let draft = serde_json::json!({
        "schema_version": 1,
        "workdir": workdir,
        "max_stdout_bytes": 4096,
        "max_stderr_bytes": 2048,
        "termination_grace_ms": 100
    })
    .to_string();

    let proposal = app
        .stage_execution_profile_json(&layout, &draft, 100)
        .unwrap();
    assert!(proposal.diff.contains("workdir"));
    assert!(
        app.apply_execution_profile(
            &layout,
            &proposal.id,
            "otro-id",
            &proposal.expected_active_hash,
            101,
        )
        .is_err()
    );
    app.apply_execution_profile(
        &layout,
        &proposal.id,
        &proposal.id,
        &proposal.expected_active_hash,
        101,
    )
    .unwrap();
    assert!(
        OperationalApi::new(&layout, 101)
            .profile_status()
            .unwrap()
            .data
            .active
            .is_some()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn preview_run_no_reserva_y_solo_encola_tras_escribir_el_run_id() {
    let root = root("preview");
    let layout = Layout::under(root.clone());
    let now = now_secs();
    let manifest_hash = install_state(&root, now + 3_600);
    OperationalApi::new(&layout, now)
        .grant_create_json(&grant_json(&manifest_hash, now), true)
        .unwrap();
    let mut app = TuiApp::new();

    let preview = app
        .preview_run_json(&layout, &request_json(), now * 1_000)
        .unwrap();
    assert_eq!(preview.manifest_hash, manifest_hash);
    assert_eq!(preview.route, route());
    assert_eq!(preview.grant_id, "grant-1");
    assert!(!preview.reserved);
    assert!(!root.join("ledger").exists());
    assert!(!root.join("runs").exists());

    let seen = Arc::new(Mutex::new(Vec::new()));
    let worker_seen = Arc::clone(&seen);
    let worker = TuiExecutionWorker::spawn(move |job| {
        worker_seen.lock().unwrap().push(job);
        Ok("queued".to_string())
    });
    assert!(app.queue_previewed_run(&worker, "otro-id").is_err());
    app.queue_previewed_run(&worker, "run-tui-1").unwrap();
    for _ in 0..100 {
        if worker.poll().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(matches!(
        &seen.lock().unwrap()[0],
        TuiExecutionJob::Run { request_json } if request_json.contains("run-tui-1")
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_cambia_entre_perfil_grants_y_runs() {
    let mut app = TuiApp::new();
    while app.view() != batuta_cli::TuiView::Execution {
        app.next_view();
    }
    assert_eq!(app.execution_section(), TuiExecutionSection::Profile);
    app.next_execution_section();
    assert_eq!(app.execution_section(), TuiExecutionSection::Grants);
    app.next_execution_section();
    assert_eq!(app.execution_section(), TuiExecutionSection::Runs);
}

#[test]
fn cli_formulario_tui_y_json_producen_request_decision_y_recibo_normalizados_iguales() {
    let cli_root = root("parity-cli");
    let tui_root = root("parity-tui");
    let cli_layout = Layout::under(cli_root.clone());
    let tui_layout = Layout::under(tui_root.clone());
    let now = now_secs();
    let cli_manifest = install_state(&cli_root, now + 3_600);
    let tui_manifest = install_state(&tui_root, now + 3_600);
    assert_eq!(cli_manifest, tui_manifest);
    for layout in [&cli_layout, &tui_layout] {
        OperationalApi::new(layout, now)
            .grant_create_json(&grant_json(&cli_manifest, now), true)
            .unwrap();
    }

    let input = request_json();
    let typed: RunRequestV2 = serde_json::from_str(&input).unwrap();
    let mut app = TuiApp::new();
    let form_preview = app.preview_run(&tui_layout, &typed, now * 1_000).unwrap();
    let json_preview = app
        .preview_run_json(&tui_layout, &input, now * 1_000)
        .unwrap();
    assert_eq!(form_preview, json_preview);

    let cli_executor = FakeExecutor::default();
    let cli_status = run_json(&cli_layout, &cli_executor, &input).unwrap().data;

    let tui_executor = Arc::new(FakeExecutor::default());
    let worker_executor = Arc::clone(&tui_executor);
    let worker_layout = tui_layout.clone();
    let worker = TuiExecutionWorker::spawn(move |job| match job {
        TuiExecutionJob::Run { request_json } => {
            run_json(&worker_layout, worker_executor.as_ref(), &request_json)
                .map(|response| serde_json::to_string(&response).unwrap())
        }
        TuiExecutionJob::Resume { .. } => unreachable!(),
    });
    app.queue_previewed_run(&worker, "run-tui-1").unwrap();
    let tui_output = (0..1_000)
        .find_map(|_| {
            let output = worker.poll();
            if output.is_none() {
                std::thread::sleep(Duration::from_millis(1));
            }
            output
        })
        .expect("TUI worker did not finish")
        .unwrap();
    let tui_response: serde_json::Value = serde_json::from_str(&tui_output).unwrap();

    assert_eq!(cli_executor.requests_json(), tui_executor.requests_json());
    assert_eq!(
        serde_json::to_value(&cli_status.request).unwrap(),
        tui_response["data"]["request"]
    );
    assert_eq!(
        serde_json::to_value(&cli_status.decisions).unwrap(),
        tui_response["data"]["decisions"]
    );
    assert_eq!(normalized_receipt(&cli_root), normalized_receipt(&tui_root));

    drop(worker);
    std::fs::remove_dir_all(cli_root).unwrap();
    std::fs::remove_dir_all(tui_root).unwrap();
}

#[test]
fn controles_interactivos_exponen_formulario_diff_y_confirmacion_del_perfil() {
    let root = root("interactive-profile");
    let workdir = root.join("workdir");
    std::fs::create_dir_all(&workdir).unwrap();
    let layout = Layout::under(root.clone());
    let worker = TuiExecutionWorker::spawn(|_| Ok("unused".to_string()));
    let mut app = TuiApp::new();

    app.begin_execution_input(TuiInputAction::ProfileForm)
        .unwrap();
    assert!(app.execution_input_prompt().unwrap().contains("workdir"));
    enter(
        &mut app,
        &layout,
        &worker,
        100_000,
        &workdir.display().to_string(),
    );
    enter(&mut app, &layout, &worker, 100_000, "4096");
    enter(&mut app, &layout, &worker, 100_000, "2048");
    enter(&mut app, &layout, &worker, 100_000, "100");

    let proposal = app.execution_profile_proposal().unwrap().clone();
    while app.view() != batuta_cli::TuiView::Execution {
        app.next_view();
    }
    let snapshot = app.snapshot(120);
    assert!(snapshot.contains(&proposal.id));
    assert!(snapshot.contains(&proposal.diff));

    app.begin_execution_input(TuiInputAction::ProfileApply)
        .unwrap();
    app.replace_execution_input("otro-id");
    assert!(
        app.submit_execution_input(&layout, &worker, 101_000)
            .is_err()
    );
    app.replace_execution_input(&proposal.id);
    app.submit_execution_input(&layout, &worker, 101_000)
        .unwrap();
    assert!(
        OperationalApi::new(&layout, 101)
            .profile_status()
            .unwrap()
            .data
            .active
            .is_some()
    );

    drop(worker);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn controles_interactivos_admiten_grant_run_formulario_json_preview_y_ejecucion() {
    let root = root("interactive-run");
    let layout = Layout::under(root.clone());
    let now = now_secs();
    let manifest_hash = install_state(&root, now + 3_600);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let worker_seen = Arc::clone(&seen);
    let worker = TuiExecutionWorker::spawn(move |job| {
        worker_seen.lock().unwrap().push(job);
        Ok("queued".to_string())
    });
    let mut app = TuiApp::new();

    app.begin_execution_input(TuiInputAction::GrantCreate)
        .unwrap();
    enter(
        &mut app,
        &layout,
        &worker,
        now * 1_000,
        &grant_json(&manifest_hash, now),
    );
    assert!(
        app.execution_input_prompt()
            .unwrap()
            .contains("confirmar grant ID")
    );
    enter(&mut app, &layout, &worker, now * 1_000, "grant-1");

    let request = serde_json::from_str::<serde_json::Value>(&request_json()).unwrap();
    app.begin_execution_input(TuiInputAction::RunForm).unwrap();
    enter(&mut app, &layout, &worker, now * 1_000, "run-tui-1");
    enter(&mut app, &layout, &worker, now * 1_000, "implement");
    enter(
        &mut app,
        &layout,
        &worker,
        now * 1_000,
        &request["task"].to_string(),
    );
    enter(
        &mut app,
        &layout,
        &worker,
        now * 1_000,
        &request["routing"].to_string(),
    );
    enter(&mut app, &layout, &worker, now * 1_000, "grant-1");
    let form_preview = app.execution_run_preview().unwrap().clone();
    assert!(!form_preview.reserved);
    assert!(!root.join("ledger").exists());

    app.begin_execution_input(TuiInputAction::RunJson).unwrap();
    enter(&mut app, &layout, &worker, now * 1_000, &request_json());
    assert_eq!(app.execution_run_preview(), Some(&form_preview));
    while app.view() != batuta_cli::TuiView::Execution {
        app.next_view();
    }
    app.next_execution_section();
    app.next_execution_section();
    let snapshot = app.snapshot(120);
    assert!(snapshot.contains(&manifest_hash));
    assert!(snapshot.contains("grant-1"));
    assert!(snapshot.contains("sin reserva"));

    app.begin_execution_input(TuiInputAction::RunExecute)
        .unwrap();
    app.replace_execution_input("otro-id");
    assert!(
        app.submit_execution_input(&layout, &worker, now * 1_000)
            .is_err()
    );
    app.replace_execution_input("run-tui-1");
    app.submit_execution_input(&layout, &worker, now * 1_000)
        .unwrap();
    for _ in 0..100 {
        if worker.poll().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(
        &seen.lock().unwrap()[0],
        TuiExecutionJob::Run { request_json } if request_json.contains("run-tui-1")
    ));

    drop(worker);
    std::fs::remove_dir_all(root).unwrap();
}
