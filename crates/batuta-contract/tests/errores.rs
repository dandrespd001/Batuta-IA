//! El error paraguas y las propiedades que se cobran tarde si faltan.

use batuta_contract::{
    ContractError, IdentifierError, OutputContract, ProviderId, SchemaVersion, SchemaVersionError,
    TaskSpecError, VocabularyError,
};

/// Un error que no es `Send + Sync` bloquea el servidor MCP el día que haya un
/// hilo de por medio, y para entonces el tipo ya está en veinte firmas.
fn exige_error_de_verdad<E: core::error::Error + Send + Sync + 'static>() {}

#[test]
fn todos_los_errores_cruzan_hilos() {
    exige_error_de_verdad::<VocabularyError>();
    exige_error_de_verdad::<IdentifierError>();
    exige_error_de_verdad::<SchemaVersionError>();
    exige_error_de_verdad::<TaskSpecError>();
    exige_error_de_verdad::<ContractError>();
}

/// El paraguas existe para que un crate de arriba pueda escribir `?` sobre
/// cualquiera de los errores del contrato sin envolverlos a mano.
#[test]
fn el_paraguas_recoge_los_cuatro() {
    fn intento(valor: &str) -> Result<(OutputContract, ProviderId), ContractError> {
        let contrato: OutputContract = valor.parse()?;
        let proveedor: ProviderId = "abacus".parse()?;
        SchemaVersion::new(1).require_supported()?;
        Ok((contrato, proveedor))
    }

    assert!(intento("review").is_ok());

    let error = intento("patch").expect_err("'patch' no vale");
    assert!(matches!(error, ContractError::Vocabulary(_)));
    for valido in ["text", "json", "unified_diff", "review"] {
        assert!(error.to_string().contains(valido), "{error}");
    }
}

#[test]
fn el_paraguas_conserva_la_causa() {
    use core::error::Error as _;

    let original = "Abacus".parse::<ProviderId>().unwrap_err();
    let envuelto = ContractError::from(original.clone());

    assert_eq!(envuelto.to_string(), original.to_string());
    let causa = envuelto.source().expect("hay causa");
    assert_eq!(causa.to_string(), original.to_string());
}
