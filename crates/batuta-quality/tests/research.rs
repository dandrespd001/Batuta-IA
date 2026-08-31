//! Contrato de staging y aplicación confirmada de investigación.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_quality::{
    ActiveEvidence, BenchmarkObservation, ProposalError, ResearchProposal, SourceKind,
};

fn observation(id: &str) -> BenchmarkObservation {
    BenchmarkObservation {
        schema_version: 2,
        id: id.to_string(),
        route: RouteRef::from_str("dsh/deepseek-official/deepseek-v4-flash").unwrap(),
        benchmark: "swe-bench".to_string(),
        benchmark_version: "v1".to_string(),
        scenario: "verified".to_string(),
        configuration: "official".to_string(),
        scaffold: "official".to_string(),
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        normalized_score: 81.0,
        source_url: "https://example.test/primary".to_string(),
        observed_at: 1_000,
        source_kind: SourceKind::Independent,
    }
}

#[test]
fn q6_stage_no_modifica_evidencia_activa_y_apply_exige_confirmacion() {
    let active = ActiveEvidence::new(vec![observation("old")]).unwrap();
    let proposal = ResearchProposal::new(
        "proposal-1",
        1_100,
        RouteRef::from_str("codex/openai/gpt-5.6").unwrap(),
        vec![observation("new")],
        active.evidence_hash().to_string(),
    )
    .unwrap();

    assert_eq!(active.observations()[0].id, "old");
    assert_eq!(active.observations().len(), 1);
    assert_eq!(
        active.apply(&proposal, false).unwrap_err(),
        ProposalError::NotConfirmed
    );

    let applied = active.apply(&proposal, true).unwrap();
    assert_eq!(applied.observations().len(), 2);
    assert_eq!(active.observations().len(), 1);
}

#[test]
fn una_propuesta_alterada_no_se_aplica() {
    let active = ActiveEvidence::new(vec![]).unwrap();
    let mut proposal = ResearchProposal::new(
        "proposal-1",
        1_100,
        RouteRef::from_str("codex/openai/gpt-5.6").unwrap(),
        vec![observation("new")],
        active.evidence_hash().to_string(),
    )
    .unwrap();
    proposal.observations[0].normalized_score = 1.0;

    assert_eq!(
        active.apply(&proposal, true).unwrap_err(),
        ProposalError::HashMismatch
    );
}

#[test]
fn la_ruta_investigadora_no_puede_autocertificar_su_puntaje() {
    let researcher: RouteRef = "dsh/deepseek/researcher/r1".parse().unwrap();
    let mut own = observation("own");
    own.route = researcher.clone();

    let error = ResearchProposal::new(
        "self-certification",
        101,
        researcher,
        vec![own],
        "sha256:active",
    )
    .unwrap_err();

    assert!(error.to_string().contains("researcher_route"), "{error}");
}
