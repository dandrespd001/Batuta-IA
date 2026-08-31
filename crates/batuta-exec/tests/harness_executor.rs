//! Contrato de una sola invocación normalizada.

use std::str::FromStr as _;

use batuta_contract::{RouteRef, TaskSpec};
use batuta_exec::{
    FakeHarnessExecutor, HarnessExecutor, InvocationFailure, InvocationRequestV2,
    NormalizedInvocationResult, TokenUsage,
};

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

fn result() -> NormalizedInvocationResult {
    NormalizedInvocationResult {
        output: "done".to_string(),
        usage: TokenUsage {
            input_tokens: 12,
            output_tokens: 4,
        },
        latency_ms: 8,
        provenance: Some("fake/model/revision".to_string()),
        manifest_hash: None,
        failure: None,
    }
}

#[test]
fn el_fake_hace_exactamente_una_invocacion_por_llamada() {
    let fake = FakeHarnessExecutor::new(result());
    let request = InvocationRequestV2 {
        run_id: "run-1".to_string(),
        route: RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap(),
        objective: "implement".to_string(),
        task: task(),
        max_output_bytes: 128,
        timeout_ms: 1_000,
    };

    assert_eq!(fake.invoke(&request).unwrap(), result());
    assert_eq!(fake.invocation_count(), 1);
}

#[test]
fn la_peticion_v2_es_cerrada_y_no_admite_limites_cero() {
    let request = InvocationRequestV2 {
        run_id: "run-closed".to_string(),
        route: RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap(),
        objective: "implement".to_string(),
        task: task(),
        max_output_bytes: 128,
        timeout_ms: 1_000,
    };
    request.validate().unwrap();

    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["argv"] = serde_json::json!(["--unsafe"]);
    assert!(serde_json::from_value::<InvocationRequestV2>(unknown).is_err());

    let mut invalid = request;
    invalid.timeout_ms = 0;
    assert!(invalid.validate().is_err());
}

#[test]
fn la_taxonomia_cubre_fallos_operativos_sin_consultar_cuenta() {
    let failures = [
        InvocationFailure::RateLimited {
            retry_after_ms: Some(1_000),
        },
        InvocationFailure::RateLimited {
            retry_after_ms: None,
        },
        InvocationFailure::Quota,
        InvocationFailure::Authentication,
        InvocationFailure::Balance,
        InvocationFailure::Transient,
        InvocationFailure::Timeout,
        InvocationFailure::Permanent,
    ];
    assert_eq!(failures.len(), 8);
}
