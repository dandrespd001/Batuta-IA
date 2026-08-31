//! El estado de ejecución se recupera sin reconstruir el historial conversacional.

use std::str::FromStr as _;

use batuta_contract::RouteRef;
use batuta_routing::{RunState, RunStateStore};

#[test]
fn reiniciar_recupera_la_unica_ruta_activa() {
    let root = std::env::temp_dir().join(format!("batuta-run-store-{}", std::process::id()));
    let store = RunStateStore::open(root.join("run-1.json"));
    let state = RunState::Running {
        route: RouteRef::from_str("dsh/deepseek/v4/r1").unwrap(),
        handoff: None,
    };

    store.save("run-1", &state).unwrap();
    let loaded = RunStateStore::open(root.join("run-1.json"))
        .load("run-1")
        .unwrap();
    assert_eq!(loaded, state);

    std::fs::remove_dir_all(root).unwrap();
}
