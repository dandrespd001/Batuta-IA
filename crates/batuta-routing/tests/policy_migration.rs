//! Migrar v1 ofrece dry-run, confirmación, backup y segunda ejecución idempotente.

use batuta_routing::{MigrationSettings, PolicyMigration, PolicyMigrationOutcome};

#[test]
fn dry_run_no_escribe_y_apply_conserva_v1_recuperable() {
    let root = std::env::temp_dir().join(format!("batuta-policy-migration-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("politica.toml");
    let legacy = "schema_version = 1\n[modelos.\"dsh/model\"]\nhabilitado = true\n";
    std::fs::write(&path, legacy).unwrap();
    let migration = PolicyMigration::plan(
        path.clone(),
        MigrationSettings {
            minimum_quality: 70.0,
            selection_margin: 5.0,
            minimum_coverage: 80,
            max_evidence_age_seconds: 86_400,
            allow_any_eligible: false,
            allow_unverified_quality: false,
            max_attempts: 3,
            max_retry_after_ms: 30_000,
            max_handoffs: 2,
        },
    )
    .unwrap();

    assert!(migration.diff().contains("schema_version: 1 -> 2"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), legacy);
    assert!(migration.apply(false).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), legacy);
    assert_eq!(
        migration.apply(true).unwrap(),
        PolicyMigrationOutcome::Applied
    );
    assert_eq!(
        std::fs::read_to_string(path.with_extension("toml.v1.bak")).unwrap(),
        legacy
    );
    assert_eq!(
        migration.apply(true).unwrap(),
        PolicyMigrationOutcome::AlreadyApplied
    );

    std::fs::remove_dir_all(root).unwrap();
}
