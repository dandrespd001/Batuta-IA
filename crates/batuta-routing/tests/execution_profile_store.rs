//! Staging, CAS y confirmación del perfil operativo.

use std::time::{SystemTime, UNIX_EPOCH};

use batuta_exec::{ExecutionProfileDraftV1, ExecutionProfileV1};
use batuta_routing::{EMPTY_EXECUTION_PROFILE_HASH, ExecutionProfileStore};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-profile-store-{nonce}"))
}

fn profile(root: &std::path::Path) -> ExecutionProfileV1 {
    std::fs::create_dir_all(root.join("work")).unwrap();
    ExecutionProfileV1::seal(ExecutionProfileDraftV1 {
        schema_version: 1,
        workdir: root.join("work"),
        max_stdout_bytes: 4_096,
        max_stderr_bytes: 2_048,
        termination_grace_ms: 500,
    })
    .unwrap()
}

#[test]
fn import_status_y_apply_comparten_staging_sin_autoaplicar() {
    let root = root();
    let store = ExecutionProfileStore::open(root.join("profiles"), root.join("leases"));
    let proposed = profile(&root);

    let proposal = store.stage("profile-1", 100, proposed.clone()).unwrap();
    assert_eq!(proposal.expected_active_hash, EMPTY_EXECUTION_PROFILE_HASH);
    assert_eq!(proposal.proposed_profile_hash, proposed.profile_hash());
    assert!(proposal.diff.contains("workdir"));
    assert!(proposal.proposal_hash.starts_with("sha256:"));

    let staged = ExecutionProfileStore::open(root.join("profiles"), root.join("leases"))
        .status()
        .unwrap();
    assert!(staged.active.is_none());
    assert_eq!(staged.proposals, vec![proposal.clone()]);
    assert!(
        store
            .apply("profile-1", EMPTY_EXECUTION_PROFILE_HASH, false)
            .is_err()
    );
    assert!(
        store
            .apply(
                "profile-1",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                true,
            )
            .is_err()
    );

    let active = store
        .apply("profile-1", EMPTY_EXECUTION_PROFILE_HASH, true)
        .unwrap();
    assert_eq!(active, proposed);
    assert_eq!(store.status().unwrap().active, Some(active));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn propuesta_obsoleta_falla_por_cas_y_no_reemplaza_el_activo() {
    let root = root();
    let store = ExecutionProfileStore::open(root.join("profiles"), root.join("leases"));
    let first = profile(&root);
    store.stage("first", 100, first.clone()).unwrap();
    store
        .apply("first", EMPTY_EXECUTION_PROFILE_HASH, true)
        .unwrap();

    let mut second_draft = ExecutionProfileDraftV1 {
        schema_version: 1,
        workdir: root.join("work"),
        max_stdout_bytes: 8_192,
        max_stderr_bytes: 2_048,
        termination_grace_ms: 500,
    };
    let second = ExecutionProfileV1::seal(second_draft.clone()).unwrap();
    let proposal = store.stage("second", 101, second).unwrap();

    second_draft.max_stdout_bytes = 16_384;
    let competing = ExecutionProfileV1::seal(second_draft).unwrap();
    store.stage("competing", 102, competing.clone()).unwrap();
    store
        .apply("competing", first.profile_hash(), true)
        .unwrap();

    assert!(
        store
            .apply("second", &proposal.expected_active_hash, true)
            .is_err()
    );
    assert_eq!(store.status().unwrap().active, Some(competing));
    std::fs::remove_dir_all(root).unwrap();
}
