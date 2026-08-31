//! Adaptador real construido sólo desde perfil y manifests confiables.

use std::os::unix::fs::PermissionsExt as _;
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{RouteRef, TaskSpec};
use batuta_exec::{
    ExecutionProfileDraftV1, ExecutionProfileV1, HarnessExecutor, InvocationFailure,
    InvocationRequestV2, ManifestHarnessExecutor,
};
use sha2::{Digest as _, Sha256};

fn root(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-manifest-executor-{label}-{nonce}"))
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

fn profile(root: &std::path::Path) -> ExecutionProfileV1 {
    std::fs::create_dir_all(root.join("work")).unwrap();
    ExecutionProfileV1::seal(ExecutionProfileDraftV1 {
        schema_version: 1,
        workdir: root.join("work"),
        max_stdout_bytes: 4_096,
        max_stderr_bytes: 4_096,
        termination_grace_ms: 1_000,
    })
    .unwrap()
}

fn executable_hash(path: &std::path::Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

fn manifest(program: &std::path::Path, version_pin: &str) -> String {
    format!(
        r#"schema_version = 1
id = "eco"
kind = "cli"

[executable]
program = "{}"
version_pin = "{version_pin}"
version_probe = ["--version"]
sha256 = "{}"
resolve = ["{}"]

[auth]
method = "oauth_cli"

[invoke]
argv = ["{{prompt}}"]
workdir = "worktree"
prompt = {{ via = "argv" }}

[env]
allow = ["PATH"]

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id = "eco-modelo"
route_model = "eco-modelo"
roles = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "{{token}}"
expect = "token_echo"
"#,
        program.display(),
        executable_hash(program),
        program.display(),
    )
}

fn request(route: &str, objective: &str) -> InvocationRequestV2 {
    InvocationRequestV2 {
        run_id: "run-1".to_string(),
        route: RouteRef::from_str(route).unwrap(),
        objective: objective.to_string(),
        task: task(),
        max_output_bytes: 1_024,
        timeout_ms: 2_000,
    }
}

#[test]
fn resuelve_ruta_version_hash_argv_entorno_y_procedencia_desde_manifest() {
    let root = root("success");
    std::fs::create_dir_all(root.join("manifests")).unwrap();
    std::fs::write(
        root.join("manifests/eco.toml"),
        manifest(std::path::Path::new("/bin/echo"), "coreutils"),
    )
    .unwrap();
    let executor =
        ManifestHarnessExecutor::open(&root.join("manifests"), profile(&root), root.join("runs"))
            .unwrap();

    let result = executor
        .invoke(&request("eco/eco/eco-modelo/coreutils", "hello"))
        .unwrap();
    assert_eq!(result.output, "hello\n");
    assert_eq!(result.failure, None);
    assert!(
        result
            .manifest_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("sha256:"))
    );

    assert!(
        executor
            .invoke(&request("eco/eco/eco-modelo/otra-version", "hello"))
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn solo_clasifica_un_fallo_observado_y_lo_desconocido_es_permanente() {
    let root = root("failures");
    std::fs::create_dir_all(root.join("manifests")).unwrap();
    let script = root.join("fake-harness");
    std::fs::write(
        &script,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo fake-1; exit 0; fi\nprintf '%s\\n' 'BATUTA_RESULT_V1:{\"failure\":{\"code\":\"rate_limited\",\"retry_after_ms\":25},\"input_tokens\":3,\"output_tokens\":1,\"provenance\":\"fake/eco/r1\"}' >&2\nexit 75\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&script, permissions).unwrap();
    std::fs::write(root.join("manifests/eco.toml"), manifest(&script, "fake-1")).unwrap();
    let executor =
        ManifestHarnessExecutor::open(&root.join("manifests"), profile(&root), root.join("runs"))
            .unwrap();
    let observed = executor
        .invoke(&request("eco/eco/eco-modelo/fake-1", "work"))
        .unwrap();
    assert_eq!(
        observed.failure,
        Some(InvocationFailure::RateLimited {
            retry_after_ms: Some(25)
        })
    );
    assert_eq!(observed.usage.input_tokens, 3);
    assert_eq!(observed.provenance.as_deref(), Some("fake/eco/r1"));

    std::fs::write(
        &script,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo fake-1; exit 0; fi\necho unclassified >&2\nexit 9\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&script, permissions).unwrap();
    std::fs::write(root.join("manifests/eco.toml"), manifest(&script, "fake-1")).unwrap();
    let unknown = ManifestHarnessExecutor::open(
        &root.join("manifests"),
        profile(&root),
        root.join("runs-unknown"),
    )
    .unwrap()
    .invoke(&request("eco/eco/eco-modelo/fake-1", "work"))
    .unwrap();
    assert_eq!(unknown.failure, Some(InvocationFailure::Permanent));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn perfil_o_manifest_invalido_impiden_abrir_el_ejecutor() {
    let root = root("tamper");
    std::fs::create_dir_all(root.join("manifests")).unwrap();
    let text = manifest(std::path::Path::new("/bin/echo"), "version-imposible").replace(
        &executable_hash(std::path::Path::new("/bin/echo")),
        &"0".repeat(64),
    );
    std::fs::write(root.join("manifests/eco.toml"), text).unwrap();
    assert!(
        ManifestHarnessExecutor::open(&root.join("manifests"), profile(&root), root.join("runs"),)
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
