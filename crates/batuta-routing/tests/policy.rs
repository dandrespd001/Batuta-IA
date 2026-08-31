//! Contrato de alias y migración explícita de política.

use std::collections::BTreeMap;
use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_routing::{AliasCatalog, MigrationSettings, RoutingPolicy};

#[test]
fn cada_alias_resuelve_una_ruta_exacta() {
    let route = RouteRef::from_str("dsh/deepseek/deepseek-v4-flash").unwrap();
    let aliases = AliasCatalog::new(BTreeMap::from([(
        "deepseekV4-Flash".to_string(),
        route.clone(),
    )]))
    .unwrap();

    assert_eq!(aliases.resolve("deepseekV4-Flash").unwrap(), route);
    assert_eq!(
        aliases.resolve("dsh/deepseek/deepseek-v4-flash").unwrap(),
        route
    );
}

#[test]
fn la_politica_v1_no_se_carga_como_v2_sin_migracion() {
    let legacy = r#"
schema_version = 1

[modelos.dsh-deepseek-v4-flash]
habilitado = true
esfuerzo = "high"
"#;

    assert!(
        RoutingPolicy::from_toml(legacy)
            .unwrap_err()
            .to_string()
            .contains("migrate")
    );

    let migrated = RoutingPolicy::migrate_v1(
        legacy,
        MigrationSettings {
            minimum_quality: 78.0,
            selection_margin: 4.0,
            minimum_coverage: 70,
            max_evidence_age_seconds: 2_592_000,
            allow_any_eligible: false,
            allow_unverified_quality: false,
            max_attempts: 3,
            max_retry_after_ms: 30_000,
            max_handoffs: 2,
        },
    )
    .unwrap();

    assert_eq!(migrated.schema_version(), 2);
    assert!((migrated.settings().minimum_quality - 78.0).abs() < f64::EPSILON);
    assert!(!migrated.settings().allow_unverified_quality);
    assert!(migrated.legacy_models()["dsh-deepseek-v4-flash"].enabled);
    assert_eq!(
        RoutingPolicy::from_toml(&migrated.to_toml().unwrap()).unwrap(),
        migrated
    );
}
