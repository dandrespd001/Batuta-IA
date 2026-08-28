//! Las reglas de derivación del veredicto, una por prueba.
//!
//! Todas comprueban la misma propiedad desde ángulos distintos: **un recibo
//! verde no se declara, se concluye**. Nadie puede pasar un veredicto; `seal()`
//! lo deriva de los hechos.

use std::path::PathBuf;
use std::time::Duration;

use batuta_contract::ProvenanceSource;
use batuta_receipt::{MaterializedFile, ObservedProvenance, Receipt, RedReason, RunFacts};

/// Una corrida que salió bien y cuya procedencia coincide con lo pedido.
fn hechos_buenos() -> RunFacts {
    RunFacts {
        provider: "dsh".to_string(),
        model_requested: "dsh-deepseek-v4-flash".to_string(),
        route_model: "deepseek-v4-flash".to_string(),
        observed_as: None,
        provenance_source: ProvenanceSource::SessionLog,
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

/// **El tercer estado.**
///
/// Abacus no deja registro legible y su modelo lo elige el producto, no la
/// bandera. Con dos estados sólo cabían dos salidas y las dos eran malas: o todo
/// recibo suyo salía rojo, o se fabricaba una procedencia con el modelo pedido —y
/// eso es exactamente la mentira que este crate existe para impedir—.
///
/// Un verde de un proveedor `declared` significa **«el transporte funciona»**, no
/// «corrió el modelo que pedí». El recibo tiene que decirlo, no dejar que el
/// lector suponga lo segundo.
#[test]
fn un_proveedor_sin_registro_sale_verde_pero_sin_confirmar_el_modelo() {
    let mut hechos = hechos_buenos();
    hechos.provenance_source = ProvenanceSource::Declared;
    hechos.observed = Err("este proveedor no deja registro de sesión".to_string());

    let recibo = Receipt::seal(hechos);

    assert!(
        recibo.verdict().is_green(),
        "no poder comprobar el modelo no es un fallo de la corrida: {:?}",
        recibo.verdict()
    );
    assert!(
        !recibo.model_confirmed(),
        "y sin embargo el modelo NO quedó confirmado"
    );
}

/// La otra mitad, que es la que impide relajar la regla por comodidad: donde sí
/// hay registro, no poder leerlo sigue siendo rojo.
#[test]
fn donde_hay_registro_no_poder_leerlo_sigue_siendo_rojo() {
    let mut hechos = hechos_buenos();
    hechos.provenance_source = ProvenanceSource::SessionLog;
    hechos.observed = Err("marco tronchado".to_string());

    let recibo = Receipt::seal(hechos);

    assert!(
        matches!(
            recibo.verdict().reason(),
            Some(RedReason::ProvenanceUnreadable { .. })
        ),
        "{:?}",
        recibo.verdict()
    );
    assert!(!recibo.model_confirmed());
}

/// Una corrida con registro legible y modelo coincidente **sí** confirma.
#[test]
fn con_registro_legible_y_modelo_coincidente_el_modelo_queda_confirmado() {
    let recibo = Receipt::seal(hechos_buenos());
    assert!(recibo.verdict().is_green());
    assert!(recibo.model_confirmed());
}

/// El recibo se lee y se comparte: la diferencia entre «comprobado» y «no
/// comprobable» tiene que estar en el documento, no en la cabeza de quien lo
/// escribió.
#[test]
fn el_json_del_recibo_dice_si_el_modelo_quedo_confirmado() {
    let mut hechos = hechos_buenos();
    hechos.provenance_source = ProvenanceSource::Declared;
    hechos.observed = Err("sin registro".to_string());

    let json = Receipt::seal(hechos).to_json().expect("serializa");
    assert!(
        json.contains("model_confirmed"),
        "el recibo no dice si el modelo se pudo comprobar: {json}"
    );
    assert!(json.contains("false"), "{json}");
}

/// **El identificador de batuta y el nombre que el proveedor entiende son dos
/// cosas, y el registro sólo conoce el segundo.**
///
/// El manifiesto de dsh llama `dsh-deepseek-v4-flash` a un modelo cuyo
/// `route_model` es `deepseek-v4-flash`, y el registro de sesión anota el
/// segundo. Comparar el registro contra el identificador de batuta habría dado
/// `ProvenanceMismatch` en **todas** las corridas de dsh, para siempre: el
/// canario nunca habría podido salir verde, y el motivo habría acusado al
/// proveedor de correr un modelo distinto cuando corría el correcto.
///
/// El test viejo no lo veía porque ponía el mismo nombre en los dos sitios, que
/// es justo el caso que no ocurre en ningún manifiesto real.
#[test]
fn el_registro_se_compara_con_el_nombre_que_el_proveedor_entiende() {
    let hechos = hechos_buenos();
    assert_ne!(
        hechos.model_requested, hechos.route_model,
        "el fixture tiene que usar los dos nombres, o no prueba nada"
    );

    let recibo = Receipt::seal(hechos);

    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
    assert!(recibo.model_confirmed());
    // Y el recibo conserva el identificador de batuta, que es por el que se pidió.
    assert_eq!(recibo.model_requested(), "dsh-deepseek-v4-flash");
    assert_eq!(recibo.route_model(), "deepseek-v4-flash");
}

/// Y la discrepancia de verdad sigue siendo roja, nombrando los dos nombres del
/// mismo espacio: el que se pidió al proveedor y el que el proveedor anotó.
#[test]
fn un_modelo_de_ruta_distinto_del_anotado_sigue_siendo_rojo() {
    let mut hechos = hechos_buenos();
    hechos.observed = Ok(ObservedProvenance::new(
        "minimax".to_string(),
        "minimax-m2".to_string(),
        vec!["session-abc".to_string()],
        vec![("bash".to_string(), 4)],
        Some("workspace-write".to_string()),
        Some("batuta-escritura".to_string()),
    ));

    let recibo = Receipt::seal(hechos);

    assert_eq!(
        *recibo.verdict(),
        batuta_receipt::Verdict::Red(RedReason::ProvenanceMismatch {
            requested: "deepseek-v4-flash".to_string(),
            observed: "minimax-m2".to_string(),
        }),
        "el motivo compara los dos nombres del mismo espacio"
    );
    assert!(!recibo.model_confirmed());
}

/// **El nombre que la máquina anota es un tercer nombre, y se declara.**
///
/// Medido contra abacus el 2026-08-28: se pide `Qwen3.8 Max` y su stderr anota
/// `QWEN3_8_MAX_THINKING`. Sin `observed_as`, la única salida sería normalizar
/// —mayúsculas y espacios a guiones bajos—, que habría acertado en siete de los
/// nueve modelos y habría tapado los dos únicos interesantes: los que Abacus
/// resolvió a una variante **distinta** de la pedida.
///
/// Un normalizador convierte una discrepancia en una coincidencia. Un alias
/// declarado la conserva.
#[test]
fn el_alias_declarado_es_lo_que_se_contrasta_con_el_registro() {
    let mut hechos = hechos_buenos();
    hechos.route_model = "Qwen3.8 Max".to_string();
    hechos.observed_as = Some("QWEN3_8_MAX_THINKING".to_string());
    hechos.observed = Ok(ObservedProvenance::new(
        "abacus".to_string(),
        "QWEN3_8_MAX_THINKING".to_string(),
        vec!["conv-1".to_string()],
        vec![("bash".to_string(), 4)],
        None,
        None,
    ));

    let recibo = Receipt::seal(hechos);

    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
    assert!(
        recibo.model_confirmed(),
        "con alias declarado y registro que lo nombra, el modelo SÍ está confirmado"
    );
    // Y el recibo conserva los tres nombres, que es lo que permite ir en las dos
    // direcciones.
    assert_eq!(recibo.route_model(), "Qwen3.8 Max");
    assert_eq!(recibo.observed_as(), Some("QWEN3_8_MAX_THINKING"));
}

/// Y el alias no tapa una discrepancia de verdad: si la máquina anota otra cosa
/// distinta del alias, sigue siendo rojo.
#[test]
fn un_alias_declarado_no_tapa_una_discrepancia() {
    let mut hechos = hechos_buenos();
    hechos.route_model = "Gemini 3.7 Flash".to_string();
    hechos.observed_as = Some("GEMINI_3_7_FLASH_THINKING".to_string());
    hechos.observed = Ok(ObservedProvenance::new(
        "abacus".to_string(),
        "GEMINI_3_7_FLASH".to_string(),
        vec!["conv-1".to_string()],
        vec![("bash".to_string(), 4)],
        None,
        None,
    ));

    let recibo = Receipt::seal(hechos);

    assert!(
        !recibo.verdict().is_green(),
        "una variante distinta es roja"
    );
    assert!(!recibo.model_confirmed());
}
