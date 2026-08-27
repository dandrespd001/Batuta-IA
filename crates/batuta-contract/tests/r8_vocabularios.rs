//! R8 — «Vocabularios enumerables; los errores listan los valores válidos».
//!
//! El fallo que la paga: `unknown output_contract: 'patch'` sin decir que valen
//! `text`, `json`, `unified_diff`, `review` (brief §2).
//!
//! Este fichero comprueba la propiedad para **todos** los vocabularios a la vez,
//! no sólo para el que la destapó: si mañana se añade uno que no la cumpla, aquí
//! se rompe.

use batuta_contract::{
    AuthMethod, CanaryExpectation, Capability, ClosedVocabulary, DocumentFormat, ExecutionProfile,
    OutputContract, ParserKind, PromptDelivery, ProvenanceSource, ProviderKind, ReasoningEffort,
    Role, Sensitivity, TrustTier, WriteMode,
};

/// Comprueba la propiedad R8 sobre un vocabulario cualquiera.
fn cumple_r8<V: ClosedVocabulary>() {
    assert!(!V::ALL.is_empty(), "{}: vocabulario vacío", V::NAME);
    assert_eq!(
        V::ALL.len(),
        V::tokens().len(),
        "{}: ALL y tokens() descuadran",
        V::NAME
    );

    for valor in V::ALL {
        let token = valor.as_str();
        assert_eq!(
            *valor,
            token.parse::<V>().expect("el token propio debe parsear"),
            "{}: {token} no hace ida y vuelta",
            V::NAME
        );
    }

    let mut vistos = V::tokens().to_vec();
    vistos.sort_unstable();
    let antes = vistos.len();
    vistos.dedup();
    assert_eq!(antes, vistos.len(), "{}: tokens duplicados", V::NAME);

    let error = "no_existe_este_valor"
        .parse::<V>()
        .expect_err("un valor fuera del vocabulario debe fallar");
    let mensaje = error.to_string();
    assert!(
        mensaje.contains(V::NAME),
        "{}: el error no nombra el vocabulario: {mensaje}",
        V::NAME
    );
    assert!(
        mensaje.contains("no_existe_este_valor"),
        "{}: el error no repite el valor recibido: {mensaje}",
        V::NAME
    );
    for token in V::tokens() {
        assert!(
            mensaje.contains(token),
            "{}: el error no lista el valor válido {token}: {mensaje}",
            V::NAME
        );
    }
}

#[test]
fn todos_los_vocabularios_cumplen_r8() {
    cumple_r8::<AuthMethod>();
    cumple_r8::<CanaryExpectation>();
    cumple_r8::<Capability>();
    cumple_r8::<DocumentFormat>();
    cumple_r8::<ExecutionProfile>();
    cumple_r8::<OutputContract>();
    cumple_r8::<ParserKind>();
    cumple_r8::<PromptDelivery>();
    cumple_r8::<ProvenanceSource>();
    cumple_r8::<ProviderKind>();
    cumple_r8::<ReasoningEffort>();
    cumple_r8::<Role>();
    cumple_r8::<Sensitivity>();
    cumple_r8::<TrustTier>();
    cumple_r8::<WriteMode>();
}

/// El caso medido en el brief §2, con sus cuatro valores exactos.
#[test]
fn r8_regresion_output_contract_patch() {
    let error = "patch"
        .parse::<OutputContract>()
        .expect_err("'patch' no es un output_contract válido");
    let mensaje = error.to_string();

    assert!(mensaje.contains("output_contract"), "{mensaje}");
    assert!(mensaje.contains("patch"), "{mensaje}");
    for valido in ["text", "json", "unified_diff", "review"] {
        assert!(mensaje.contains(valido), "falta {valido} en: {mensaje}");
    }
}
