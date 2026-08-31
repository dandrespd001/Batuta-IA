//! Contrato de salud durable y checkpoint acotado.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_routing::{
    FailureCategory, HandoffCheckpoint, HandoffDraft, RecoveryAction, RouteHealth, TestFact,
    TestStatus, record_failure,
};

fn route(value: &str) -> RouteRef {
    RouteRef::from_str(value).unwrap()
}

#[test]
fn retry_after_corto_reanuda_la_misma_ruta() {
    let result = record_failure(
        &RouteHealth::healthy(),
        &route("dsh/deepseek/deepseek-v4"),
        FailureCategory::RateLimited {
            retry_after_seconds: 30,
        },
        1_000,
    );

    assert_eq!(result.action, RecoveryAction::RetrySameRoute { at: 1_030 });
    assert_eq!(result.health.cooldown_until, Some(1_030));
}

#[test]
fn minimax_sin_plazo_sondea_15_30_60_y_luego_cada_hora() {
    let minimax = route("dsh/minimax/minimax-m2.5");
    let mut health = RouteHealth::healthy();
    let mut now = 1_000;
    let mut waits = Vec::new();
    for _ in 0..4 {
        let result = record_failure(&health, &minimax, FailureCategory::RateLimitedUnknown, now);
        let RecoveryAction::ProbeSameRoute { at } = result.action else {
            panic!("se esperaba sonda");
        };
        waits.push(at - now);
        health = result.health;
        now = at;
    }
    assert_eq!(waits, vec![900, 1_800, 3_600, 3_600]);
}

#[test]
fn autenticacion_y_saldo_bloquean_hasta_intervencion_del_harness() {
    for category in [FailureCategory::Authentication, FailureCategory::Balance] {
        let result = record_failure(
            &RouteHealth::healthy(),
            &route("abacus/abacus/glm-5.3"),
            category,
            1_000,
        );
        assert_eq!(result.action, RecoveryAction::FallbackImmediately);
        assert!(result.health.blocked_by_harness);
    }
}

#[test]
fn checkpoint_valida_objetivo_fallo_siguiente_paso_y_rutas_relativas() {
    let checkpoint = HandoffCheckpoint::try_from(HandoffDraft {
        schema_version: 1,
        objective: "implementar selector".to_string(),
        constraints: vec!["sin llamadas paralelas".to_string()],
        decisions: vec!["usar coste esperado".to_string()],
        files: vec!["src/lib.rs".to_string()],
        diff_summary: "tests rojos".to_string(),
        tests: vec![TestFact {
            command: "cargo test".to_string(),
            status: TestStatus::Failed,
            summary: "selector no existe".to_string(),
        }],
        failure: FailureCategory::QuotaExhausted,
        failure_message: "cuota agotada".to_string(),
        next_step: "usar fallback aprobado".to_string(),
        remaining_tokens: 12_000,
        remaining_wall_seconds: 900,
    })
    .unwrap();

    let json = serde_json::to_string(&checkpoint).unwrap();
    assert!(!json.contains("history"));
    assert_eq!(
        serde_json::from_str::<HandoffCheckpoint>(&json).unwrap(),
        checkpoint
    );

    let mut bad: HandoffDraft = checkpoint.into();
    bad.files = vec!["/etc/passwd".to_string()];
    assert!(
        HandoffCheckpoint::try_from(bad)
            .unwrap_err()
            .to_string()
            .contains("relative")
    );
}
