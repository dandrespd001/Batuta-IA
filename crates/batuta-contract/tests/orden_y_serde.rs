//! El orden de `Sensitivity` es la política, y serde tiene que arrastrar R8
//! hasta los manifiestos TOML y hasta el JSON del MCP.

use batuta_contract::{Capability, OutputContract, Role, Sensitivity, WriteMode};
use serde::{Deserialize, Serialize};

#[test]
fn el_orden_de_sensitivity_es_el_de_la_politica() {
    assert_eq!(
        Sensitivity::ALL,
        &[
            Sensitivity::Public,
            Sensitivity::Internal,
            Sensitivity::Confidential,
            Sensitivity::Secrets,
            Sensitivity::FinancialControl,
            Sensitivity::Deployment,
        ]
    );

    assert!(Sensitivity::Public < Sensitivity::Internal);
    assert!(Sensitivity::Internal < Sensitivity::Confidential);
    assert!(Sensitivity::Confidential < Sensitivity::Secrets);
    assert!(Sensitivity::Secrets < Sensitivity::FinancialControl);
    assert!(Sensitivity::FinancialControl < Sensitivity::Deployment);

    for (indice, nivel) in Sensitivity::ALL.iter().enumerate() {
        assert_eq!(u8::try_from(indice).unwrap(), nivel.rank());
    }
}

#[test]
fn un_techo_admite_lo_suyo_y_lo_de_debajo() {
    assert!(Sensitivity::Public.fits_within(Sensitivity::Internal));
    assert!(Sensitivity::Internal.fits_within(Sensitivity::Internal));
    assert!(!Sensitivity::Secrets.fits_within(Sensitivity::Internal));
    assert!(Sensitivity::Deployment.fits_within(Sensitivity::Deployment));
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Muestra {
    role: Role,
    sensitivity: Sensitivity,
    output_contract: OutputContract,
    write_mode: WriteMode,
    capabilities: Vec<Capability>,
}

fn muestra() -> Muestra {
    Muestra {
        role: Role::BoundedCpp,
        sensitivity: Sensitivity::Internal,
        output_contract: OutputContract::UnifiedDiff,
        write_mode: WriteMode::ValidatedPatch,
        capabilities: vec![Capability::Read, Capability::Write],
    }
}

#[test]
fn ida_y_vuelta_por_toml() {
    let texto = toml::to_string(&muestra()).expect("serializa");
    assert!(texto.contains("role = \"bounded_cpp\""), "{texto}");
    assert!(
        texto.contains("output_contract = \"unified_diff\""),
        "{texto}"
    );
    assert_eq!(
        muestra(),
        toml::from_str::<Muestra>(&texto).expect("parsea")
    );
}

#[test]
fn ida_y_vuelta_por_json() {
    let texto = serde_json::to_string(&muestra()).expect("serializa");
    assert!(
        texto.contains("\"write_mode\":\"validated_patch\""),
        "{texto}"
    );
    assert_eq!(
        muestra(),
        serde_json::from_str::<Muestra>(&texto).expect("parsea")
    );
}

/// R8 no se queda en `FromStr`: el mensaje tiene que llegar igual desde el TOML.
#[test]
fn un_manifiesto_con_valor_invalido_lista_los_validos() {
    let error = toml::from_str::<Muestra>(
        r#"
role = "bounded_cpp"
sensitivity = "internal"
output_contract = "patch"
write_mode = "validated_patch"
capabilities = ["read"]
"#,
    )
    .expect_err("'patch' no es un output_contract");

    let mensaje = error.to_string();
    assert!(mensaje.contains("output_contract"), "{mensaje}");
    for valido in ["text", "json", "unified_diff", "review"] {
        assert!(mensaje.contains(valido), "falta {valido} en: {mensaje}");
    }
}
