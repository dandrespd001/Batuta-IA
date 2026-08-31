//! Exclusión durable real entre procesos sobre el mismo run y grant.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use batuta_exec::{
    ExecutorError, HarnessExecutor, InvocationRequestV2, NormalizedInvocationResult, TokenUsage,
};
use batuta_routing::{ExecutionPolicyV2, GrantLimits, LedgerStore, RunCoordinator, RunPhaseV2};

mod support;

use support::coordinator::*;

const CHILD_ENV: &str = "BATUTA_COORDINATOR_MP_CHILD";
const ROOT_ENV: &str = "BATUTA_COORDINATOR_MP_ROOT";

#[derive(Debug)]
struct FileCountingExecutor {
    counter_dir: std::path::PathBuf,
    calls: AtomicUsize,
}

impl HarnessExecutor for FileCountingExecutor {
    fn invoke(
        &self,
        _request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        let marker = self
            .counter_dir
            .join(format!("invocation-{}-{call}", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker)
            .map_err(|error| ExecutorError::Configuration(error.to_string()))?;
        file.write_all(b"invoked\n")
            .map_err(|error| ExecutorError::Configuration(error.to_string()))?;
        file.sync_all()
            .map_err(|error| ExecutorError::Configuration(error.to_string()))?;
        Ok(NormalizedInvocationResult {
            output: "done".to_string(),
            usage: TokenUsage {
                input_tokens: 3,
                output_tokens: 2,
            },
            latency_ms: 5,
            provenance: Some("fake/multiprocess/r1".to_string()),
            manifest_hash: Some(hash('f')),
            failure: None,
        })
    }
}

fn wait_for_barrier(root: &std::path::Path) {
    let barrier = root.join("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !barrier.exists() {
        assert!(Instant::now() < deadline, "multiprocess barrier timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn child_worker() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    let root = std::path::PathBuf::from(std::env::var_os(ROOT_ENV).unwrap());
    wait_for_barrier(&root);
    let executor = FileCountingExecutor {
        counter_dir: root.join("invocations"),
        calls: AtomicUsize::new(0),
    };
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root, &executor, &runtime, &runtime);
    let _ = coordinator.execute(request("run-shared"));
}

#[test]
fn varios_procesos_producen_una_invocacion_y_no_exceden_el_presupuesto() {
    let root = root("multiprocess");
    std::fs::create_dir_all(root.join("invocations")).unwrap();
    install_state(
        &root,
        &[route_one()],
        ExecutionPolicyV2::new(1, 1_000, 0).unwrap(),
    );
    let limits = GrantLimits {
        requests: 1,
        input_tokens: 100,
        output_tokens: 100,
        wall_time_ms: 10_000,
    };
    install_grant_with_limits(&root, BTreeSet::from([route_one()]), 100, 1_000, limits);

    let executable = std::env::current_exe().unwrap();
    let mut children = (0..8)
        .map(|_| {
            std::process::Command::new(&executable)
                .arg("--exact")
                .arg("child_worker")
                .arg("--nocapture")
                .env(CHILD_ENV, "1")
                .env(ROOT_ENV, &root)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    std::fs::write(root.join("start"), b"go\n").unwrap();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }

    let invocation_count = std::fs::read_dir(root.join("invocations")).unwrap().count();
    assert_eq!(invocation_count, 1);

    let executor = FileCountingExecutor {
        counter_dir: root.join("invocations"),
        calls: AtomicUsize::new(0),
    };
    let runtime = Runtime::new(120_000);
    let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
        .status("run-shared")
        .unwrap();
    assert_eq!(status.phase, RunPhaseV2::Completed);
    assert_eq!(status.total_reserved.requests, 1);
    assert_eq!(status.consumed.requests, 1);

    let ledger = LedgerStore::open(root.join("ledger"), root.join("budget-leases"))
        .status("grant-1")
        .unwrap();
    assert!(ledger.consumed.requests <= limits.requests);
    assert!(ledger.consumed.input_tokens <= limits.input_tokens);
    assert!(ledger.consumed.output_tokens <= limits.output_tokens);
    assert!(ledger.consumed.wall_time_ms <= limits.wall_time_ms);
    assert!(root.join("run-receipts/run-shared.json").is_file());
    std::fs::remove_dir_all(root).unwrap();
}
