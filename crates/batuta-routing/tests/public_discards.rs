//! Los descartes públicos tienen una forma estable y cerrada.

use batuta_contract::Capability;
use batuta_routing::DiscardReason;

#[test]
fn cada_descarte_publico_expone_codigo_campo_mensaje_y_detalles() {
    let value = serde_json::to_value(DiscardReason::MissingCapability {
        capability: Capability::WebResearch,
    })
    .unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "code": "missing_capability",
            "field": "task.required_capabilities",
            "message": "route does not provide a required capability",
            "details": {"capability": "web_research"}
        })
    );
    assert_eq!(
        serde_json::from_value::<DiscardReason>(value).unwrap(),
        DiscardReason::MissingCapability {
            capability: Capability::WebResearch,
        }
    );
}

#[test]
fn descarte_publico_rechaza_campos_desconocidos() {
    let value = serde_json::json!({
        "code": "disabled",
        "field": "policy.routes.enabled",
        "message": "route is disabled by policy",
        "details": {},
        "internal_candidate": true
    });
    assert!(serde_json::from_value::<DiscardReason>(value).is_err());
}
