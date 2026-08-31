//! CLI JSON y MCP son sobres de la misma función pura.

use std::collections::BTreeSet;
use std::str::FromStr;

use batuta_cli::{
    ApiResponseV2, ApplicationService, Command, McpServer, RouteRequestEnvelopeV2, RouteRequestV2,
    TuiApp, decision_html, decision_table, parse, route_json,
};
use batuta_contract::{Capability, ReasoningEffort, RouteRef, Sensitivity};
use batuta_quality::QualityProjection;
use batuta_routing::{
    ExecutionPolicyV2, RouteCandidate, RouteClass, RouteDecision, RoutingActionProfile,
    RoutingSnapshot, SelectionMargin,
};

fn fixture() -> (ApplicationService, RouteRequestEnvelopeV2) {
    let route = RouteRef::from_str("dsh/deepseek/deepseek-v4").unwrap();
    let request = RouteRequestV2 {
        schema_version: 2,
        action: "implementation".to_string(),
        required_capabilities: BTreeSet::from([Capability::Write]),
        sensitivity: Sensitivity::Internal,
        required_context: 10_000,
        effort: Some(ReasoningEffort::High),
        minimum_quality: None,
        selection_margin: None,
        predicted_tokens: 5_000,
        allow_any_eligible: None,
        allow_unverified_quality: None,
    };
    let candidate = RouteCandidate {
        route: route.clone(),
        alias: Some("deepseekV4".to_string()),
        enabled: true,
        class: RouteClass::Production,
        capabilities: BTreeSet::from([Capability::Write]),
        max_sensitivity: Sensitivity::Internal,
        context_window: 100_000,
        supported_efforts: BTreeSet::from([ReasoningEffort::High]),
        quality: QualityProjection {
            route,
            action: "implementation".to_string(),
            researched_score: Some(82.0),
            effective_score: Some(82.0),
            coverage: 100,
            contributing_range: None,
            verified: true,
            contributions: vec![],
            exclusions: vec![],
            override_history: vec![],
            active_override: None,
            evidence_hash: "sha256:evidence".to_string(),
        },
        relative_cost: 1.0,
        handoff_penalty: 0.0,
        recent_success_rate: 1.0,
        latency_p95_ms: 100,
        cooldown_until: None,
        approved_fallback: true,
    };
    let snapshot = RoutingSnapshot::new(
        "sha256:policy".to_string(),
        ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
        std::collections::BTreeMap::from([(
            "implementation".to_string(),
            RoutingActionProfile {
                action: "implementation".to_string(),
                minimum_quality: 75.0,
                selection_margin: SelectionMargin::new(5.0).unwrap(),
                allow_any_eligible: false,
                allow_unverified_quality: false,
            },
        )]),
        vec![candidate],
    )
    .unwrap();
    (
        ApplicationService::with_context(snapshot, 1_000, RouteClass::Production, false),
        RouteRequestEnvelopeV2 {
            schema_version: 2,
            request,
        },
    )
}

#[test]
fn route_tui_mcp_y_research_parsean() {
    assert!(matches!(
        parse(&["route".into(), "--json".into(), "{}".into()]).unwrap(),
        Command::Route { .. }
    ));
    assert_eq!(
        parse(&["tui".into()]).unwrap(),
        Command::Tui { route_file: None }
    );
    assert_eq!(
        parse(&["tui".into(), "--route".into(), "request.json".into()]).unwrap(),
        Command::Tui {
            route_file: Some("request.json".to_string())
        }
    );
    assert_eq!(parse(&["mcp".into()]).unwrap(), Command::Mcp);
    assert!(matches!(
        parse(&[
            "research".into(),
            "update".into(),
            "--action".into(),
            "implementation".into()
        ])
        .unwrap(),
        Command::Research { .. }
    ));
}

#[test]
fn cli_json_y_mcp_devuelven_exactamente_la_misma_decision() {
    let (service, envelope) = fixture();
    let direct = route_json(&service, &serde_json::to_string(&envelope).unwrap()).unwrap();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "batuta.route",
            "arguments": envelope
        }
    });
    let response = McpServer::handle_line_with_service(&request.to_string(), &service).unwrap();
    let response: serde_json::Value = serde_json::from_str(&response).unwrap();
    let direct: serde_json::Value = serde_json::from_str(&direct).unwrap();

    assert_eq!(
        response["result"]["structuredContent"], direct,
        "MCP response: {response}"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            response["result"]["content"][0]["text"].as_str().unwrap()
        )
        .unwrap(),
        direct
    );
}

#[test]
fn la_tui_explica_exactamente_la_misma_ruta_y_puntaje() {
    let (service, envelope) = fixture();
    let input = serde_json::to_string(&envelope).unwrap();
    let direct = route_json(&service, &input).unwrap();
    let mut app = TuiApp::new();

    app.explain_route(&service, &input).unwrap();

    assert_eq!(app.route_decision_json(), Some(direct.as_str()));
    assert!(app.snapshot(120).contains("dsh/deepseek/deepseek-v4"));
    assert!(app.snapshot(120).contains("82"));
}

#[test]
fn mcp_no_expone_aceptar_aplicar_parches() {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });
    let response = McpServer::handle_line(&request.to_string()).unwrap();

    assert!(!response.contains("accept"));
    assert!(!response.contains("reject"));
    assert!(!response.contains("patch"));
    assert!(response.contains("batuta.route"));
    assert!(!response.contains("batuta.research.status"));
    assert!(!response.contains("\"root\""));
}

#[test]
fn cli_json_resuelve_umbrales_omitidos_desde_el_perfil() {
    let (service, draft) = fixture();

    let decision: serde_json::Value = serde_json::from_str(
        &route_json(&service, &serde_json::to_string(&draft).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(decision["schema_version"], 2);
    assert_eq!(decision["data"]["route"], "dsh/deepseek/deepseek-v4");
}

#[test]
fn tabla_html_y_tui_muestran_los_mismos_campos_de_la_decision() {
    let (service, envelope) = fixture();
    let input = serde_json::to_string(&envelope).unwrap();
    let json = route_json(&service, &input).unwrap();
    let response: ApiResponseV2<RouteDecision> = serde_json::from_str(&json).unwrap();
    let table = decision_table(&response.data);
    let html = decision_html(&response.data);
    let mut tui = TuiApp::new();
    tui.explain_route(&service, &input).unwrap();
    let snapshot = tui.snapshot(120);

    for expected in [
        "dsh/deepseek/deepseek-v4",
        "82",
        "100",
        "sha256:evidence",
        "sha256:policy",
    ] {
        assert!(table.contains(expected), "table lacks {expected}: {table}");
        assert!(html.contains(expected), "html lacks {expected}: {html}");
        assert!(
            snapshot.contains(expected),
            "tui lacks {expected}: {snapshot}"
        );
    }
    assert!(table.contains("sin override"));
}

#[test]
fn los_ejemplos_json_del_spec_se_validan_automaticamente() {
    let valid = include_str!("../../../docs/examples/route-request-v2.json");
    let invalid =
        include_str!("../../../docs/examples/route-request-v2-invalid-client-candidate.json");

    serde_json::from_str::<RouteRequestEnvelopeV2>(valid).unwrap();
    let error = serde_json::from_str::<RouteRequestEnvelopeV2>(invalid).unwrap_err();
    assert!(error.to_string().contains("candidates"), "{error}");
}
