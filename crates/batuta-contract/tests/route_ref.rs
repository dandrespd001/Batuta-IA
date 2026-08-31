//! Referencias de ruta inequívocas.

use std::str::FromStr;

use batuta_contract::RouteRef;

#[test]
fn minimax_es_una_ruta_de_dsh_y_hace_ida_y_vuelta() {
    let route = RouteRef::from_str("dsh/minimax/minimax-m2.5").unwrap();

    assert_eq!(route.harness(), "dsh");
    assert_eq!(route.provider(), "minimax");
    assert_eq!(route.model(), "minimax-m2.5");
    assert_eq!(route.to_string(), "dsh/minimax/minimax-m2.5");

    let json = serde_json::to_string(&route).unwrap();
    assert_eq!(json, "\"dsh/minimax/minimax-m2.5\"");
    assert_eq!(serde_json::from_str::<RouteRef>(&json).unwrap(), route);
}

#[test]
fn una_ruta_exige_harness_proveedor_modelo_y_revision_opcional() {
    for invalid in [
        "dsh/minimax",
        "/minimax/model",
        "dsh//model",
        "dsh/../model",
    ] {
        let error = RouteRef::from_str(invalid).unwrap_err().to_string();
        assert!(error.contains(invalid), "{error}");
        assert!(error.contains("harness/provider/model"), "{error}");
    }
}

#[test]
fn una_revision_opcional_forma_parte_de_la_identidad() {
    let route: RouteRef = "dsh/deepseek/deepseek-v4/2026-08-15"
        .parse()
        .expect("la revisión es el cuarto segmento contractual");

    assert_eq!(route.harness(), "dsh");
    assert_eq!(route.provider(), "deepseek");
    assert_eq!(route.model(), "deepseek-v4");
    assert_eq!(route.revision(), Some("2026-08-15"));
    assert_eq!(route.to_string(), "dsh/deepseek/deepseek-v4/2026-08-15");
    assert!(
        "dsh/deepseek/deepseek-v4/rev/extra"
            .parse::<RouteRef>()
            .is_err()
    );
}
