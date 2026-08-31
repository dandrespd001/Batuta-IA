//! Límites y categorías de recuperación exigidos por K4.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use batuta_exec::{
    ExecutorError, HarnessExecutor, InvocationFailure, InvocationRequestV2,
    NormalizedInvocationResult, TokenUsage,
};
use batuta_routing::{
    BudgetAmount, ExecutionPolicyV2, GrantLimits, GrantStore, LedgerStore, RunCoordinator,
    RunJournalKindV2, RunPhaseV2, RunStatusV2,
};

mod support;

use support::coordinator::*;

fn standard_limits() -> GrantLimits {
    GrantLimits {
        requests: 8,
        input_tokens: 2_000,
        output_tokens: 2_000,
        wall_time_ms: 100_000,
    }
}

#[derive(Debug)]
struct DurabilityInspectingExecutor {
    root: std::path::PathBuf,
}

impl HarnessExecutor for DurabilityInspectingExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        let status: RunStatusV2 = serde_json::from_slice(
            &std::fs::read(
                self.root
                    .join("runs")
                    .join(format!("{}.json", request.run_id)),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(status.phase, RunPhaseV2::InvocationStarted);
        assert_eq!(
            status.journal.last().map(|event| event.kind),
            Some(RunJournalKindV2::InvocationStarted)
        );
        let ledger = LedgerStore::open(self.root.join("ledger"), self.root.join("budget-leases"))
            .status("grant-1")
            .unwrap();
        assert!(
            ledger
                .reservations
                .contains_key(&status.attempts.last().unwrap().reservation_id)
        );
        Ok(NormalizedInvocationResult {
            output: "done".to_string(),
            usage: TokenUsage {
                input_tokens: 3,
                output_tokens: 2,
            },
            latency_ms: 5,
            provenance: Some("fake/durable/r1".to_string()),
            manifest_hash: Some(hash('f')),
            failure: None,
        })
    }
}

#[test]
fn reserva_y_invocation_started_estan_sincronizados_antes_del_ejecutor() {
    let root = root("durable-before-invoke");
    install_state(
        &root,
        &[route_one()],
        ExecutionPolicyV2::new(1, 500, 0).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one()]));
    let executor = DurabilityInspectingExecutor { root: root.clone() };
    let runtime = Runtime::new(120_000);

    let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
        .execute(request("run-durable"))
        .unwrap();

    assert_eq!(status.phase, RunPhaseV2::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retry_after_fuera_de_politica_hace_fallback_sin_dormir() {
    let root = root("retry-policy-limit");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(3, 500, 1).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one(), route_two()]));
    let executor = ScriptedExecutor::new(vec![
        result(
            Some(InvocationFailure::RateLimited {
                retry_after_ms: Some(501),
            }),
            "limited",
            10,
        ),
        result(None, "done", 5),
    ]);
    let runtime = Runtime::new(120_000);

    let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
        .execute(request("run-policy-limit"))
        .unwrap();

    assert_eq!(runtime.sleeps(), Vec::<u64>::new());
    assert_eq!(executor.requests()[1].route, route_two());
    assert_eq!(status.phase, RunPhaseV2::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retry_after_que_supera_caducidad_y_deadline_hace_fallback_inmediato() {
    let root = root("retry-deadline-limit");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(3, 5_000, 1).unwrap(),
    );
    install_grant_with_limits(
        &root,
        BTreeSet::from([route_one(), route_two()]),
        100,
        121,
        standard_limits(),
    );
    let executor = ScriptedExecutor::new(vec![
        result(
            Some(InvocationFailure::RateLimited {
                retry_after_ms: Some(1_000),
            }),
            "limited",
            10,
        ),
        result(None, "done", 5),
    ]);
    let runtime = Runtime::new(120_000);

    let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
        .execute(request("run-deadline-limit"))
        .unwrap();

    assert!(runtime.sleeps().is_empty());
    assert_eq!(executor.requests()[1].route, route_two());
    assert_eq!(status.phase, RunPhaseV2::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[derive(Debug)]
struct BudgetPressureExecutor {
    root: std::path::PathBuf,
    calls: AtomicUsize,
    scripted: ScriptedExecutor,
}

impl HarnessExecutor for BudgetPressureExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        if self.calls.fetch_add(1, Ordering::AcqRel) == 0 {
            let grant = GrantStore::open(self.root.join("grants"))
                .status("grant-1")
                .unwrap()
                .grant;
            LedgerStore::open(self.root.join("ledger"), self.root.join("budget-leases"))
                .reserve(
                    &grant,
                    "external-pressure",
                    BudgetAmount {
                        wall_time_ms: 89_990,
                        ..BudgetAmount::default()
                    },
                )
                .unwrap();
        }
        self.scripted.invoke(request)
    }
}

#[test]
fn retry_que_no_cabe_en_presupuesto_no_duerme_y_usa_fallback_que_si_cabe() {
    let root = root("retry-budget-limit");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(3, 5_000, 1).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one(), route_two()]));
    let executor = BudgetPressureExecutor {
        root: root.clone(),
        calls: AtomicUsize::new(0),
        scripted: ScriptedExecutor::new(vec![
            result(
                Some(InvocationFailure::RateLimited {
                    retry_after_ms: Some(1_000),
                }),
                "limited",
                10,
            ),
            result(None, "done", 5),
        ]),
    };
    let runtime = Runtime::new(120_000);

    let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
        .execute(request("run-budget-limit"))
        .unwrap();

    assert!(runtime.sleeps().is_empty());
    assert_eq!(executor.scripted.requests()[1].route, route_two());
    assert_eq!(status.phase, RunPhaseV2::Completed);
    assert!(status.consumed.wall_time_ms <= standard_limits().wall_time_ms);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cada_fallo_conocido_sin_retry_valido_releva_sin_historial() {
    let cases = [
        (
            "rate-limit-unknown",
            InvocationFailure::RateLimited {
                retry_after_ms: None,
            },
            "rate_limited_unknown",
        ),
        ("quota", InvocationFailure::Quota, "quota_exhausted"),
        (
            "authentication",
            InvocationFailure::Authentication,
            "authentication",
        ),
        ("balance", InvocationFailure::Balance, "balance"),
        ("timeout", InvocationFailure::Timeout, "timeout"),
        ("transient", InvocationFailure::Transient, "transient"),
        ("permanent", InvocationFailure::Permanent, "permanent"),
    ];

    for (label, failure, category) in cases {
        let root = root(label);
        install_state(
            &root,
            &[route_one(), route_two()],
            ExecutionPolicyV2::new(3, 5_000, 1).unwrap(),
        );
        install_grant(&root, BTreeSet::from([route_one(), route_two()]));
        let executor = ScriptedExecutor::new(vec![
            result(Some(failure), "HISTORY_SENTINEL", 10),
            result(None, "done", 5),
        ]);
        let runtime = Runtime::new(120_000);

        let status = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime)
            .execute(request(&format!("run-{label}")))
            .unwrap();

        let invocations = executor.requests();
        assert_eq!(invocations.len(), 2, "case {label}");
        assert_eq!(invocations[1].route, route_two(), "case {label}");
        assert!(
            !invocations[1].objective.contains("HISTORY_SENTINEL"),
            "case {label}"
        );
        let checkpoint = serde_json::to_string(status.checkpoint.as_ref().unwrap()).unwrap();
        assert!(checkpoint.contains(category), "case {label}: {checkpoint}");
        assert!(!checkpoint.contains("HISTORY_SENTINEL"), "case {label}");
        assert_eq!(status.phase, RunPhaseV2::Completed, "case {label}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
