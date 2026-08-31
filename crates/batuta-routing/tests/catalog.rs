//! El catálogo conserva la identidad DSH y filtra `OpenCode` por coste declarado.

use batuta_routing::{CatalogClass, DshCatalogBridge};

#[test]
fn opencode_solo_entra_con_todos_los_componentes_de_coste_en_cero() {
    let document = serde_json::json!({
        "routes": [
            {"provider":"opencode","model":"real-free","cost":{"input":0.0,"output":0.0,"cache_read":0.0,"cache_write":0.0},"api_key":"must-not-leak"},
            {"provider":"opencode","model":"named-free","cost":{"input":null,"output":0.0,"cache_read":0.0,"cache_write":0.0}},
            {"provider":"opencode","model":"paid","cost":{"input":0.0,"output":0.1,"cache_read":0.0,"cache_write":0.0}},
            {"provider":"minimax","model":"minimax-m2.5","revision":"r7","cost":{"input":0.2,"output":0.4,"cache_read":null,"cache_write":null}},
            {"provider":"deepseek","model":"deepseek-v4-flash","cost":{"input":0.1,"output":0.2,"cache_read":0.0,"cache_write":0.0}}
        ],
        "balance": 9000,
        "subscription": "secret-plan"
    });

    let report = DshCatalogBridge::import_json(&document.to_string()).unwrap();
    let routes = report.catalog.routes();

    assert_eq!(routes.len(), 3);
    assert_eq!(
        routes[0].route.to_string(),
        "dsh/deepseek/deepseek-v4-flash"
    );
    assert_eq!(routes[1].route.to_string(), "dsh/minimax/minimax-m2.5/r7");
    assert_eq!(routes[2].route.to_string(), "dsh/opencode/real-free");
    assert!(
        routes
            .iter()
            .all(|route| route.class == CatalogClass::ProbeTest)
    );
    assert_eq!(report.rejected.len(), 2);

    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("must-not-leak"));
    assert!(!serialized.contains("balance"));
    assert!(!serialized.contains("subscription"));
}

#[test]
fn un_nombre_free_sin_descriptor_de_coste_no_se_importa() {
    let document = r#"{"routes":[{"provider":"opencode","model":"definitely-free"}]}"#;
    let report = DshCatalogBridge::import_json(document).unwrap();

    assert!(report.catalog.routes().is_empty());
    assert_eq!(report.rejected[0].code, "opencode_cost_not_proven_zero");
}
