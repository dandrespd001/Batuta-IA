//! T5 (`docs/FASE5_PANEL.md`) — `enable`, `disable`, `effort` desde fuera.

use std::fs;
use std::path::{Path, PathBuf};

use batuta_cli::{Layout, canary, disable, effort, enable};
use batuta_policy::Politica;

fn proveedores() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn disposicion(nombre: &str) -> Layout {
    let raiz = std::env::temp_dir().join(format!("batuta-cli-eleccion-{nombre}"));
    let _ = fs::remove_dir_all(&raiz);
    Layout::under(raiz)
}

fn politica(disposicion: &Layout) -> Politica {
    Politica::cargar(&disposicion.politica()).expect("la política escrita se relee")
}

/// `enable` deja el modelo activo y en disco.
#[test]
fn enable_activa_y_guarda() {
    let disposicion = disposicion("enable");
    enable(&proveedores(), &disposicion, "eco/eco-modelo").expect("se activa");

    let politica = politica(&disposicion);
    assert!(politica.esta_habilitado(&"eco-modelo".parse().unwrap()));
}

/// `disable` lo apaga sin borrar el esfuerzo que ya tuviera.
#[test]
fn disable_apaga_y_conserva_el_esfuerzo() {
    let disposicion = disposicion("disable");
    let referencia = "con-esfuerzo/con-esfuerzo-modelo";
    effort(&proveedores(), &disposicion, referencia, "high").expect("declara el mapa");
    enable(&proveedores(), &disposicion, referencia).expect("se activa");
    disable(&proveedores(), &disposicion, referencia).expect("se apaga");

    let politica = politica(&disposicion);
    let id = "con-esfuerzo-modelo".parse().unwrap();
    assert!(!politica.esta_habilitado(&id));
    assert_eq!(
        politica.esfuerzo(&id),
        Some(batuta_contract::ReasoningEffort::High),
        "el esfuerzo no se pierde al apagar"
    );
}

/// El criterio explícito de T5: `disable` no toca ni el manifiesto ni los
/// recibos. Sólo cambia la política.
#[test]
fn disable_no_toca_el_manifiesto_ni_los_recibos() {
    let disposicion = disposicion("no-toca-nada");
    let manifiesto_ruta = proveedores().join("eco.toml");
    let manifiesto_antes = fs::read(&manifiesto_ruta).expect("se lee el manifiesto");

    let salida = canary(
        "eco",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect("el canario del eco corre");
    let recibo_antes = fs::read(&salida.receipt_path).expect("se lee el recibo");

    enable(&proveedores(), &disposicion, "eco/eco-modelo").expect("se activa");
    disable(&proveedores(), &disposicion, "eco/eco-modelo").expect("se apaga");

    let manifiesto_despues = fs::read(&manifiesto_ruta).expect("se relee el manifiesto");
    assert_eq!(
        manifiesto_antes, manifiesto_despues,
        "disable no puede tocar el manifiesto"
    );

    let recibo_despues = fs::read(&salida.receipt_path).expect("se relee el recibo");
    assert_eq!(
        recibo_antes, recibo_despues,
        "disable no puede tocar el recibo"
    );

    // Y no aparece ningún recibo nuevo: disable no canaria nada.
    let recibos: Vec<_> = fs::read_dir(disposicion.receipts())
        .expect("el directorio de recibos existe")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(recibos.len(), 1, "disable no debió generar ningún recibo");
}

/// `effort` con un nivel que `ReasoningEffort` no admite lo dice, listando
/// los válidos (R8, heredado del vocabulario).
#[test]
fn effort_con_un_nivel_invalido_enumera_los_validos() {
    let disposicion = disposicion("nivel-malo");
    let error = effort(
        &proveedores(),
        &disposicion,
        "eco/eco-modelo",
        "urgentísimo",
    )
    .expect_err("ese nivel no existe");
    let mensaje = error.to_string();

    assert!(mensaje.contains("urgentísimo"), "{mensaje}");
    assert!(mensaje.contains("high"), "{mensaje}");
    assert!(mensaje.contains("max"), "{mensaje}");
}

/// El criterio explícito de T5: pedir un esfuerzo a un proveedor que no
/// declara mapa es un error que lo dice, no un valor que se guarda y se
/// ignora luego.
#[test]
fn effort_a_un_proveedor_sin_mapa_lo_dice_y_no_guarda_nada() {
    let disposicion = disposicion("sin-mapa");
    let error = effort(&proveedores(), &disposicion, "eco/eco-modelo", "high")
        .expect_err("eco no declara ningún mapa de esfuerzo");

    assert!(error.to_string().contains("eco"), "{error}");
    assert!(
        !disposicion.politica().exists(),
        "no se debió escribir ninguna política"
    );
}

/// `<proveedor>/<modelo>` inexistente enumera lo que sí hay (R8) — el
/// proveedor primero, el modelo dentro de él después.
#[test]
fn un_proveedor_inexistente_enumera_los_que_hay() {
    let disposicion = disposicion("proveedor-malo");
    let error = enable(&proveedores(), &disposicion, "zapatilla/algo")
        .expect_err("zapatilla no es un proveedor");
    let mensaje = error.to_string();

    assert!(mensaje.contains("zapatilla"), "{mensaje}");
    assert!(mensaje.contains("eco"), "{mensaje}");
}

#[test]
fn un_modelo_inexistente_en_un_proveedor_real_enumera_los_suyos() {
    let disposicion = disposicion("modelo-malo");
    let error =
        enable(&proveedores(), &disposicion, "eco/no-existe").expect_err("ese modelo no es de eco");
    let mensaje = error.to_string();

    assert!(mensaje.contains("no-existe"), "{mensaje}");
    assert!(mensaje.contains("eco-modelo"), "{mensaje}");
}

/// Una referencia sin barra lo dice, distinto de un proveedor desconocido.
#[test]
fn una_referencia_sin_barra_lo_dice() {
    let disposicion = disposicion("sin-barra");
    let error = enable(&proveedores(), &disposicion, "eco-modelo")
        .expect_err("sin barra no se puede partir");
    assert!(error.to_string().contains("proveedor"), "{error}");
}
