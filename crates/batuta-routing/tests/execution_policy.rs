//! Política mínima de intentos, Retry-After y relevos, sin defaults.

use batuta_routing::ExecutionPolicyV2;

#[test]
fn politica_es_cerrada_explicita_y_valida_limites() {
    let policy = ExecutionPolicyV2::new(3, 30_000, 2).unwrap();
    assert_eq!(policy.max_attempts, 3);
    assert_eq!(policy.max_retry_after_ms, 30_000);
    assert_eq!(policy.max_handoffs, 2);

    let missing = r#"{"schema_version":2,"max_attempts":3,"max_handoffs":2}"#;
    assert!(serde_json::from_str::<ExecutionPolicyV2>(missing).is_err());
    let unknown = r#"{"schema_version":2,"max_attempts":3,"max_retry_after_ms":30000,"max_handoffs":2,"backoff":"hidden"}"#;
    assert!(serde_json::from_str::<ExecutionPolicyV2>(unknown).is_err());

    assert!(ExecutionPolicyV2::new(0, 30_000, 2).is_err());
    assert!(ExecutionPolicyV2::new(3, 0, 2).is_err());
    assert!(ExecutionPolicyV2::new(1, 1, 0).is_ok());
}
