//! Las reglas de derivación del veredicto, una por prueba.
//!
//! Todas comprueban la misma propiedad desde ángulos distintos: **un recibo
//! verde no se declara, se concluye**. Nadie puede pasar un veredicto; `seal()`
//! lo deriva de los hechos.

use std::path::PathBuf;
use std::time::Duration;

use batuta_receipt::{MaterializedFile, ObservedProvenance, Receipt, RedReason, RunFacts};

/// Una corrida que salió bien y cuya procedencia coincide con lo pedido.
fn hechos_buenos() -> RunFacts {
    RunFacts {
        provider: "dsh".to_string(),
        model_requested: "deepseek-v4-flash".to_string(),
        manifest: PathBuf::from("providers/dsh.toml"),
        manifest_sha256: "a".repeat(64),
        argv: vec!["--profile".into(), "headless".into(), "PONG".into()],
        cwd: PathBuf::from("/tmp/corrida/arbol"),
        env_names: vec!["HOME".into(), "PATH".into()],
        runtime_files: vec![MaterializedFile::new(
            PathBuf::from("settings.yaml"),
            "agent-default-model:\n  model: deepseek-v4-flash\n".to_string(),
        )],
        exit_code: Some(0),
        stdout: "PONG-7F3A9\n".to_string(),
        stderr: String::new(),
        duration: Duration::from_secs(3),
        observed: Ok(ObservedProvenance::new(
            "deepseek-official".to_string(),
            "deepseek-v4-flash".to_string(),
            vec!["session-abc".to_string()],
            vec![("bash".to_string(), 4)],
            Some("workspace-write".to_string()),
            Some("batuta-escritura".to_string()),
        )),
        expected_token: Some("PONG-7F3A9".to_string()),
        declared_tools: vec!["bash".to_string()],
        scope_violations: Vec::new(),
    }
}

#[test]
fn una_corrida_coherente_sale_verde() {
    let recibo = Receipt::seal(hechos_buenos());
    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
}

/// La regla central de la fase.
///
/// El fallo que la paga: se pidió `deepseek-v4-flash` tres veces y corrió otro
/// modelo las tres. Un recibo que anotara la petición mentiría sobre lo único
/// que le da valor.
#[test]
fn correr_otro_modelo_del_pedido_es_rojo_y_nombra_los_dos() {
    let mut hechos = hechos_buenos();
    hechos.observed = Ok(ObservedProvenance::new(
        "minimax".to_string(),
        "MiniMax-M2.7".to_string(),
        vec!["session-abc".to_string()],
        Vec::new(),
        None,
        None,
    ));

    let recibo = Receipt::seal(hechos);
    match recibo.verdict().reason() {
        Some(RedReason::ProvenanceMismatch {
            requested,
            observed,
        }) => {
            assert_eq!(requested, "deepseek-v4-flash");
            assert_eq!(observed, "MiniMax-M2.7");
        }
        otro => panic!("se esperaba ProvenanceMismatch: {otro:?}"),
    }
}

/// «No pude leerlo» y «no pasó nada» son cosas distintas.
#[test]
fn una_procedencia_ilegible_es_roja_y_no_se_rellena_con_lo_pedido() {
    let mut hechos = hechos_buenos();
    hechos.observed = Err("registro tronchado en el ultimo marco".to_string());

    let recibo = Receipt::seal(hechos);

    assert!(
        matches!(
            recibo.verdict().reason(),
            Some(RedReason::ProvenanceUnreadable { .. })
        ),
        "{:?}",
        recibo.verdict()
    );
    assert!(
        recibo.observed().is_none(),
        "un recibo sin procedencia legible no puede inventarse una"
    );
}

/// Un canario es observacional: se compara con el token que se generó, y no se
/// busca una subcadena en un juicio propio (R3).
#[test]
fn un_canario_sin_su_token_es_rojo() {
    let mut hechos = hechos_buenos();
    hechos.stdout = "Claro, aquí tienes la respuesta.\n".to_string();

    let recibo = Receipt::seal(hechos);
    assert!(
        matches!(recibo.verdict().reason(), Some(RedReason::TokenMissing)),
        "{:?}",
        recibo.verdict()
    );
}

/// Las herramientas del proveedor no se apagan, se observan.
#[test]
fn usar_una_herramienta_no_declarada_es_rojo_y_la_nombra() {
    let mut hechos = hechos_buenos();
    hechos.observed = Ok(ObservedProvenance::new(
        "deepseek-official".to_string(),
        "deepseek-v4-flash".to_string(),
        vec!["session-abc".to_string()],
        vec![("bash".to_string(), 4), ("web_search".to_string(), 2)],
        Some("workspace-write".to_string()),
        Some("batuta-escritura".to_string()),
    ));

    let recibo = Receipt::seal(hechos);
    match recibo.verdict().reason() {
        Some(RedReason::UndeclaredToolUse { tools }) => {
            assert_eq!(tools, &["web_search".to_string()]);
        }
        otro => panic!("se esperaba UndeclaredToolUse: {otro:?}"),
    }
}

/// El orden de diagnóstico importa: un proceso que ni terminó bien no puede
/// reportarse como «modelo equivocado», aunque también lo fuera.
#[test]
fn un_proceso_fallido_se_diagnostica_antes_que_la_procedencia() {
    let mut hechos = hechos_buenos();
    hechos.exit_code = Some(1);
    hechos.stdout = String::new();
    hechos.stderr = "dsh: TRANSPORT: Connection error.\n".to_string();
    hechos.observed = Ok(ObservedProvenance::new(
        "llamacpp".to_string(),
        "otro-modelo".to_string(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    ));

    let recibo = Receipt::seal(hechos);
    assert!(
        matches!(
            recibo.verdict().reason(),
            Some(RedReason::ProcessFailed { exit_code: Some(1) })
        ),
        "{:?}",
        recibo.verdict()
    );
}

/// stderr se conserva **aunque el proceso saliera con cero**. El día que tres
/// causas distintas dieron el mismo 0-bytes, el error literal estaba ahí.
#[test]
fn el_stderr_se_conserva_integro_en_una_corrida_con_exito() {
    let mut hechos = hechos_buenos();
    hechos.stderr = "aviso del proveedor que nadie deberia perder\n".to_string();

    let recibo = Receipt::seal(hechos);
    assert!(recibo.verdict().is_green());
    assert_eq!(
        recibo.stderr(),
        "aviso del proveedor que nadie deberia perder\n"
    );
}

/// Un recibo se lee y se comparte: lleva los nombres del entorno, nunca sus
/// valores (R10).
#[test]
fn el_recibo_lleva_nombres_de_entorno_y_ningun_valor() {
    let recibo = Receipt::seal(hechos_buenos());
    assert_eq!(
        recibo.env_names(),
        &["HOME".to_string(), "PATH".to_string()]
    );

    let json = recibo.to_json().expect("el recibo debe serializar");
    assert!(json.contains("HOME"), "faltan los nombres");
    assert!(
        !json.contains("\"/home/adquiod\""),
        "un valor de entorno se coló en el recibo"
    );
}

/// La jaula declarada tiene que constar: batuta no la construye, la comprueba.
#[test]
fn el_recibo_anota_la_jaula_que_la_maquina_registro() {
    let recibo = Receipt::seal(hechos_buenos());
    let observada = recibo.observed().expect("la procedencia era legible");
    assert_eq!(observada.sandbox_mode(), Some("workspace-write"));
}

/// El alcance sólo se puede verificar sobre el resultado, porque el proveedor no
/// conoce la allowlist.
#[test]
fn tocar_fuera_de_la_allowlist_es_rojo_y_dice_que_rutas() {
    let mut hechos = hechos_buenos();
    hechos.scope_violations = vec![".scratch/spantest/Cargo.toml".to_string()];

    let recibo = Receipt::seal(hechos);
    match recibo.verdict().reason() {
        Some(RedReason::ScopeViolation { paths }) => {
            assert_eq!(paths, &[".scratch/spantest/Cargo.toml".to_string()]);
        }
        otro => panic!("se esperaba ScopeViolation: {otro:?}"),
    }
}
