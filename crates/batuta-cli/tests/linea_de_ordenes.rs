//! El parseo de la orden, y lo que dice cuando no la entiende.
//!
//! Un error de línea de órdenes no es cosmética: es la primera vez que batuta
//! habla con quien la usa. R8 —los vocabularios son cerrados y los errores
//! **enumeran** lo válido— vale aquí igual que en el manifiesto, y por el mismo
//! motivo: `"invalid task_type"` obliga a leer el código fuente para saber qué se
//! podía haber escrito.

use batuta_cli::{Command, USAGE, parse};

fn argumentos(partes: &[&str]) -> Vec<String> {
    partes.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn canary_con_proveedor_y_modelo_se_entiende() {
    let orden = parse(&argumentos(&[
        "canary",
        "--provider",
        "dsh",
        "--model",
        "dsh-deepseek-v4-flash",
    ]))
    .expect("la orden es correcta");

    assert_eq!(
        orden,
        Command::Canary {
            provider: "dsh".to_string(),
            model: Some("dsh-deepseek-v4-flash".to_string()),
            all: false,
        }
    );
}

#[test]
fn el_modelo_es_opcional() {
    let orden = parse(&argumentos(&["canary", "--provider", "abacus"])).expect("orden correcta");

    assert_eq!(
        orden,
        Command::Canary {
            provider: "abacus".to_string(),
            model: None,
            all: false,
        }
    );
}

/// R8 aplicado a la línea de órdenes: el error dice qué se podía escribir.
#[test]
fn una_orden_desconocida_enumera_las_que_hay() {
    let error = parse(&argumentos(&["deploy"])).expect_err("`deploy` no existe");
    let mensaje = error.to_string();

    assert!(mensaje.contains("deploy"), "no nombra lo pedido: {mensaje}");
    assert!(
        mensaje.contains("canary"),
        "no enumera lo válido: {mensaje}"
    );
}

#[test]
fn una_bandera_sin_valor_no_se_traga_la_siguiente() {
    let error = parse(&argumentos(&["canary", "--provider"])).expect_err("`--provider` sin valor");

    assert!(
        error.to_string().contains("--provider"),
        "{}",
        error.to_string()
    );
}

#[test]
fn canary_sin_proveedor_lo_dice() {
    let error = parse(&argumentos(&["canary"])).expect_err("falta `--provider`");

    assert!(
        error.to_string().contains("--provider"),
        "{}",
        error.to_string()
    );
}

/// Una ayuda que promete lo que no hay es peor que no tenerla: quien la lee
/// escribe la orden y descubre el hueco después. Se comprueba contra el parseo,
/// no contra un texto fijo.
#[test]
fn la_ayuda_no_promete_ninguna_bandera_que_el_parseo_no_admita() {
    for orden in [&argumentos(&["--help"]), &argumentos(&["help"])] {
        assert_eq!(parse(orden).expect("la ayuda se pide así"), Command::Help);
    }

    for bandera in ["--provider", "--model"] {
        assert!(USAGE.contains(bandera), "la ayuda omite {bandera}");
    }
    assert!(USAGE.contains("canary"), "la ayuda omite la única orden");

    // Toda bandera larga que la ayuda nombre tiene que ser admitida por el
    // parseo. Es la comprobación que impide que la ayuda envejezca sola.
    for palabra in USAGE.split_whitespace() {
        let bandera = palabra.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
        if !bandera.starts_with("--") || bandera == "--help" {
            continue;
        }
        // Se prueban las dos formas —con valor y sin él— y basta con que una
        // valga: hay banderas que llevan valor (`--model`) y hay interruptores
        // que no (`--all`), y la ayuda nombra las dos.
        let con_valor = parse(&argumentos(&["canary", "--provider", "eco", bandera, "x"]));
        let sin_valor = parse(&argumentos(&["canary", "--provider", "eco", bandera]));
        assert!(
            con_valor.is_ok() || sin_valor.is_ok(),
            "la ayuda nombra {bandera} y el parseo la rechaza de las dos formas"
        );
    }
}

/// `--all` reparte los permisos de un proveedor entero.
///
/// Es la respuesta concreta a «que anadir o quitar modelos sea sencillo»: anadir
/// uno son cinco lineas de manifiesto **mas un canario que pase**, y esta es la
/// orden que lo pasa. Sin ella, R2 —un modelo sin recibo no se enruta— seria una
/// regla que obliga a un trabajo manual por modelo.
#[test]
fn all_pide_el_canario_de_todos_los_modelos() {
    let orden = parse(&argumentos(&["canary", "--provider", "abacus", "--all"])).expect("orden");

    assert_eq!(
        orden,
        Command::Canary {
            provider: "abacus".to_string(),
            model: None,
            all: true,
        }
    );
}

/// `--all` y `--model` se contradicen, y contradecirse es un error, no una
/// preferencia que batuta resuelva por su cuenta. Elegir en silencio entre dos
/// instrucciones incompatibles es la forma en que se pidio un modelo y corrio
/// otro.
#[test]
fn pedir_todos_y_uno_a_la_vez_no_se_resuelve_en_silencio() {
    let error = parse(&argumentos(&[
        "canary",
        "--provider",
        "abacus",
        "--model",
        "abacus-grok-4.6",
        "--all",
    ]))
    .expect_err("`--all` y `--model` no pueden ir juntas");
    let mensaje = error.to_string();

    assert!(mensaje.contains("--all"), "{mensaje}");
    assert!(mensaje.contains("--model"), "{mensaje}");
}

/// T4 (`docs/FASE5_PANEL.md`) — `panel` sin bandera enseña todo.
#[test]
fn panel_sin_bandera_no_filtra() {
    let orden = parse(&argumentos(&["panel"])).expect("orden correcta");
    assert_eq!(orden, Command::Panel { provider: None });
}

/// `--provider` filtra el panel a un solo proveedor.
#[test]
fn panel_con_provider_filtra() {
    let orden = parse(&argumentos(&["panel", "--provider", "dsh"])).expect("orden correcta");
    assert_eq!(
        orden,
        Command::Panel {
            provider: Some("dsh".to_string()),
        }
    );
}

/// Una bandera que `panel` no admite se rechaza nombrando lo que sí (R8).
#[test]
fn panel_con_una_bandera_ajena_la_rechaza() {
    let error =
        parse(&argumentos(&["panel", "--model", "algo"])).expect_err("--model no es de panel");
    assert!(error.to_string().contains("--provider"), "{error}");
}

/// T5 — `enable`/`disable` toman `<proveedor>/<modelo>` posicional, sin
/// bandera: es la referencia entera, sin partir todavía.
#[test]
fn enable_toma_la_referencia_entera() {
    let orden =
        parse(&argumentos(&["enable", "dsh/dsh-deepseek-v4-flash"])).expect("orden correcta");
    assert_eq!(
        orden,
        Command::Enable {
            model_ref: "dsh/dsh-deepseek-v4-flash".to_string(),
        }
    );
}

#[test]
fn disable_toma_la_referencia_entera() {
    let orden = parse(&argumentos(&["disable", "abacus/abacus-routellm"])).expect("orden correcta");
    assert_eq!(
        orden,
        Command::Disable {
            model_ref: "abacus/abacus-routellm".to_string(),
        }
    );
}

/// `enable` sin ninguna referencia lo dice.
#[test]
fn enable_sin_referencia_lo_dice() {
    let error = parse(&argumentos(&["enable"])).expect_err("falta la referencia");
    assert!(error.to_string().contains("proveedor"), "{error}");
}

/// `enable` con algo de más después de la referencia lo rechaza: no elige
/// en silencio cuál de los dos argumentos vale.
#[test]
fn enable_con_un_argumento_de_mas_lo_rechaza() {
    let error =
        parse(&argumentos(&["enable", "dsh/modelo", "sobra"])).expect_err("un argumento de más");
    assert!(error.to_string().contains("sobra"), "{error}");
}

/// `effort` toma la referencia y el nivel, los dos posicionales.
#[test]
fn effort_toma_la_referencia_y_el_nivel() {
    let orden = parse(&argumentos(&[
        "effort",
        "dsh/dsh-deepseek-v4-flash",
        "high",
    ]))
    .expect("orden correcta");
    assert_eq!(
        orden,
        Command::Effort {
            model_ref: "dsh/dsh-deepseek-v4-flash".to_string(),
            level: "high".to_string(),
        }
    );
}

/// `effort` sin nivel lo dice, distinto de sin referencia.
#[test]
fn effort_sin_nivel_lo_dice() {
    let error =
        parse(&argumentos(&["effort", "dsh/dsh-deepseek-v4-flash"])).expect_err("falta el nivel");
    assert!(error.to_string().contains("nivel"), "{error}");
}

#[test]
fn effort_sin_nada_pide_la_referencia_primero() {
    let error = parse(&argumentos(&["effort"])).expect_err("falta todo");
    assert!(error.to_string().contains("proveedor"), "{error}");
}

/// T6 (`docs/FASE5_PANEL.md`) — `nuevo-proveedor <id>`: un solo posicional,
/// sin bandera.
#[test]
fn nuevo_proveedor_toma_el_id() {
    let orden = parse(&argumentos(&["nuevo-proveedor", "mi-proveedor"])).expect("orden correcta");
    assert_eq!(
        orden,
        Command::NuevoProveedor {
            id: "mi-proveedor".to_string(),
        }
    );
}

#[test]
fn nuevo_proveedor_sin_id_lo_dice() {
    let error = parse(&argumentos(&["nuevo-proveedor"])).expect_err("falta el id");
    assert!(error.to_string().contains("id"), "{error}");
}

#[test]
fn nuevo_proveedor_con_un_argumento_de_mas_lo_rechaza() {
    let error = parse(&argumentos(&["nuevo-proveedor", "mi-proveedor", "sobra"]))
        .expect_err("un argumento de más");
    assert!(error.to_string().contains("sobra"), "{error}");
}

/// `nuevo-modelo <proveedor> <id> <ruta>`: tres posicionales.
#[test]
fn nuevo_modelo_toma_los_tres_posicionales() {
    let orden = parse(&argumentos(&[
        "nuevo-modelo",
        "dsh",
        "dsh-modelo-nuevo",
        "Modelo Nuevo",
    ]))
    .expect("orden correcta");
    assert_eq!(
        orden,
        Command::NuevoModelo {
            provider: "dsh".to_string(),
            id: "dsh-modelo-nuevo".to_string(),
            route_model: "Modelo Nuevo".to_string(),
        }
    );
}

#[test]
fn nuevo_modelo_sin_nada_pide_el_proveedor_primero() {
    let error = parse(&argumentos(&["nuevo-modelo"])).expect_err("falta todo");
    assert!(error.to_string().contains("proveedor"), "{error}");
}

#[test]
fn nuevo_modelo_sin_id_lo_dice() {
    let error = parse(&argumentos(&["nuevo-modelo", "dsh"])).expect_err("falta el id");
    assert!(error.to_string().contains("id"), "{error}");
}

#[test]
fn nuevo_modelo_sin_ruta_lo_dice() {
    let error = parse(&argumentos(&["nuevo-modelo", "dsh", "modelo"])).expect_err("falta la ruta");
    assert!(error.to_string().contains("ruta"), "{error}");
}

#[test]
fn nuevo_modelo_con_un_argumento_de_mas_lo_rechaza() {
    let error = parse(&argumentos(&[
        "nuevo-modelo",
        "dsh",
        "modelo",
        "Modelo",
        "sobra",
    ]))
    .expect_err("un argumento de más");
    assert!(error.to_string().contains("sobra"), "{error}");
}

/// `quitar-modelo <proveedor>/<modelo>`: un solo posicional, como
/// `enable`/`disable`.
#[test]
fn quitar_modelo_toma_la_referencia_entera() {
    let orden = parse(&argumentos(&["quitar-modelo", "dsh/dsh-deepseek-v4-flash"])).expect("orden");
    assert_eq!(
        orden,
        Command::QuitarModelo {
            model_ref: "dsh/dsh-deepseek-v4-flash".to_string(),
        }
    );
}

#[test]
fn quitar_modelo_sin_referencia_lo_dice() {
    let error = parse(&argumentos(&["quitar-modelo"])).expect_err("falta la referencia");
    assert!(error.to_string().contains("proveedor"), "{error}");
}

#[test]
fn quitar_modelo_con_un_argumento_de_mas_lo_rechaza() {
    let error = parse(&argumentos(&["quitar-modelo", "dsh/modelo", "sobra"]))
        .expect_err("un argumento de más");
    assert!(error.to_string().contains("sobra"), "{error}");
}
