//! Cliente Rust del sidecar offline y con salida cerrada.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use batuta_routing::DshSidecarClient;

#[test]
fn usa_catalogo_sin_stream_y_opencode_desconocido_no_llega_al_selector() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let module = format!(
        "file://{}",
        root.join("sidecar/fake_dsh_catalog.mjs").display()
    );
    let client = DshSidecarClient::new(
        PathBuf::from("node"),
        vec![root.join("sidecar/dsh_catalog.mjs").display().to_string()],
        BTreeMap::from([
            ("PATH".to_string(), std::env::var("PATH").unwrap()),
            ("BATUTA_DSH_CATALOG_MODULE".to_string(), module),
        ]),
        Duration::from_secs(5),
        64 * 1024,
        16 * 1024,
    )
    .unwrap();
    let report = client.catalog_snapshot("catalog-test").unwrap();

    assert_eq!(report.catalog.routes().len(), 1);
    assert_eq!(report.rejected.len(), 1);
    assert_eq!(report.rejected[0].code, "opencode_cost_not_proven_zero");
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("apiKey"));
    assert!(!json.contains("balance"));
}
