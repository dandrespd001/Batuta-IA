//! T2 (`docs/FASE5_PANEL.md`) — R3 comprobado en el propio `Cargo.toml`: la
//! política no depende de quien mide.

use std::path::Path;

fn cargo_toml() -> String {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    std::fs::read_to_string(&ruta).unwrap_or_else(|_| panic!("{} se lee", ruta.display()))
}

/// R3: la medición nunca consulta la política que informa. Si
/// `batuta-policy` dependiera de `batuta-exec`, un ciclo esperaría a
/// formarse el día que `batuta-exec` necesite leer la política (y lo va a
/// necesitar, en T5).
#[test]
fn la_politica_no_depende_de_quien_mide() {
    let manifiesto: toml::Value = toml::from_str(&cargo_toml()).expect("Cargo.toml es TOML válido");
    let dependencias = manifiesto
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("hay una tabla [dependencies]");

    assert!(
        !dependencias.contains_key("batuta-exec"),
        "batuta-policy no puede depender de batuta-exec (R3)"
    );
}

/// El crate se queda deliberadamente pequeño: sólo lo que necesita para
/// nombrar un modelo y guardar una elección.
#[test]
fn la_politica_solo_depende_del_contrato() {
    let manifiesto: toml::Value = toml::from_str(&cargo_toml()).expect("Cargo.toml es TOML válido");
    let dependencias = manifiesto
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("hay una tabla [dependencies]");

    let nombres: Vec<&str> = dependencias.keys().map(String::as_str).collect();
    for nombre in &nombres {
        assert!(
            *nombre == "batuta-contract" || !nombre.starts_with("batuta-"),
            "dependencia inesperada de otro crate de batuta: {nombre}. {nombres:?}"
        );
    }
}
