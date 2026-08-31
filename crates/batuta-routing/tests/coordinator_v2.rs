//! Recuperación K4: retry explícito, relevo compacto y resultado ambiguo.

use std::collections::BTreeSet;

use batuta_exec::InvocationFailure;
use batuta_routing::{
    ExecutionPolicyV2, RunCoordinator, RunCoordinatorError, RunJournalKindV2, RunPhaseV2,
};

mod support;

use support::coordinator::*;

#[test]
fn grant_se_intersecta_antes_de_seleccionar_y_no_autoriza_rutas_nuevas() {
    let root = root("intersection");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(1, 5_000, 0).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_two(), route_new()]));
    let executor = ScriptedExecutor::new(vec![result(None, "done", 5)]);
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime);

    let status = coordinator.execute(request("run-intersection")).unwrap();

    assert_eq!(executor.requests()[0].route, route_two());
    assert_eq!(status.phase, RunPhaseV2::Completed);
    assert!(status.receipt.is_some());
    assert!(root.join("run-receipts/run-intersection.json").is_file());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retry_solo_con_retry_after_reserva_espera_e_intento_antes_de_dormir() {
    let root = root("retry");
    install_state(
        &root,
        &[route_one()],
        ExecutionPolicyV2::new(3, 5_000, 0).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one()]));
    let executor = ScriptedExecutor::new(vec![
        result(
            Some(InvocationFailure::RateLimited {
                retry_after_ms: Some(1_000),
            }),
            "HISTORY_SENTINEL",
            10,
        ),
        result(None, "done", 5),
    ]);
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime);

    let status = coordinator.execute(request("run-retry")).unwrap();

    assert_eq!(executor.requests().len(), 2);
    assert_eq!(executor.requests()[0].route, route_one());
    assert_eq!(executor.requests()[1].route, route_one());
    assert_eq!(runtime.sleeps(), vec![1_000]);
    assert_eq!(status.attempts.len(), 2);
    assert_eq!(status.phase, RunPhaseV2::Completed);
    assert_eq!(status.total_reserved.requests, 2);
    assert_eq!(status.total_reserved.wall_time_ms, 21_000);
    assert_eq!(
        status
            .journal
            .iter()
            .filter(|event| event.kind == RunJournalKindV2::RetryScheduled)
            .count(),
        1
    );
    assert_eq!(status.consumed.requests, 2);
    assert_eq!(status.consumed.wall_time_ms, 1_015);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fallo_sin_retry_after_releva_sin_reenviar_salida_previa() {
    let root = root("handoff");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(3, 5_000, 1).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one(), route_two()]));
    let executor = ScriptedExecutor::new(vec![
        result(Some(InvocationFailure::Quota), "HISTORY_SENTINEL", 10),
        result(None, "done", 5),
    ]);
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime);

    let status = coordinator.execute(request("run-handoff")).unwrap();

    let invocations = executor.requests();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].route, route_one());
    assert_eq!(invocations[1].route, route_two());
    assert_ne!(invocations[1].objective, invocations[0].objective);
    assert!(!invocations[1].objective.contains("HISTORY_SENTINEL"));
    assert!(invocations[1].objective.contains("failure_message"));
    assert_eq!(status.handoffs, 1);
    assert!(status.checkpoint.is_some());
    assert_eq!(status.phase, RunPhaseV2::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resume_antes_de_la_espera_devuelve_probe_not_due_y_no_invoca() {
    let root = root("resume-wait");
    install_state(
        &root,
        &[route_one()],
        ExecutionPolicyV2::new(3, 5_000, 0).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one()]));
    let first = ScriptedExecutor::new(vec![result(
        Some(InvocationFailure::RateLimited {
            retry_after_ms: Some(2_000),
        }),
        "limited",
        10,
    )]);
    let crashing_runtime = Runtime::panicking(120_000);
    let coordinator =
        RunCoordinator::with_runtime(root.clone(), &first, &crashing_runtime, &crashing_runtime);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = coordinator.execute(request("run-wait"));
        }))
        .is_err()
    );

    let second = ScriptedExecutor::new(vec![result(None, "done", 5)]);
    let runtime = Runtime::new(120_000);
    let resumed = RunCoordinator::with_runtime(root.clone(), &second, &runtime, &runtime);
    assert!(matches!(
        resumed.resume("run-wait"),
        Err(RunCoordinatorError::ProbeNotDue { .. })
    ));
    assert!(second.requests().is_empty());

    runtime.advance(2_000);
    let status = resumed.resume("run-wait").unwrap();
    assert_eq!(second.requests().len(), 1);
    assert_eq!(status.phase, RunPhaseV2::Completed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn err_despues_de_invocation_started_es_ambiguo_y_prohibe_relevo() {
    let root = root("unknown");
    install_state(
        &root,
        &[route_one(), route_two()],
        ExecutionPolicyV2::new(3, 5_000, 2).unwrap(),
    );
    install_grant(&root, BTreeSet::from([route_one(), route_two()]));
    let executor = ScriptedExecutor::new(vec![Step::Error, result(None, "forbidden", 1)]);
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime);

    let status = coordinator.execute(request("run-unknown")).unwrap();

    assert_eq!(executor.requests().len(), 1);
    assert_eq!(status.phase, RunPhaseV2::OutcomeUnknown);
    assert!(status.outcome_unknown);
    assert_eq!(status.consumed, status.total_reserved);
    assert!(status.receipt.is_some());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn manifest_ausente_impide_cualquier_reserva_e_invocacion() {
    let root = root("no-manifest");
    install_grant(&root, BTreeSet::from([route_one()]));
    let executor = ScriptedExecutor::new(vec![result(None, "forbidden", 1)]);
    let runtime = Runtime::new(120_000);
    let coordinator = RunCoordinator::with_runtime(root.clone(), &executor, &runtime, &runtime);

    assert!(coordinator.execute(request("run-no-manifest")).is_err());
    assert!(executor.requests().is_empty());
    assert!(!root.join("ledger/grant-1.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}
