//! Máquina de estados: una sola ruta activa y relevo por checkpoint.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_routing::{
    FailureCategory, HandoffCheckpoint, HandoffDraft, RecoveryAction, RunEvent, RunState,
    advance_run,
};

fn route(value: &str) -> RouteRef {
    RouteRef::from_str(value).unwrap()
}

fn checkpoint() -> HandoffCheckpoint {
    HandoffCheckpoint::try_from(HandoffDraft {
        schema_version: 1,
        objective: "terminar routing".to_string(),
        constraints: vec![],
        decisions: vec![],
        files: vec!["src/lib.rs".to_string()],
        diff_summary: String::new(),
        tests: vec![],
        failure: FailureCategory::QuotaExhausted,
        failure_message: "quota".to_string(),
        next_step: "fallback".to_string(),
        remaining_tokens: 1_000,
        remaining_wall_seconds: 60,
    })
    .unwrap()
}

#[test]
fn cuota_hace_checkpoint_selecciona_fallback_y_reanuda_sin_historial() {
    let first = route("dsh/minimax/minimax-m2.5");
    let fallback = route("codex/openai/gpt-5.6");
    let running = advance_run(
        &RunState::Planned,
        RunEvent::Start {
            route: first.clone(),
        },
    )
    .unwrap();
    let checkpointed = advance_run(
        &running,
        RunEvent::Failure {
            checkpoint: checkpoint(),
            recovery: RecoveryAction::FallbackImmediately,
        },
    )
    .unwrap();
    assert!(matches!(checkpointed, RunState::Checkpointed { .. }));

    let selected = advance_run(
        &checkpointed,
        RunEvent::SelectFallback {
            route: fallback.clone(),
        },
    )
    .unwrap();
    assert!(matches!(selected, RunState::FallbackSelected { .. }));
    let resumed = advance_run(&selected, RunEvent::Resume { now: 1_000 }).unwrap();
    assert!(matches!(
        resumed,
        RunState::Running { route, .. } if route == fallback
    ));
}

#[test]
fn una_ruta_en_ejecucion_no_admite_arrancar_otra_en_paralelo() {
    let running = RunState::Running {
        route: route("dsh/deepseek/deepseek-v4"),
        handoff: None,
    };
    let error = advance_run(
        &running,
        RunEvent::Start {
            route: route("codex/openai/gpt-5.6"),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("running"));
    assert!(error.to_string().contains("one active route"));
}
