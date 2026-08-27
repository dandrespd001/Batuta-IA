//! Identificadores validados en la frontera.
//!
//! Un `ProviderId` mal formado no debe poder existir; si existe, viaja hasta el
//! `argv` o hasta el nombre de una credencial sellada. R10 se paga aquí: sellada
//! como `qwen-deepseek-api-key` y buscada como `deepseek-api-key`, semanas sin
//! credencial con la clave válida en la máquina.

use batuta_contract::{
    CredentialName, EnvVarName, GateProfileId, ModelId, ProviderId, RelativePath, RouteModel,
    SchemaVersion,
};

#[test]
fn provider_id_acepta_los_nombres_reales() {
    for bueno in [
        "abacus",
        "deepseek",
        "deepseek-flash",
        "omniroute",
        "qwen_minimax",
    ] {
        let id: ProviderId = bueno.parse().unwrap_or_else(|e| panic!("{bueno}: {e}"));
        assert_eq!(id.as_str(), bueno);
        assert_eq!(id.to_string(), bueno);
    }
}

#[test]
fn provider_id_rechaza_lo_que_no_es_un_slug() {
    for malo in [
        "", "Abacus", "1abacus", "abacus-", "-abacus", "abacus/x", "aba cus", "abacus.",
    ] {
        assert!(
            malo.parse::<ProviderId>().is_err(),
            "{malo:?} no debería ser un provider_id"
        );
    }
    assert!("a".repeat(65).parse::<ProviderId>().is_err());
}

#[test]
fn model_id_admite_el_espacio_de_nombres_del_proveedor() {
    for bueno in [
        "deepseek-v4-flash",
        "ds/deepseek-v4-flash",
        "kimi-code/k3-256k",
        "abacus-glm-5.3-flash",
        "gpt-5.6-luna",
    ] {
        let id: ModelId = bueno.parse().unwrap_or_else(|e| panic!("{bueno}: {e}"));
        assert_eq!(id.as_str(), bueno);
    }
    for malo in ["/ds", "ds/", "ds//x", "ds/../x", "DS", "ds x", ""] {
        assert!(
            malo.parse::<ModelId>().is_err(),
            "{malo:?} no es un model_id"
        );
    }
}

/// El nombre que el proveedor entiende es opaco: `ZAI GLM 5.3 Flash` lleva
/// espacios y mayúsculas y así hay que pasárselo. Por eso es un tipo distinto
/// del `ModelId` canónico de batuta.
#[test]
fn route_model_es_opaco_pero_no_cualquier_cosa() {
    for bueno in [
        "ZAI GLM 5.3 Flash",
        "Grok 4.6",
        "MiniMax-M3",
        "gpt-5.6-luna",
    ] {
        assert_eq!(bueno.parse::<RouteModel>().unwrap().as_str(), bueno);
    }
    for malo in ["", " Grok", "Grok ", "Grok\n4.6", "Grok\t4"] {
        assert!(malo.parse::<RouteModel>().is_err(), "{malo:?}");
    }
}

#[test]
fn credential_name_acepta_los_cuatro_nombres_sellados() {
    for bueno in [
        "deepseek-api-key",
        "minimax-api-key",
        "omniroute_api_key",
        "web_search_proxy_key",
    ] {
        assert_eq!(bueno.parse::<CredentialName>().unwrap().as_str(), bueno);
    }
    for malo in [
        "",
        "..",
        "../otra",
        "qwen/deepseek-api-key",
        "DEEPSEEK_API_KEY",
    ] {
        assert!(malo.parse::<CredentialName>().is_err(), "{malo:?}");
    }
}

#[test]
fn env_var_name_es_mayusculas() {
    for bueno in ["DEEPSEEK_API_KEY", "MINIMAX_API_KEY", "_PRIVADA", "PATH"] {
        assert_eq!(bueno.parse::<EnvVarName>().unwrap().as_str(), bueno);
    }
    for malo in ["", "deepseek_api_key", "1KEY", "MI-VAR", "MI VAR"] {
        assert!(malo.parse::<EnvVarName>().is_err(), "{malo:?}");
    }
}

/// R5 — contención determinista por nombre. Esta es la comprobación que impide
/// que `allowed_write_paths` se escape del árbol.
#[test]
fn relative_path_no_se_sale_del_arbol() {
    for buena in [
        "tests",
        "addons/chunsa_sim/core",
        "docs/specs",
        "crates/batuta-contract/src",
    ] {
        assert_eq!(buena.parse::<RelativePath>().unwrap().as_str(), buena);
    }
    for mala in [
        "",
        "/etc/passwd",
        "..",
        "../fuera",
        "a/../../fuera",
        "a/./b",
        "a//b",
        "a/",
        "~/secretos",
        "a\\b",
        "a\0b",
    ] {
        assert!(
            mala.parse::<RelativePath>().is_err(),
            "{mala:?} debería rechazarse"
        );
    }
}

#[test]
fn gate_profile_id_admite_los_tres_perfiles() {
    for bueno in ["chunsa-docs", "chunsa-standard", "chunsa-determinism"] {
        assert_eq!(bueno.parse::<GateProfileId>().unwrap().as_str(), bueno);
    }
}

/// R8 aplicado a los identificadores: el error dice qué se recibió y qué regla
/// se incumplió, no un «invalid identifier» a secas.
#[test]
fn el_error_de_identificador_explica_la_regla() {
    let error = "Abacus".parse::<ProviderId>().unwrap_err();
    let mensaje = error.to_string();
    assert!(mensaje.contains("provider_id"), "{mensaje}");
    assert!(mensaje.contains("Abacus"), "{mensaje}");
    assert!(
        mensaje.contains("minúscula") || mensaje.contains("lowercase"),
        "{mensaje}"
    );
}

#[test]
fn la_version_de_esquema_en_curso_es_uno() {
    assert_eq!(SchemaVersion::CURRENT.get(), 1);
    assert!(SchemaVersion::CURRENT.is_supported());

    let futura = SchemaVersion::new(2);
    assert!(!futura.is_supported());
    let error = futura.require_supported().unwrap_err();
    let mensaje = error.to_string();
    assert!(mensaje.contains('2'), "{mensaje}");
    assert!(mensaje.contains('1'), "{mensaje}");
}

#[test]
fn los_identificadores_hacen_ida_y_vuelta_por_serde() {
    let id: ProviderId = "deepseek-flash".parse().unwrap();
    let texto = serde_json::to_string(&id).unwrap();
    assert_eq!(texto, "\"deepseek-flash\"");
    assert_eq!(serde_json::from_str::<ProviderId>(&texto).unwrap(), id);

    let error = serde_json::from_str::<ProviderId>("\"Abacus\"").unwrap_err();
    assert!(error.to_string().contains("provider_id"), "{error}");
}
