//! Una capacidad sólo se acredita si la procedencia contiene el hecho positivo.

use batuta_contract::Capability;
use batuta_exec::capability_was_observed;
use batuta_receipt::ObservedProvenance;

fn observed(calls: Vec<(&str, u32)>) -> ObservedProvenance {
    ObservedProvenance::new(
        "deepseek".to_string(),
        "deepseek-v4".to_string(),
        vec!["session".to_string()],
        calls
            .into_iter()
            .map(|(name, count)| (name.to_string(), count))
            .collect(),
        Some("workspace-write".to_string()),
        None,
    )
}

#[test]
fn declarar_tools_sin_ninguna_llamada_no_lo_demuestra() {
    assert!(!capability_was_observed(
        Capability::Tools,
        &observed(vec![])
    ));
    assert!(capability_was_observed(
        Capability::Tools,
        &observed(vec![("bash", 1)])
    ));
}

#[test]
fn web_research_exige_una_herramienta_web_observada() {
    assert!(!capability_was_observed(
        Capability::WebResearch,
        &observed(vec![("read", 4)])
    ));
    assert!(capability_was_observed(
        Capability::WebResearch,
        &observed(vec![("web_search", 1)])
    ));
}
