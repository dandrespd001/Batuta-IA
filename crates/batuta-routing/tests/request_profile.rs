//! Los valores omitidos se resuelven desde el perfil, no desde defaults ocultos.

use std::collections::BTreeSet;

use batuta_contract::{Capability, ReasoningEffort, Sensitivity};
use batuta_routing::{RouteClass, RouteRequestDraft, RoutingActionProfile, SelectionMargin};

#[test]
fn calidad_y_margen_omitidos_salen_del_perfil_pero_permisos_no_se_autosolicitan() {
    let profile = RoutingActionProfile {
        action: "implementation".to_string(),
        minimum_quality: 81.0,
        selection_margin: SelectionMargin::new(3.0).unwrap(),
        allow_any_eligible: true,
        allow_unverified_quality: false,
    };
    let request = RouteRequestDraft {
        schema_version: 1,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 10_000,
        effort: Some(ReasoningEffort::High),
        minimum_quality: None,
        selection_margin: None,
        predicted_tokens: 20_000,
        allow_any_eligible: None,
        allow_unverified_quality: None,
        fallback: false,
        class: RouteClass::Production,
        now: 1_000,
    }
    .resolve(&profile)
    .unwrap();

    assert!((request.minimum_quality - 81.0).abs() < f64::EPSILON);
    assert!((request.selection_margin.get() - 3.0).abs() < f64::EPSILON);
    assert!(!request.authorizations.allow_any_eligible.requested);
    assert!(!request.authorizations.allow_any_eligible.permitted);
    assert!(!request.authorizations.allow_unverified_quality.permitted);
}

#[test]
fn un_valor_presente_gana_al_perfil_y_una_accion_distinta_falla() {
    let profile = RoutingActionProfile {
        action: "implementation".to_string(),
        minimum_quality: 81.0,
        selection_margin: SelectionMargin::new(3.0).unwrap(),
        allow_any_eligible: false,
        allow_unverified_quality: false,
    };
    let mut draft = RouteRequestDraft {
        schema_version: 1,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::new(),
        sensitivity: Sensitivity::Public,
        required_context: 0,
        effort: None,
        minimum_quality: Some(75.0),
        selection_margin: Some(SelectionMargin::new(8.0).unwrap()),
        predicted_tokens: 1_000,
        allow_any_eligible: Some(true),
        allow_unverified_quality: Some(true),
        fallback: false,
        class: RouteClass::Production,
        now: 1_000,
    };

    let request = draft.clone().resolve(&profile).unwrap();
    assert!((request.minimum_quality - 75.0).abs() < f64::EPSILON);
    assert!((request.selection_margin.get() - 8.0).abs() < f64::EPSILON);
    assert!(request.authorizations.allow_unverified_quality.requested);
    assert!(!request.authorizations.allow_unverified_quality.permitted);

    draft.action = "review".to_string();
    assert!(
        draft
            .resolve(&profile)
            .unwrap_err()
            .to_string()
            .contains("action")
    );
}
