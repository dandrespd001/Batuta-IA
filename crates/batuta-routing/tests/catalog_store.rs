//! Importar catálogo siempre pasa por staging y confirmación explícita.

use batuta_routing::{CatalogStore, DshCatalogBridge};

#[test]
fn import_status_y_apply_comparten_staging_sin_autoaplicar() {
    let root = std::env::temp_dir().join(format!("batuta-catalog-{}", std::process::id()));
    let store = CatalogStore::open(root.clone());
    let report = DshCatalogBridge::import_json(
        r#"{"routes":[{"provider":"deepseek","model":"v4","cost":{"input":1.0,"output":1.0,"cache_read":0.0,"cache_write":0.0}}]}"#,
    )
    .unwrap();

    let proposal = store.stage("proposal-1", 100, report.catalog).unwrap();
    let before = store.status().unwrap();
    assert_eq!(before.active_routes, 0);
    assert_eq!(before.staged, vec!["proposal-1"]);
    assert_eq!(
        store.apply("proposal-1", false).unwrap_err().code(),
        "proposal_not_confirmed"
    );
    assert_eq!(store.status().unwrap().active_routes, 0);

    let active = store.apply("proposal-1", true).unwrap();
    assert_eq!(active.routes().len(), 1);
    assert_eq!(
        store.status().unwrap().active_hash,
        proposal.proposed_catalog_hash
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn una_propuesta_basada_en_otro_activo_no_pisa_el_catalogo() {
    let root = std::env::temp_dir().join(format!("batuta-catalog-conflict-{}", std::process::id()));
    let store = CatalogStore::open(root.clone());
    let one = DshCatalogBridge::import_json(
        r#"{"routes":[{"provider":"deepseek","model":"one","cost":{"input":1.0,"output":1.0,"cache_read":0.0,"cache_write":0.0}}]}"#,
    )
    .unwrap();
    let two = DshCatalogBridge::import_json(
        r#"{"routes":[{"provider":"minimax","model":"two","cost":{"input":1.0,"output":1.0,"cache_read":null,"cache_write":null}}]}"#,
    )
    .unwrap();
    store.stage("old-base", 100, one.catalog).unwrap();
    store.stage("winner", 101, two.catalog).unwrap();
    store.apply("winner", true).unwrap();

    assert_eq!(
        store.apply("old-base", true).unwrap_err().code(),
        "catalog_base_conflict"
    );
    assert_eq!(
        store.load_active().unwrap().routes()[0].route.model(),
        "two"
    );

    std::fs::remove_dir_all(root).unwrap();
}
