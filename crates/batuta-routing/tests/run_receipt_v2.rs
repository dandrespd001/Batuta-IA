//! Recibo final exhaustivo, sellado y append-only.

use std::collections::BTreeSet;
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{Capability, RouteRef, Sensitivity, TaskSpec};
use batuta_exec::TokenUsage;
use batuta_routing::{
    BudgetAmount, ExecutionGrantV1, GrantLimits, GrantOperation, RouteRequestEnvelopeV2,
    RouteRequestV2, RunCandidateReceiptV2, RunConsumptionReceiptV2, RunDecisionReceiptV2,
    RunJournalEventV2, RunJournalKindV2, RunPhaseV2, RunReceiptDraftV2, RunReceiptStoreV2,
    RunReceiptV2, RunRequestV2, RunReservationKindV2, RunReservationReceiptV2, RunResultReceiptV2,
};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-run-receipt-{nonce}"))
}

fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn route() -> RouteRef {
    RouteRef::from_str("dsh/minimax/model-v1/r1").unwrap()
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

fn request() -> RunRequestV2 {
    RunRequestV2 {
        schema_version: 2,
        id: "run-1".to_string(),
        objective: "implement".to_string(),
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

fn receipt() -> RunReceiptV2 {
    let route = route();
    let grant = ExecutionGrantV1::new(
        "grant-1".to_string(),
        100,
        200,
        hash('a'),
        BTreeSet::from([route.clone()]),
        BTreeSet::from(["implementation".to_string()]),
        BTreeSet::from([GrantOperation::Run]),
        GrantLimits {
            requests: 1,
            input_tokens: 100,
            output_tokens: 100,
            wall_time_ms: 10_000,
        },
    )
    .unwrap();
    let amount = BudgetAmount {
        requests: 1,
        input_tokens: 100,
        output_tokens: 100,
        wall_time_ms: 10_000,
    };
    RunReceiptV2::seal(RunReceiptDraftV2 {
        schema_version: 2,
        id: "run-1".to_string(),
        created_at: 100,
        request: request(),
        grant_hash: grant.grant_hash.clone(),
        grant,
        candidates: vec![RunCandidateReceiptV2 {
            route: route.clone(),
            action: "implementation".to_string(),
            candidate_hash: hash('c'),
        }],
        discards: vec![],
        decisions: vec![RunDecisionReceiptV2 {
            attempt: 1,
            manifest_hash: hash('a'),
            route: route.clone(),
            candidate_hash: hash('c'),
        }],
        reservations: vec![RunReservationReceiptV2 {
            id: "run-1-attempt-1".to_string(),
            kind: RunReservationKindV2::Attempt,
            amount,
        }],
        consumptions: vec![RunConsumptionReceiptV2 {
            id: "run-1-attempt-1".to_string(),
            amount: BudgetAmount {
                requests: 1,
                input_tokens: 10,
                output_tokens: 5,
                wall_time_ms: 20,
            },
            known: true,
        }],
        transitions: vec![
            RunJournalEventV2 {
                sequence: 0,
                at: 100,
                kind: RunJournalKindV2::Planned,
                route: Some(route.clone()),
            },
            RunJournalEventV2 {
                sequence: 1,
                at: 101,
                kind: RunJournalKindV2::InvocationSucceeded,
                route: Some(route.clone()),
            },
        ],
        results: vec![RunResultReceiptV2 {
            attempt: 1,
            route,
            output: Some("done".to_string()),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
            latency_ms: 20,
            provenance: Some("fake/model/r1".to_string()),
            provider_manifest_hash: Some(hash('d')),
            failure: None,
            outcome_unknown: false,
        }],
        checkpoints: vec![],
        final_phase: RunPhaseV2::Completed,
    })
    .unwrap()
}

#[test]
fn reinicio_conserva_bytes_y_un_id_no_se_sobrescribe() {
    let root = root();
    let receipt = receipt();
    let store = RunReceiptStoreV2::open(root.clone());
    store.append(&receipt).unwrap();
    let before = std::fs::read(root.join("run-1.json")).unwrap();
    let loaded = RunReceiptStoreV2::open(root.clone()).load("run-1").unwrap();
    let after = std::fs::read(root.join("run-1.json")).unwrap();

    assert_eq!(loaded, receipt);
    assert_eq!(before, after);
    assert!(loaded.receipt_hash.starts_with("sha256:"));
    assert!(store.append(&receipt).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sello_alterado_o_estado_no_terminal_se_rechazan() {
    let receipt = receipt();
    let mut value = serde_json::to_value(&receipt).unwrap();
    value["grant_hash"] = serde_json::json!(hash('z'));
    let altered: RunReceiptV2 = serde_json::from_value(value).unwrap();
    assert!(altered.validate().is_err());

    let mut draft = receipt.into_draft();
    draft.final_phase = RunPhaseV2::Reserved;
    assert!(RunReceiptV2::seal(draft).is_err());
}
