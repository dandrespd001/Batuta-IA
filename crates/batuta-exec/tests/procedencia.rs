//! Leer del registro lo que ocurrió, y negarse a inventarlo cuando no se puede.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use batuta_exec::{parse_log, project_key, read_after, snapshot};

/// Valor **medido** contra el disco: la sesión de una delegación real vivió ahí.
#[test]
fn el_nombre_de_proyecto_reproduce_el_medido() {
    assert_eq!(
        project_key(Path::new("/tmp/batuta-encargo-manifest/arbol")),
        "--tmp-batuta-encargo-manifest-arbol--"
    );
}

/// El otro medido, y lleva acento a propósito: el escape es la parte que más
/// fácil se equivoca, y una ruta con `á` la ejercita de verdad.
#[test]
fn los_caracteres_no_seguros_se_escapan_como_los_escapa_dsh() {
    assert_eq!(
        project_key(Path::new("/home/adquiod/Imágenes/Project/batuta")),
        "--home-adquiod-Im~00E1genes-Project-batuta--"
    );
}

#[test]
fn el_nombre_de_proyecto_se_trunca_y_sigue_envuelto() {
    let larga = PathBuf::from(format!("/{}", "x".repeat(400)));
    let clave = project_key(&larga);

    assert!(clave.starts_with("--") && clave.ends_with("--"), "{clave}");
    assert!(clave.len() <= 255, "sin truncar: {}", clave.len());
}

/// Ni «la más reciente» ni adivinar: la diferencia entre las dos instantáneas.
#[test]
fn la_instantanea_distingue_la_sesion_nueva_de_las_viejas() {
    let dir = std::env::temp_dir().join("batuta-instantanea");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("session-vieja")).expect("crear");

    let antes = snapshot(&dir);
    assert_eq!(antes.len(), 1);

    fs::create_dir_all(dir.join("session-nueva")).expect("crear");
    let despues = snapshot(&dir);

    let nuevas: Vec<_> = despues.difference(&antes).collect();
    assert_eq!(nuevas, vec![&"session-nueva".to_string()]);

    let _ = fs::remove_dir_all(&dir);
}

fn registro_bueno() -> String {
    [
        r#"{"type":"session","version":0,"id":"session-abc","cwd":"/tmp/x","delegationDepth":0}"#,
        r#"{"type":"permission/preset","seq":0,"data":{"preset":"batuta-escritura"}}"#,
        r#"{"type":"sandbox/mode","seq":1,"data":{"mode":"workspace-write"}}"#,
        r#"{"type":"request/context","data":{"provider":"deepseek-official","model":"deepseek-v4-flash"}}"#,
        r#"{"type":"tool/call","data":{"name":"bash"}}"#,
        r#"{"type":"tool/call","data":{"name":"bash"}}"#,
        r#"{"type":"tool/call","data":{"name":"read"}}"#,
    ]
    .join("\n")
}

#[test]
fn del_registro_salen_modelo_jaula_y_herramientas() {
    let observada =
        parse_log(&registro_bueno(), &["session-abc".to_string()]).expect("registro completo");

    assert_eq!(observada.provider(), "deepseek-official");
    assert_eq!(observada.model(), "deepseek-v4-flash");
    assert_eq!(observada.session_ids(), &["session-abc".to_string()]);
    assert_eq!(observada.sandbox_mode(), Some("workspace-write"));

    let bash = observada
        .tool_calls()
        .iter()
        .find(|(nombre, _)| nombre == "bash")
        .expect("bash se usó");
    assert_eq!(bash.1, 2, "el recuento importa, no sólo el nombre");
}

/// Lo que le pasó al Arquitecto al abrir una sesión en vuelo: el último marco
/// está completo y el registro JSONL de dentro viene partido. Un lector estricto
/// rechaza el fichero entero; el nuestro usa lo que sí está.
#[test]
fn una_cola_partida_no_tira_el_registro_entero() {
    let tronchado = format!(
        "{}\n{{\"type\":\"tool/call\",\"data\":{{\"na",
        registro_bueno()
    );

    let observada = parse_log(&tronchado, &["session-abc".to_string()])
        .expect("lo anterior sigue siendo bueno");
    assert_eq!(observada.model(), "deepseek-v4-flash");
}

/// La otra mitad de la misma regla, y la que impide la implementación cómoda:
/// **no se rellena con lo pedido**.
#[test]
fn un_registro_que_no_nombra_el_modelo_es_un_error_y_no_un_hueco() {
    let sin_modelo = r#"{"type":"session","version":0,"id":"session-abc","cwd":"/tmp/x"}
{"type":"tool/call","data":{"name":"bash"}}"#;

    let error = parse_log(sin_modelo, &["session-abc".to_string()])
        .expect_err("sin modelo no hay procedencia que valga");
    assert!(
        error.to_lowercase().contains("model"),
        "el motivo tiene que decir qué faltaba: {error}"
    );
}

#[test]
fn si_no_aparecio_ninguna_sesion_se_dice_en_vez_de_suponerla() {
    let dir = std::env::temp_dir().join("batuta-sin-sesion");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("crear");

    let antes: BTreeSet<String> = snapshot(&dir);
    let error = read_after(&dir, &antes).expect_err("no apareció ninguna sesión");
    assert!(!error.is_empty(), "el motivo no puede estar vacío");

    let _ = fs::remove_dir_all(&dir);
}
