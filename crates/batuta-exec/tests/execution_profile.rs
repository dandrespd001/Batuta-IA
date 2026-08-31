//! Perfil operativo cerrado, canónico y sellado.

use std::time::{SystemTime, UNIX_EPOCH};

use batuta_exec::{ExecutionProfileDraftV1, ExecutionProfileV1};

fn root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-profile-{label}-{nonce}"))
}

fn draft(workdir: std::path::PathBuf) -> ExecutionProfileDraftV1 {
    ExecutionProfileDraftV1 {
        schema_version: 1,
        workdir,
        max_stdout_bytes: 4_096,
        max_stderr_bytes: 2_048,
        termination_grace_ms: 500,
    }
}

#[test]
fn perfil_canoniza_el_workdir_y_detecta_un_sello_alterado() {
    let root = root("seal");
    std::fs::create_dir_all(root.join("work")).unwrap();
    let profile = ExecutionProfileV1::seal(draft(root.join("work/../work"))).unwrap();

    assert_eq!(
        profile.workdir(),
        std::fs::canonicalize(root.join("work")).unwrap()
    );
    assert!(profile.profile_hash().starts_with("sha256:"));
    profile.validate().unwrap();

    let mut value = serde_json::to_value(&profile).unwrap();
    value["profile_hash"] = serde_json::json!(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    let altered: ExecutionProfileV1 = serde_json::from_value(value).unwrap();
    assert!(altered.validate().is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn perfil_rechaza_campos_ajenos_limites_cero_raiz_y_workdir_obsoleto() {
    let root = root("invalid");
    std::fs::create_dir_all(root.join("work")).unwrap();

    let mut unknown = serde_json::to_value(draft(root.join("work"))).unwrap();
    unknown["program"] = serde_json::json!("/bin/sh");
    assert!(serde_json::from_value::<ExecutionProfileDraftV1>(unknown).is_err());

    let mut zero = draft(root.join("work"));
    zero.max_stdout_bytes = 0;
    assert!(ExecutionProfileV1::seal(zero).is_err());
    assert!(ExecutionProfileV1::seal(draft(std::path::PathBuf::from("/"))).is_err());

    let stale = ExecutionProfileV1::seal(draft(root.join("work"))).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(stale.validate().is_err());
}
