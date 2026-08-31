//! Persistencia atómica de evidencia activa y staging.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_quality::{
    ActiveEvidence, BenchmarkObservation, ProposalError, ResearchProposal, ResearchStore,
    SourceKind,
};

fn root(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir()
        .join("batuta-quality-store")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn observation() -> BenchmarkObservation {
    BenchmarkObservation {
        schema_version: 2,
        id: "obs-1".to_string(),
        route: RouteRef::from_str("dsh/deepseek/deepseek-v4").unwrap(),
        benchmark: "swe-bench".to_string(),
        benchmark_version: "v1".to_string(),
        scenario: "verified".to_string(),
        configuration: "official".to_string(),
        scaffold: "official".to_string(),
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        normalized_score: 80.0,
        source_url: "https://example.test/obs-1".to_string(),
        observed_at: 1_000,
        source_kind: SourceKind::Independent,
    }
}

#[test]
fn stage_status_y_apply_comparten_el_mismo_almacen_sin_autoaplicar() {
    let store = ResearchStore::open(root("stage-apply"));
    let active = store.load_active().unwrap();
    assert!(active.observations().is_empty());
    let proposal = ResearchProposal::new(
        "proposal-1",
        1_100,
        RouteRef::from_str("codex/openai/gpt-5.6").unwrap(),
        vec![observation()],
        active.evidence_hash(),
    )
    .unwrap();

    store.stage(&proposal).unwrap();
    assert!(store.load_active().unwrap().observations().is_empty());
    assert_eq!(store.status().unwrap().staged, vec!["proposal-1"]);
    assert!(matches!(
        store.apply("proposal-1", false),
        Err(ProposalError::NotConfirmed)
    ));

    let applied = store.apply("proposal-1", true).unwrap();
    assert_eq!(applied.observations().len(), 1);
    assert_eq!(store.load_active().unwrap(), applied);
    assert!(store.status().unwrap().staged.is_empty());
}

#[test]
fn el_fichero_activo_siempre_es_un_documento_completo() {
    let root = root("atomic");
    let store = ResearchStore::open(root.clone());
    store
        .save_active(&ActiveEvidence::new(vec![observation()]).unwrap())
        .unwrap();

    let text = std::fs::read_to_string(root.join("active.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["schema_version"], 1);
    assert!(root.read_dir().unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")
    }));
}
