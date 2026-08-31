//! Contratos del almacén transaccional de estado v2.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::str::FromStr as _;
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::{ReasoningEffort, RouteRef, Sensitivity};
use batuta_routing::{
    CapabilityIndexV2, CatalogRouteStateV2, CatalogStateV2, EvidenceStateV2, ExecutionPolicyV2,
    HealthStateV2, PolicyStateV2, RouteClass, StateComponentsV2, StateStore,
};

fn root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-state-v2-{nonce}"))
}

fn components(marker: &str) -> StateComponentsV2 {
    let route = marker_route(marker);
    StateComponentsV2 {
        catalog: CatalogStateV2 {
            schema_version: 2,
            routes: BTreeMap::from([(
                route,
                CatalogRouteStateV2 {
                    class: RouteClass::ProbeTest,
                    max_sensitivity: Sensitivity::Public,
                    context_window: 1,
                    supported_efforts: BTreeSet::from([ReasoningEffort::Low]),
                },
            )]),
        },
        policy: PolicyStateV2 {
            schema_version: 2,
            execution: ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
            profiles: BTreeMap::new(),
            routes: BTreeMap::new(),
        },
        evidence: EvidenceStateV2 {
            schema_version: 2,
            projections: vec![],
        },
        health: HealthStateV2 {
            schema_version: 2,
            routes: BTreeMap::new(),
        },
        capabilities: CapabilityIndexV2 {
            schema_version: 2,
            routes: BTreeMap::new(),
        },
    }
}

fn marker_route(marker: &str) -> RouteRef {
    RouteRef::from_str(&format!("dsh/test/{marker}")).unwrap()
}

#[test]
fn commit_publica_un_manifest_tras_objetos_inmutables() {
    let root = root();
    let store = StateStore::open(root.clone());

    let first = store.commit(&components("first")).unwrap();
    let loaded = store.load().unwrap();

    assert_eq!(first.schema_version, 2);
    assert_eq!(first.generation, 1);
    assert_eq!(loaded.manifest, first);
    assert!(
        loaded
            .components
            .catalog
            .routes
            .contains_key(&marker_route("first"))
    );
    for hash in first.component_hashes() {
        let path = store.object_path(hash).unwrap();
        assert!(path.is_file());
        assert_eq!(fs::read(&path).unwrap(), fs::read(&path).unwrap());
    }

    let second = store.commit(&components("second")).unwrap();
    assert_eq!(second.generation, 2);
    assert_ne!(second.catalog_hash, first.catalog_hash);
    assert!(store.object_path(&first.catalog_hash).unwrap().is_file());
    assert!(
        store
            .load()
            .unwrap()
            .components
            .catalog
            .routes
            .contains_key(&marker_route("second"))
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn un_fallo_escribiendo_objetos_conserva_el_manifest_anterior() {
    let root = root();
    let store = StateStore::open(root.clone());
    let first = store.commit(&components("active")).unwrap();

    let objects = root.join("objects");
    let mut permissions = fs::metadata(&objects).unwrap().permissions();
    let original_mode = permissions.mode();
    permissions.set_mode(original_mode & !0o222);
    fs::set_permissions(&objects, permissions).unwrap();
    let failed = store.commit(&components("cannot-publish"));
    let mut permissions = fs::metadata(&objects).unwrap().permissions();
    permissions.set_mode(original_mode);
    fs::set_permissions(&objects, permissions).unwrap();

    assert!(failed.is_err());
    assert_eq!(store.load_manifest().unwrap(), first);
    assert!(
        store
            .load()
            .unwrap()
            .components
            .catalog
            .routes
            .contains_key(&marker_route("active"))
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn alterar_un_objeto_se_detecta_antes_de_ensamblar() {
    let root = root();
    let store = StateStore::open(root.clone());
    let manifest = store.commit(&components("sealed")).unwrap();
    let object = store.object_path(&manifest.catalog_hash).unwrap();
    fs::write(object, b"{}\n").unwrap();

    let error = store.load().unwrap_err();
    assert!(error.to_string().contains("hash mismatch"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dos_escritores_con_la_misma_base_publican_como_maximo_un_commit() {
    let root = root();
    let store = StateStore::open(root.clone());
    let base = store.commit(&components("base")).unwrap();
    let base_hash = base.manifest_hash().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

    let handles = ["writer-a", "writer-b"].map(|marker| {
        let root = root.clone();
        let base_hash = base_hash.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            StateStore::open(root).commit_if_base(&components(marker), Some(&base_hash))
        })
    });
    barrier.wait();
    let successes = handles
        .into_iter()
        .filter_map(|handle| handle.join().unwrap().ok())
        .count();

    assert_eq!(successes, 1);
    assert_eq!(store.load_manifest().unwrap().generation, 2);
    std::fs::remove_dir_all(root).unwrap();
}
