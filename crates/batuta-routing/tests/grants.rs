//! Grants y presupuesto durable.

use std::collections::BTreeSet;
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::RouteRef;
use batuta_routing::{
    BudgetAmount, ExecutionGrantDraftV1, ExecutionGrantV1, GrantLimits, GrantOperation, GrantStore,
    LedgerStore,
};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-grant-{nonce}"))
}

fn route() -> RouteRef {
    RouteRef::from_str("dsh/minimax/MiniMax-M2.1").unwrap()
}

fn grant() -> ExecutionGrantV1 {
    ExecutionGrantV1::new(
        "grant-1".to_string(),
        100,
        200,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        BTreeSet::from([route()]),
        BTreeSet::from(["implementation".to_string()]),
        BTreeSet::from([GrantOperation::Run]),
        GrantLimits {
            requests: 2,
            input_tokens: 1_000,
            output_tokens: 500,
            wall_time_ms: 10_000,
        },
    )
    .unwrap()
}

#[test]
fn grant_es_cerrado_sellado_y_nunca_admite_limites_cero() {
    let grant = grant();
    assert!(grant.validate_at(150).is_ok());
    assert!(grant.grant_hash.starts_with("sha256:"));

    let mut value = serde_json::to_value(&grant).unwrap();
    value["private_endpoint"] = serde_json::json!("https://secret.invalid");
    assert!(serde_json::from_value::<ExecutionGrantV1>(value).is_err());

    let mut invalid = grant;
    invalid.limits.requests = 0;
    assert!(invalid.validate_at(150).is_err());

    assert!(
        ExecutionGrantV1::new(
            "zero-limit".to_string(),
            100,
            200,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            BTreeSet::from([route()]),
            BTreeSet::from(["implementation".to_string()]),
            BTreeSet::from([GrantOperation::Run]),
            GrantLimits {
                requests: 0,
                input_tokens: 1_000,
                output_tokens: 500,
                wall_time_ms: 10_000,
            },
        )
        .is_err()
    );
}

#[test]
fn borrador_cerrado_se_convierte_en_un_grant_sellado_sin_aceptar_hash_del_cliente() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "grant-draft",
        "issued_at": 100,
        "expires_at": 200,
        "manifest_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "routes": [route()],
        "actions": ["implementation"],
        "operations": ["run"],
        "limits": {
            "requests": 2,
            "input_tokens": 1000,
            "output_tokens": 500,
            "wall_time_ms": 10000
        }
    });
    let draft: ExecutionGrantDraftV1 = serde_json::from_value(json.clone()).unwrap();
    let grant = draft.seal().unwrap();
    assert_eq!(grant.id, "grant-draft");
    assert!(grant.validate_at(150).is_ok());

    let mut with_client_hash = json;
    with_client_hash["grant_hash"] = serde_json::json!(grant.grant_hash);
    assert!(serde_json::from_value::<ExecutionGrantDraftV1>(with_client_hash).is_err());
}

#[test]
fn revocacion_impide_reservas_nuevas_y_el_ledger_nunca_excede_el_grant() {
    let root = root();
    let grants = GrantStore::open(root.join("grants"));
    let grant = grant();
    grants.append(&grant).unwrap();

    let ledger = LedgerStore::open(root.join("ledger"), root.join("leases"));
    let one = BudgetAmount {
        requests: 1,
        input_tokens: 600,
        output_tokens: 200,
        wall_time_ms: 4_000,
    };
    ledger.reserve(&grant, "run-1", one).unwrap();
    assert!(ledger.reserve(&grant, "run-2", one).is_err());

    grants.revoke("grant-1", 151, "operator").unwrap();
    assert!(grants.authorize("grant-1", 152).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resultado_ambiguo_conserva_la_reserva_completa() {
    let root = root();
    let grant = grant();
    let ledger = LedgerStore::open(root.join("ledger"), root.join("leases"));
    let reserved = BudgetAmount {
        requests: 1,
        input_tokens: 700,
        output_tokens: 400,
        wall_time_ms: 8_000,
    };
    ledger.reserve(&grant, "run-ambiguous", reserved).unwrap();
    ledger
        .mark_outcome_unknown(&grant, "run-ambiguous")
        .unwrap();

    let status = ledger.status("grant-1").unwrap();
    assert_eq!(status.consumed, reserved);
    assert!(status.reservations["run-ambiguous"].outcome_unknown);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn espera_e_intento_se_reservan_atomicamente_antes_de_dormir() {
    let root = root();
    let grant = grant();
    let ledger = LedgerStore::open(root.join("ledger"), root.join("leases"));
    let wait = BudgetAmount {
        requests: 0,
        input_tokens: 0,
        output_tokens: 0,
        wall_time_ms: 100,
    };
    let attempt = BudgetAmount {
        requests: 1,
        input_tokens: 100,
        output_tokens: 100,
        wall_time_ms: 1_000,
    };
    ledger
        .reserve_many(
            &grant,
            &[
                ("run-1-wait-1".to_string(), wait),
                ("run-1-attempt-2".to_string(), attempt),
            ],
        )
        .unwrap();
    let status = ledger.status("grant-1").unwrap();
    assert_eq!(status.reservations["run-1-wait-1"].reserved, wait);
    assert_eq!(status.reservations["run-1-attempt-2"].reserved, attempt);

    let impossible = BudgetAmount {
        requests: 2,
        input_tokens: 1_000,
        output_tokens: 500,
        wall_time_ms: 10_000,
    };
    assert!(
        ledger
            .reserve_many(
                &grant,
                &[
                    ("run-2-wait".to_string(), wait),
                    ("run-2-attempt".to_string(), impossible),
                ],
            )
            .is_err()
    );
    let after = ledger.status("grant-1").unwrap();
    assert!(!after.reservations.contains_key("run-2-wait"));
    assert!(!after.reservations.contains_key("run-2-attempt"));
    std::fs::remove_dir_all(root).unwrap();
}
