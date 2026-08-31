//! La salud y los cooldown sobreviven al proceso que los observó.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_routing::{FailureCategory, HealthStore, RouteHealth};

#[test]
fn salud_durable_conserva_cooldown_y_bloqueo_de_harness() {
    let root = std::env::temp_dir()
        .join("batuta-health-store")
        .join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&root);
    let store = HealthStore::open(root.join("health.json"));
    let route = RouteRef::from_str("abacus/abacus/glm-5.3").unwrap();
    let mut health = RouteHealth::healthy();
    health.cooldown_until = Some(2_000);
    health.blocked_by_harness = true;
    health.last_failure = Some(FailureCategory::Authentication);

    store.update(route.clone(), health.clone()).unwrap();
    let loaded = store.load().unwrap();

    assert_eq!(loaded.get(&route), Some(&health));
    assert!(root.read_dir().unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")
    }));
}
