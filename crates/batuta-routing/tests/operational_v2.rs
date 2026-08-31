//! Contratos sellados de investigación y canarios con efectos observables.

use std::str::FromStr as _;

use batuta_contract::RouteRef;
use batuta_quality::{BenchmarkObservation, SourceKind};
use batuta_routing::{
    CanaryEffectsV2, CanaryScenarioV2, CapabilityCanaryReceiptV2, GrantLimits, ResearchProposalV2,
    ResearchSourceV2, ToolEventV2,
};

fn hash(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn route(value: &str) -> RouteRef {
    RouteRef::from_str(value).unwrap()
}

fn observation(target: RouteRef) -> BenchmarkObservation {
    BenchmarkObservation {
        schema_version: 2,
        id: "obs-1".to_string(),
        route: target,
        benchmark: "swe-bench".to_string(),
        benchmark_version: "v1".to_string(),
        scenario: "verified".to_string(),
        configuration: "official".to_string(),
        scaffold: "official".to_string(),
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        normalized_score: 81.0,
        source_url: "https://example.test/result".to_string(),
        observed_at: 1_000,
        source_kind: SourceKind::Independent,
    }
}

fn source(target: RouteRef) -> ResearchSourceV2 {
    ResearchSourceV2 {
        source_url: "https://example.test/result".to_string(),
        publication: "Independent Lab".to_string(),
        query: "site:example.test swe-bench".to_string(),
        benchmark: "swe-bench".to_string(),
        benchmark_version: "v1".to_string(),
        scenario: "verified".to_string(),
        configuration: "official".to_string(),
        route: target,
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        source_kind: SourceKind::Independent,
    }
}

#[test]
fn research_v2_sella_bases_fuentes_y_rechaza_autocertificacion() {
    let researcher = route("dsh/deepseek/researcher/r1");
    let target = route("dsh/openai/gpt-5.6");
    let proposal = ResearchProposalV2::new(
        "research-1",
        1_000,
        researcher.clone(),
        "grant-research",
        hash('a'),
        hash('b'),
        vec![observation(target.clone())],
        vec![source(target)],
    )
    .unwrap();
    proposal.validate_apply(&hash('a'), &hash('b')).unwrap();

    assert!(
        ResearchProposalV2::new(
            "self-certification",
            1_000,
            researcher.clone(),
            "grant-research",
            hash('a'),
            hash('b'),
            vec![observation(researcher.clone())],
            vec![source(researcher)],
        )
        .is_err()
    );
    assert!(proposal.validate_apply(&hash('c'), &hash('b')).is_err());

    let mut value = serde_json::to_value(&proposal).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ResearchProposalV2>(value).is_err());
}

fn limits() -> GrantLimits {
    GrantLimits {
        requests: 1,
        input_tokens: 100,
        output_tokens: 100,
        wall_time_ms: 1_000,
    }
}

fn event(tool: &str, success: bool) -> ToolEventV2 {
    ToolEventV2 {
        tool: tool.to_string(),
        success,
        result: Some("verified".to_string()),
        result_digest: Some(hash('d')),
        artifact: None,
        source_url: None,
        source_status: None,
    }
}

#[test]
fn canario_tools_exige_evento_exitoso_no_una_mencion() {
    let receipt = CapabilityCanaryReceiptV2::new(
        route("dsh/openai/gpt-5.6"),
        "2026-08",
        CanaryScenarioV2::Tools,
        hash('a'),
        "grant-canary",
        limits(),
        vec![event("shell", true)],
        CanaryEffectsV2::default(),
        2_000,
    )
    .unwrap();
    assert!(receipt.is_positive_at(1_500));

    assert!(
        CapabilityCanaryReceiptV2::new(
            route("dsh/openai/gpt-5.6"),
            "2026-08",
            CanaryScenarioV2::Tools,
            hash('a'),
            "grant-canary",
            limits(),
            vec![event("shell", false)],
            CanaryEffectsV2::default(),
            2_000,
        )
        .is_err()
    );
}

#[test]
fn canarios_read_write_y_web_exigen_efectos_exactos() {
    let base = || CanaryEffectsV2 {
        expected_nonce: Some("nonce-1".to_string()),
        response: Some("nonce-1".to_string()),
        expected_artifact: Some("out/result.txt".to_string()),
        observed_artifact: Some("out/result.txt".to_string()),
        expected_artifact_digest: Some(hash('e')),
        observed_artifact_digest: Some(hash('e')),
        lateral_writes: Vec::new(),
        source_url: Some("https://example.test/source".to_string()),
        source_status: Some(200),
        source_digest: Some(hash('f')),
    };

    for scenario in [
        CanaryScenarioV2::Read,
        CanaryScenarioV2::Write,
        CanaryScenarioV2::Web,
    ] {
        let mut tool = event("read", true);
        if scenario == CanaryScenarioV2::Web {
            tool.source_url = Some("https://example.test/source".to_string());
            tool.source_status = Some(200);
        }
        CapabilityCanaryReceiptV2::new(
            route("dsh/openai/gpt-5.6"),
            "2026-08",
            scenario,
            hash('a'),
            "grant-canary",
            limits(),
            vec![tool],
            base(),
            2_000,
        )
        .unwrap();
    }

    let mut lateral = base();
    lateral.lateral_writes.push("outside.txt".to_string());
    assert!(
        CapabilityCanaryReceiptV2::new(
            route("dsh/openai/gpt-5.6"),
            "2026-08",
            CanaryScenarioV2::Write,
            hash('a'),
            "grant-canary",
            limits(),
            vec![event("write", true)],
            lateral,
            2_000,
        )
        .is_err()
    );
}
