//! Lo que cada motivo tiene que **nombrar** en su mensaje.
//!
//! Existe porque el encargo que produjo estos cuerpos afirmaba que los mensajes
//! los fijaban los tests, y era falso: ninguno de los diez aseveraba sobre el
//! `Display`. El implementador lo detectó y lo reportó como desviación en vez de
//! fingir que cumplía. Esto cierra ese hueco.
//!
//! La disciplina es la de R8, aplicada a los veredictos: **un mensaje que no
//! nombra lo concreto no es un diagnóstico**. `"Harness worker failed with exit
//! 1"` es exactamente el mensaje que costó días, porque no decía qué había
//! fallado ni con qué datos.

use batuta_receipt::RedReason;

#[test]
fn un_proceso_fallido_dice_con_que_codigo_salio() {
    let mensaje = RedReason::ProcessFailed {
        exit_code: Some(127),
    }
    .to_string();
    assert!(mensaje.contains("127"), "{mensaje}");
}

/// `None` es «lo mató una señal», que no es lo mismo que salir con error. El
/// mensaje no puede confundirlos.
#[test]
fn un_proceso_matado_por_senal_no_se_confunde_con_uno_que_salio_mal() {
    let matado = RedReason::ProcessFailed { exit_code: None }.to_string();
    let fallido = RedReason::ProcessFailed { exit_code: Some(1) }.to_string();
    assert_ne!(matado, fallido);
    assert!(
        !matado.contains("None"),
        "el mensaje es para leerlo: {matado}"
    );
}

/// El motivo central de la fase: si corrió otro modelo, el mensaje tiene que
/// enseñar **los dos**, o no sirve para nada.
#[test]
fn una_discrepancia_de_modelo_nombra_el_pedido_y_el_observado() {
    let mensaje = RedReason::ProvenanceMismatch {
        requested: "deepseek-v4-flash".to_string(),
        observed: "MiniMax-M2.7".to_string(),
    }
    .to_string();
    assert!(mensaje.contains("deepseek-v4-flash"), "{mensaje}");
    assert!(mensaje.contains("MiniMax-M2.7"), "{mensaje}");
}

#[test]
fn una_procedencia_ilegible_dice_que_lo_impidio() {
    let mensaje = RedReason::ProvenanceUnreadable {
        detail: "el ultimo marco venia partido".to_string(),
    }
    .to_string();
    assert!(
        mensaje.contains("el ultimo marco venia partido"),
        "{mensaje}"
    );
}

#[test]
fn una_herramienta_no_declarada_sale_por_su_nombre() {
    let mensaje = RedReason::UndeclaredToolUse {
        tools: vec!["web_search".to_string(), "web_fetch".to_string()],
    }
    .to_string();
    assert!(mensaje.contains("web_search"), "{mensaje}");
    assert!(mensaje.contains("web_fetch"), "{mensaje}");
}

#[test]
fn una_violacion_de_alcance_enumera_las_rutas() {
    let mensaje = RedReason::ScopeViolation {
        paths: vec![".scratch/spantest/Cargo.toml".to_string()],
    }
    .to_string();
    assert!(
        mensaje.contains(".scratch/spantest/Cargo.toml"),
        "{mensaje}"
    );
}

#[test]
fn un_hash_que_no_cuadra_ensena_los_dos() {
    let mensaje = RedReason::DigestMismatch {
        expected: "a".repeat(64),
        found: "b".repeat(64),
    }
    .to_string();
    assert!(mensaje.contains(&"a".repeat(64)), "{mensaje}");
    assert!(mensaje.contains(&"b".repeat(64)), "{mensaje}");
}

/// Ninguno puede ser un mensaje vacío ni un `Debug` disfrazado.
#[test]
fn ningun_motivo_se_queda_sin_mensaje() {
    let motivos = [
        RedReason::ExecutableUnresolved,
        RedReason::DigestMismatch {
            expected: "a".repeat(64),
            found: "b".repeat(64),
        },
        RedReason::ProcessFailed { exit_code: Some(1) },
        RedReason::TokenMissing,
        RedReason::ProvenanceUnreadable {
            detail: "x".to_string(),
        },
        RedReason::ProvenanceMismatch {
            requested: "a".to_string(),
            observed: "b".to_string(),
        },
        RedReason::UndeclaredToolUse {
            tools: vec!["t".to_string()],
        },
        RedReason::ScopeViolation {
            paths: vec!["p".to_string()],
        },
    ];

    for motivo in motivos {
        let mensaje = motivo.to_string();
        assert!(mensaje.len() > 15, "mensaje pobre: {mensaje:?}");
        assert!(
            !mensaje.starts_with(&format!("{motivo:?}")[..5.min(mensaje.len())])
                || mensaje.contains(' '),
            "parece un Debug disfrazado: {mensaje:?}"
        );
    }
}
