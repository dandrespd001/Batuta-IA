//! T4 (`docs/FASE5_PANEL.md`) — el panel desde fuera: manifiestos reales,
//! política real, recibos reales.

use std::fs;
use std::path::{Path, PathBuf};

use batuta_cli::{Layout, canary, filas, tabla};

fn proveedores() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn disposicion(nombre: &str) -> Layout {
    let raiz = std::env::temp_dir().join(format!("batuta-cli-panel-{nombre}"));
    let _ = fs::remove_dir_all(&raiz);
    Layout::under(raiz)
}

fn fila<'a>(filas: &'a [batuta_cli::Fila], provider: &str, model: &str) -> &'a batuta_cli::Fila {
    filas
        .iter()
        .find(|f| f.provider == provider && f.model == model)
        .unwrap_or_else(|| panic!("no está la fila {provider}/{model}: {filas:?}"))
}

/// Sin política y sin canarios, cada modelo sale enseñado y en su estado
/// inicial: apagado, sin evidencia, no enrutable.
#[test]
fn sin_politica_ni_recibos_todo_sale_apagado_y_no_enrutable() {
    let disposicion = disposicion("vacio");
    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");

    let fila = fila(&lista, "eco", "eco-modelo");
    assert!(!fila.enabled, "{fila:?}");
    assert!(!fila.routable, "{fila:?}");
    assert_eq!(fila.canary, "ninguno");
    assert_eq!(fila.confirmed, None);
}

/// El criterio explícito de T4: un modelo **activo** sin ningún recibo verde
/// sale enseñado, y marcado como no enrutable. «Activo» y «enrutable» no son
/// lo mismo, y el panel no puede disimularlo.
#[test]
fn un_modelo_activo_sin_recibo_no_es_enrutable() {
    let disposicion = disposicion("activo-sin-recibo");
    fs::create_dir_all(disposicion.root()).expect("raíz del estado");
    fs::write(
        disposicion.politica(),
        "schema_version = 1\n\n[modelos.\"eco-modelo\"]\nhabilitado = true\n",
    )
    .expect("política de prueba");

    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");
    let fila = fila(&lista, "eco", "eco-modelo");

    assert!(fila.enabled, "tiene que salir activo: {fila:?}");
    assert!(
        !fila.routable,
        "activo sin recibo no es enrutable: {fila:?}"
    );
    assert_eq!(fila.canary, "ninguno");
}

/// Con un canario verde real y la política activa, el modelo sí es enrutable.
#[test]
fn un_modelo_activo_con_canario_verde_es_enrutable() {
    let disposicion = disposicion("activo-con-recibo");
    canary(
        "eco",
        None,
        &proveedores(),
        &disposicion,
        Path::new("/inexistente"),
    )
    .expect("el canario del eco corre");
    fs::write(
        disposicion.politica(),
        "schema_version = 1\n\n[modelos.\"eco-modelo\"]\nhabilitado = true\nesfuerzo = \"high\"\n",
    )
    .expect("política de prueba");

    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");
    let fila = fila(&lista, "eco", "eco-modelo");

    assert!(fila.enabled, "{fila:?}");
    assert!(fila.routable, "activo y con recibo verde: {fila:?}");
    assert!(
        fila.canary.starts_with("verde"),
        "columna de canario: {}",
        fila.canary
    );
    assert_eq!(fila.effort.as_deref(), Some("high"));
}

/// `--provider` filtra: sólo aparecen las filas de ese proveedor.
#[test]
fn provider_filtra_las_filas() {
    let disposicion = disposicion("filtro");
    let lista = filas(&proveedores(), &disposicion, Some("dos-modelos")).expect("se listan");

    assert!(
        lista.iter().all(|f| f.provider == "dos-modelos"),
        "{lista:?}"
    );
    assert_eq!(lista.len(), 2, "dos-modelos declara dos modelos: {lista:?}");
}

/// Un proveedor que no existe en el filtro lo dice, y enumera los que sí hay
/// (R8) — igual que `canary --provider`.
#[test]
fn un_provider_inexistente_en_el_filtro_enumera_los_que_hay() {
    let disposicion = disposicion("filtro-malo");
    let error = filas(&proveedores(), &disposicion, Some("zapatilla")).expect_err("no existe");
    let mensaje = error.to_string();

    assert!(mensaje.contains("zapatilla"), "{mensaje}");
    assert!(mensaje.contains("eco"), "{mensaje}");
}

/// Un modelo cuyo `observed_as` difiere de `route_model` sale marcado con
/// `⚠`, con el alias declarado — es un hecho del manifiesto, no hace falta
/// ningún canario para saberlo.
#[test]
fn un_alias_declarado_distinto_sale_marcado() {
    let disposicion = disposicion("alias");
    let lista = filas(&proveedores(), &disposicion, Some("alias")).expect("se listan");
    let fila = fila(&lista, "alias", "alias-modelo");

    assert_eq!(fila.warning.as_deref(), Some("ECO_CON_ALIAS"), "{fila:?}");
}

/// Un modelo sin `observed_as` no lleva marca.
#[test]
fn sin_alias_declarado_no_hay_marca() {
    let disposicion = disposicion("sin-alias");
    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");
    let fila = fila(&lista, "eco", "eco-modelo");

    assert_eq!(fila.warning, None, "{fila:?}");
}

/// La tabla renderizada lleva la cabecera y cada fila, con la marca `⚠`
/// pegada al final de la línea que corresponde.
#[test]
fn la_tabla_lleva_cabecera_y_la_marca_en_su_fila() {
    let disposicion = disposicion("tabla");
    let lista = filas(&proveedores(), &disposicion, Some("alias")).expect("se listan");
    let texto = tabla(&lista);

    assert!(texto.contains("PROVEEDOR"), "{texto}");
    assert!(texto.contains("CANARIO"), "{texto}");
    assert!(texto.contains("alias-modelo"), "{texto}");
    assert!(texto.contains("⚠ ECO_CON_ALIAS"), "{texto}");
}
