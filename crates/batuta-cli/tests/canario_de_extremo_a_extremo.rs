//! El canario desde fuera: manifiestos en un directorio, recibo en otro.
//!
//! Aquí es donde se ve si las cinco piezas encajan de verdad. Todo con el
//! fixture del eco, **sin una sola llamada de red**: cuando el canario contra un
//! proveedor real falle, éste dirá si el fallo es del proveedor o de batuta.

use std::fs;
use std::path::{Path, PathBuf};

use batuta_cli::{Layout, canary, canary_all};

fn proveedores() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn disposicion(nombre: &str) -> Layout {
    let raiz = std::env::temp_dir().join(format!("batuta-cli-{nombre}"));
    let _ = fs::remove_dir_all(&raiz);
    Layout::under(raiz)
}

/// R8: el error nombra lo pedido **y enumera lo que sí hay**. Un
/// `"provider not found"` obliga a ir a mirar el directorio a mano.
#[test]
fn un_proveedor_que_no_existe_nombra_los_que_si() {
    let disposicion = disposicion("inexistente");
    let error = canary(
        "zapatilla",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect_err("`zapatilla` no es un proveedor");
    let mensaje = error.to_string();

    assert!(mensaje.contains("zapatilla"), "{mensaje}");
    assert!(mensaje.contains("eco"), "no enumera los que hay: {mensaje}");
    assert!(mensaje.contains("dos-modelos"), "{mensaje}");
}

/// Elegir en silencio es como se pidió un modelo tres veces y corrió otro las
/// tres. Con más de uno y sin `--model`, se para y se enumeran.
#[test]
fn varios_modelos_y_ninguno_pedido_los_enumera() {
    let disposicion = disposicion("ambiguo");
    let error = canary(
        "dos-modelos",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect_err("dos modelos y ninguno pedido");
    let mensaje = error.to_string();

    assert!(mensaje.contains("--model"), "{mensaje}");
    assert!(mensaje.contains("eco-rapido"), "{mensaje}");
    assert!(mensaje.contains("eco-lento"), "{mensaje}");
}

#[test]
fn un_modelo_que_el_proveedor_no_declara_enumera_los_suyos() {
    let disposicion = disposicion("modelo-ajeno");
    let error = canary(
        "dos-modelos",
        Some("eco-inventado"),
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect_err("ese modelo no es suyo");
    let mensaje = error.to_string();

    assert!(mensaje.contains("eco-inventado"), "{mensaje}");
    assert!(mensaje.contains("eco-rapido"), "{mensaje}");
}

/// El recorrido entero: manifiesto, admisión, sustitución, materialización,
/// proceso, procedencia y recibo **en disco**.
#[test]
fn el_canario_del_eco_deja_su_recibo_en_disco() {
    let disposicion = disposicion("eco-verde");
    let salida = canary(
        "eco",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect("el canario del eco tiene que poder ejecutarse");

    assert!(
        salida.receipt.verdict().is_green(),
        "{:?}",
        salida.receipt.verdict()
    );
    assert!(
        salida.receipt_path.is_file(),
        "el recibo no está en disco: {:?}",
        salida.receipt_path
    );
    assert!(
        salida.receipt_path.starts_with(disposicion.receipts()),
        "el recibo se guarda donde se dijo: {:?}",
        salida.receipt_path
    );

    // Los leases se sueltan al acabar: la otra mitad de R6.
    for espacio in ["model", "repository"] {
        let dir = disposicion.leases().join(espacio);
        let vivos: Vec<_> = fs::read_dir(&dir)
            .map(|e| e.filter_map(Result::ok).collect())
            .unwrap_or_default();
        assert!(vivos.is_empty(), "quedó un lease en {espacio}: {vivos:?}");
    }
}

/// Un recibo que no lleva el `argv` real, el código y el stderr íntegro no sirve
/// para diagnosticar nada: `"Harness worker failed with exit 1"` es exactamente
/// el mensaje que costó días.
///
/// Y **ningún valor de entorno**. Los nombres sí, para poder auditar qué se pasó;
/// los valores nunca, porque un recibo se archiva y se comparte.
#[test]
fn el_recibo_escrito_lleva_los_hechos_y_ningun_valor_de_entorno() {
    let disposicion = disposicion("eco-hechos");
    let salida = canary(
        "eco",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect("canario");

    let json = fs::read_to_string(&salida.receipt_path).expect("el recibo se lee");
    let valor: serde_json::Value = serde_json::from_str(&json).expect("el recibo es JSON");

    assert!(valor["argv"].is_array(), "sin argv: {json}");
    assert!(!valor["argv"].as_array().expect("argv").is_empty());
    assert_eq!(valor["exit_code"], serde_json::json!(0));
    assert!(valor.get("stderr").is_some(), "sin stderr: {json}");
    assert_eq!(valor["env_names"], serde_json::json!(["HOME", "PATH"]));
    assert_eq!(
        valor["demonstrated_capabilities"],
        serde_json::json!([]),
        "el canario básico no demuestra capacidades de tarea: {json}"
    );

    // La sonda es `PATH` y no `HOME` a propósito. El recibo lleva rutas de
    // fichero legítimas —el manifiesto, el worktree— y muchas caen bajo `$HOME`,
    // así que buscar el valor de `HOME` daría un falso positivo. El de `PATH` es
    // una cadena larga y unívoca que no tiene ninguna razón para aparecer.
    let ruta = std::env::var("PATH").expect("PATH");
    assert!(
        !json.contains(&ruta),
        "el recibo lleva el VALOR de una variable de entorno"
    );
}

/// Un canario por modelo, y **ninguno detiene a los demás**.
///
/// Un modelo rojo no es un fallo del lote: es el resultado de ese modelo. Parar
/// en el primero dejaría sin medir a los que van detrás, y el lote existe
/// justamente para saber cuáles valen.
#[test]
fn todos_los_modelos_del_proveedor_reciben_su_canario() {
    let disposicion = disposicion("lote");
    let salidas = canary_all(
        "dos-modelos",
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect("el lote tiene que poder ejecutarse");

    assert_eq!(salidas.len(), 2, "un canario por modelo");
    for salida in &salidas {
        assert!(
            salida.receipt.verdict().is_green(),
            "{:?}",
            salida.receipt.verdict()
        );
        assert!(salida.receipt_path.is_file(), "recibo en disco");
    }

    // Cada modelo tiene el suyo, no dos veces el mismo.
    let modelos: Vec<&str> = salidas
        .iter()
        .map(|s| s.receipt.model_requested())
        .collect();
    assert!(modelos.contains(&"eco-rapido"), "{modelos:?}");
    assert!(modelos.contains(&"eco-lento"), "{modelos:?}");

    // Y los leases quedan sueltos: el lote los toma y los suelta uno a uno.
    for espacio in ["model", "repository"] {
        let vivos: Vec<_> = fs::read_dir(disposicion.leases().join(espacio))
            .map(|e| e.filter_map(Result::ok).collect())
            .unwrap_or_default();
        assert!(vivos.is_empty(), "quedó un lease en {espacio}: {vivos:?}");
    }
}
