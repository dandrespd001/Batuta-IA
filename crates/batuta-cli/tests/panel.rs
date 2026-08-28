//! T4 (`docs/FASE5_PANEL.md`) — el panel desde fuera: manifiestos reales,
//! política real, recibos reales.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use batuta_cli::{Fila, Layout, canary, escribir_html, filas, tabla, tabla_html};

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

/// Parte el `<tbody>` de una tabla HTML en el contenido de cada `<tr>`, en
/// orden. T7: la prueba de consistencia necesita comparar fila a fila, no
/// sólo "el documento entero contiene esta subcadena en algún sitio" —eso
/// dejaría pasar una fila que perdió su canario mientras otra lo repite.
fn filas_del_cuerpo_html(html: &str) -> Vec<&str> {
    let cuerpo = html
        .split("<tbody>")
        .nth(1)
        .unwrap_or_else(|| panic!("la tabla no tiene <tbody>: {html}"))
        .split("</tbody>")
        .next()
        .unwrap_or_else(|| panic!("el <tbody> no cierra: {html}"));

    cuerpo
        .split("<tr>")
        .skip(1) // lo que hay antes del primer <tr> no es una fila
        .map(|trozo| {
            trozo
                .split("</tr>")
                .next()
                .unwrap_or_else(|| panic!("una fila no cierra </tr>: {html}"))
        })
        .collect()
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

/// T7 — la garantía central del checklist: `tabla` y `tabla_html` sobre el
/// MISMO `&[Fila]` (una sola llamada a `filas()`) dicen la misma verdad. Se
/// compara fila a fila —no sólo "el documento la contiene en algún sitio"—
/// para que una fila que pierde su canario, o que se lo cuelga a la fila de
/// al lado, no se cuele.
#[test]
fn tabla_html_dice_lo_mismo_que_tabla_para_las_mismas_filas() {
    let disposicion = disposicion("html-consistencia");
    // Sin filtro: barre los cuatro manifiestos de prueba, así se ejercitan a
    // la vez filas con `warning` y filas sin él.
    let lista = filas(&proveedores(), &disposicion, None).expect("se listan");
    assert!(
        lista.len() >= 4,
        "pocas filas para una prueba útil: {lista:?}"
    );

    let texto = tabla(&lista);
    let html = tabla_html(&lista);

    let cuerpo = filas_del_cuerpo_html(&html);
    assert_eq!(
        cuerpo.len(),
        lista.len(),
        "el HTML no tiene una <tr> por cada Fila: {html}"
    );

    for (fila, fila_html) in lista.iter().zip(cuerpo.iter()) {
        assert!(
            fila_html.contains(&fila.provider),
            "falta el proveedor en su fila: {fila_html}"
        );
        assert!(
            fila_html.contains(&fila.model),
            "falta el modelo en su fila: {fila_html}"
        );
        assert!(
            fila_html.contains(&fila.canary),
            "falta el texto exacto del canario en su fila: {fila_html}"
        );
        let esfuerzo = fila.effort.as_deref().unwrap_or("—");
        let activo = if fila.enabled { "sí" } else { "no" };
        let enrutable = if fila.routable { "sí" } else { "no" };
        let confirmado = match fila.confirmed {
            Some(true) => "confirmado",
            Some(false) => "sin confirmar",
            None => "—",
        };
        for valor in [esfuerzo, activo, enrutable, confirmado] {
            assert!(
                fila_html.contains(&format!(">{valor}</td>")),
                "la fila HTML perdió `{valor}`: {fila_html}"
            );
        }
        // La misma verdad, no sólo la misma forma: lo que dice `tabla` sobre
        // este modelo tiene que aparecer también en la salida de texto.
        assert!(
            texto.contains(&fila.model),
            "la tabla de texto no trae el modelo: {texto}"
        );
        assert!(
            texto.contains(&fila.canary),
            "la tabla de texto no trae el canario: {texto}"
        );

        if let Some(alias) = &fila.warning {
            assert!(
                fila_html.contains(alias.as_str()),
                "la fila no enseña el alias declarado: {fila_html}"
            );
        }
    }
}

/// Página autocontenida: nada de red, CDN ni fuente externa (T7).
#[test]
fn tabla_html_es_autocontenida_sin_red_ni_cdn() {
    let disposicion = disposicion("html-autocontenida");
    let lista = filas(&proveedores(), &disposicion, Some("alias")).expect("se listan");
    let html = tabla_html(&lista);

    assert!(!html.contains("<link"), "{html}");
    assert!(!html.contains("src="), "{html}");
    assert!(!html.contains("@import"), "{html}");
    assert!(!html.contains("http"), "no es autocontenida: {html}");
}

/// Sólo lectura, y lo dice en el cuerpo de la página, no sólo en un
/// comentario HTML (T7).
#[test]
fn tabla_html_dice_que_es_de_solo_lectura() {
    let disposicion = disposicion("html-solo-lectura");
    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");
    let html = tabla_html(&lista);

    let cuerpo = html
        .split("<body>")
        .nth(1)
        .unwrap_or_else(|| panic!("la página no tiene <body>: {html}"));
    assert!(
        cuerpo.contains("sólo lectura") || cuerpo.contains("solo lectura"),
        "no dice que es de sólo lectura en el cuerpo: {html}"
    );
    assert!(
        cuerpo.contains("no aplica ningún cambio") || cuerpo.contains("no aplica ningun cambio"),
        "no dice que no aplica ningún cambio: {html}"
    );
}

/// Todo valor de texto libre del manifiesto (`provider`, `model`, el alias de
/// `warning`) pasa por escapado antes de insertarse en el HTML: un `<` o un
/// `&` no rompen la estructura de la página, y el escapado exacto importa —
/// escapar en el orden equivocado dobla el escapado (`<` → `&lt;` →
/// `&amp;lt;`).
#[test]
fn tabla_html_escapa_html_de_los_campos_libres() {
    let fila = Fila {
        provider: "prov <b>".to_string(),
        model: "modelo & cosas".to_string(),
        effort: None,
        enabled: true,
        routable: true,
        canary: "verde hace 2 h".to_string(),
        confirmed: Some(true),
        warning: Some("<script>&\"alerta\"</script>".to_string()),
    };

    let html = tabla_html(std::slice::from_ref(&fila));

    assert!(!html.contains("<script>"), "no escapó el alias: {html}");
    assert!(!html.contains("<b>"), "no escapó el proveedor: {html}");
    assert!(
        html.contains("prov &lt;b&gt;"),
        "el proveedor no salió escapado tal cual: {html}"
    );
    assert!(
        html.contains("modelo &amp; cosas"),
        "el modelo no salió escapado tal cual: {html}"
    );
    assert!(
        html.contains("&lt;script&gt;&amp;&quot;alerta&quot;&lt;/script&gt;"),
        "el alias no salió escapado en el orden correcto (& antes que < > \"): {html}"
    );
}

/// T7 (corregido tras releer §2/§3) — `--html <ruta>` escribe de verdad en
/// disco. `escribir_html` es el camino real que usa `ejecutar_panel`: crea el
/// fichero, y lo que queda escrito tiene que ser byte a byte lo mismo que
/// devuelve la función pura `tabla_html` sobre las mismas `filas` — nada se
/// pierde ni se transforma entre construir la página y guardarla.
#[test]
fn escribir_html_escribe_el_fichero_con_la_misma_tabla() {
    let disposicion = disposicion("html-escritura");
    let lista = filas(&proveedores(), &disposicion, Some("alias")).expect("se listan");

    let destino =
        std::env::temp_dir().join(format!("batuta-cli-panel-html-{}.html", std::process::id()));
    let _ = fs::remove_file(&destino);

    escribir_html(&destino, &lista).expect("se escribe el fichero");

    let leido = fs::read_to_string(&destino).expect("el fichero escrito se puede leer");
    assert_eq!(
        leido,
        tabla_html(&lista),
        "el fichero no trae la misma tabla que la función pura sobre las mismas filas"
    );
    assert!(leido.contains("alias-modelo"), "{leido}");
    assert!(leido.contains("ECO_CON_ALIAS"), "{leido}");

    let _ = fs::remove_file(&destino);
}

/// Una ruta de destino que no se puede crear (directorio padre inexistente)
/// falla con un error que nombra la ruta, no con un `panic!` ni un `Ok`
/// silencioso que finja haber escrito algo.
#[test]
fn escribir_html_a_una_ruta_sin_directorio_falla_y_nombra_la_ruta() {
    let disposicion = disposicion("html-escritura-invalida");
    let lista = filas(&proveedores(), &disposicion, Some("eco")).expect("se listan");

    let destino = Path::new("/ruta/que/no/existe/de/verdad/panel.html");
    let error = escribir_html(destino, &lista).expect_err("el directorio padre no existe");

    assert!(
        error.to_string().contains("panel.html"),
        "no nombra la ruta: {error}"
    );
}

/// El cableado del binario también forma parte de T7: no basta con que el
/// parser acepte `--html` y que una función aislada sepa escribir. La orden
/// real debe combinar ambos, respetar `--provider`, crear el fichero pedido y
/// no volcar la tabla HTML por stdout.
#[test]
fn el_binario_panel_html_escribe_la_ruta_y_respeta_el_filtro() {
    let raiz = std::env::temp_dir().join(format!(
        "batuta-cli-panel-html-binario-{}",
        std::process::id()
    ));
    let destino = raiz.join("panel.html");
    let _ = fs::remove_dir_all(&raiz);
    fs::create_dir_all(&raiz).expect("directorio temporal");

    let salida = Command::new(env!("CARGO_BIN_EXE_batuta"))
        .args([
            "panel",
            "--provider",
            "eco",
            "--html",
            destino.to_str().expect("ruta temporal UTF-8"),
        ])
        .env("BATUTA_PROVIDERS", proveedores())
        .env("XDG_STATE_HOME", &raiz)
        .output()
        .expect("el binario se ejecuta");

    assert!(
        salida.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    let stdout = String::from_utf8_lossy(&salida.stdout);
    assert!(
        stdout.contains(destino.to_string_lossy().as_ref()),
        "{stdout}"
    );
    assert!(!stdout.contains("<!doctype html>"), "{stdout}");

    let html = fs::read_to_string(&destino).expect("la orden escribió el HTML");
    assert!(html.contains("eco-modelo"), "{html}");
    assert!(
        !html.contains("alias-modelo"),
        "el filtro no se aplicó: {html}"
    );

    let _ = fs::remove_dir_all(&raiz);
}
