//! Contrato de proyección de calidad por ruta y acción.

use std::str::FromStr;

use batuta_contract::RouteRef;
use batuta_quality::{
    ActionProfile, BenchmarkObservation, BenchmarkObservationV1, BenchmarkWeight, ExclusionCode,
    OverrideEvent, OverrideOperation, SourceKind, project,
};

fn route() -> RouteRef {
    RouteRef::from_str("dsh/deepseek-official/deepseek-v4-flash").unwrap()
}

fn observation(
    id: &str,
    benchmark: &str,
    scenario: &str,
    compatibility: &str,
    score: f64,
    source_kind: SourceKind,
    observed_at: u64,
) -> BenchmarkObservation {
    BenchmarkObservation {
        schema_version: 2,
        id: id.to_string(),
        route: route(),
        benchmark: benchmark.to_string(),
        benchmark_version: "v1".to_string(),
        scenario: scenario.to_string(),
        configuration: "official".to_string(),
        scaffold: compatibility.to_string(),
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        normalized_score: score,
        source_url: format!("https://example.test/{id}"),
        observed_at,
        source_kind,
    }
}

fn profile(action: &str, basket: Vec<BenchmarkWeight>) -> ActionProfile {
    ActionProfile::new(action, basket, 50, 1_000).unwrap()
}

fn weight(benchmark: &str, scenario: &str, revision: Option<&str>, value: u8) -> BenchmarkWeight {
    BenchmarkWeight::new(
        benchmark,
        scenario,
        "v1",
        "official",
        "official",
        "pass_rate",
        revision,
        value,
    )
    .unwrap()
}

#[test]
fn q1_las_observaciones_incompatibles_no_se_promedian() {
    let profile = profile(
        "implementation",
        vec![weight("swe-bench", "verified", Some("2026-08"), 100)],
    );
    let observations = vec![
        observation(
            "official",
            "swe-bench",
            "verified",
            "official",
            80.0,
            SourceKind::Independent,
            900,
        ),
        observation(
            "different-scaffold",
            "swe-bench",
            "verified",
            "custom-agent",
            20.0,
            SourceKind::Independent,
            900,
        ),
    ];

    let projection = project(&route(), &profile, &observations, &[], 1_000).unwrap();

    assert_eq!(projection.researched_score, Some(80.0));
    assert_eq!(projection.contributions.len(), 1);
    assert_eq!(projection.contributions[0].observation, "official");
    assert!(
        projection
            .exclusions
            .iter()
            .any(|item| item.observation == "different-scaffold"
                && item.code == ExclusionCode::ScaffoldMismatch)
    );
}

#[test]
fn q2_la_misma_ruta_tiene_puntajes_distintos_por_accion() {
    let observations = vec![
        observation(
            "swe",
            "swe-bench",
            "verified",
            "official",
            86.0,
            SourceKind::Independent,
            900,
        ),
        observation(
            "gaia",
            "gaia",
            "level-2",
            "official",
            61.0,
            SourceKind::Independent,
            900,
        ),
    ];
    let implementation = profile(
        "implementation",
        vec![weight("swe-bench", "verified", Some("2026-08"), 100)],
    );
    let research = profile(
        "web_research",
        vec![weight("gaia", "level-2", Some("2026-08"), 100)],
    );

    assert_eq!(
        project(&route(), &implementation, &observations, &[], 1_000)
            .unwrap()
            .effective_score,
        Some(86.0)
    );
    assert_eq!(
        project(&route(), &research, &observations, &[], 1_000)
            .unwrap()
            .effective_score,
        Some(61.0)
    );
}

#[test]
fn q3_cobertura_rango_y_caducidad_son_visibles() {
    let profile = ActionProfile::new(
        "implementation",
        vec![
            weight("swe-bench", "verified", Some("2026-08"), 70),
            weight("local", "repair", Some("2026-08"), 30),
        ],
        100,
        1_000,
    )
    .unwrap();
    let observations = vec![
        observation(
            "fresh",
            "swe-bench",
            "verified",
            "official",
            84.0,
            SourceKind::Independent,
            900,
        ),
        observation(
            "expired",
            "local",
            "repair",
            "official",
            99.0,
            SourceKind::LocalEvaluation,
            0,
        ),
    ];

    let projection = project(&route(), &profile, &observations, &[], 1_100).unwrap();

    assert_eq!(projection.coverage, 70);
    assert_eq!(projection.researched_score, Some(84.0));
    assert!((projection.contributing_range.unwrap().min - 84.0).abs() < f64::EPSILON);
    assert!(!projection.verified);
    let expired = projection
        .exclusions
        .iter()
        .find(|item| item.observation == "expired")
        .unwrap();
    assert_eq!(expired.code, ExclusionCode::Expired);
    assert_eq!(expired.observed_at, 0);
    assert_eq!(expired.age_seconds, 1_100);
    assert_eq!(expired.expires_at, 1_000);
    assert_eq!(expired.source_url, "https://example.test/expired");
}

#[test]
fn q4_el_fabricante_solo_no_verifica_produccion() {
    let profile = profile("tools", vec![weight("bfcl", "tools", Some("2026-08"), 100)]);
    let observations = vec![observation(
        "vendor",
        "bfcl",
        "tools",
        "official",
        97.0,
        SourceKind::Manufacturer,
        900,
    )];

    let projection = project(&route(), &profile, &observations, &[], 1_000).unwrap();

    assert_eq!(projection.effective_score, Some(97.0));
    assert_eq!(projection.coverage, 100);
    assert!(!projection.verified);
}

#[test]
fn q5_el_override_conserva_el_valor_investigado_y_no_inventa_verificacion() {
    let profile = profile("tools", vec![weight("bfcl", "tools", Some("2026-08"), 100)]);
    let observations = vec![observation(
        "vendor",
        "bfcl",
        "tools",
        "official",
        72.0,
        SourceKind::Manufacturer,
        900,
    )];
    let manual = OverrideEvent::set(
        "override-1",
        88.0,
        "canario interno revisado",
        950,
        "arquitecto",
        Some(72.0),
    )
    .unwrap();

    let clear = OverrideEvent::clear("override-2", "fin de excepción", 975, "arquitecto").unwrap();
    let projection = project(
        &route(),
        &profile,
        &observations,
        std::slice::from_ref(&manual),
        1_000,
    )
    .unwrap();

    assert_eq!(projection.researched_score, Some(72.0));
    assert_eq!(projection.effective_score, Some(88.0));
    assert_eq!(
        projection.active_override.unwrap().researched_original,
        Some(72.0)
    );
    assert_eq!(projection.override_history, vec![manual]);
    assert!(!projection.verified);

    let cleared = project(
        &route(),
        &profile,
        &observations,
        &[projection.override_history[0].clone(), clear.clone()],
        1_000,
    )
    .unwrap();
    assert_eq!(cleared.effective_score, Some(72.0));
    assert_eq!(cleared.active_override, None);
    assert_eq!(cleared.override_history.len(), 2);
    assert_eq!(
        cleared.override_history[1].operation,
        OverrideOperation::Clear
    );
}

#[test]
fn una_ruta_sin_revision_no_mezcla_dos_revisiones() {
    let profile = profile("tools", vec![weight("bfcl", "tools", None, 100)]);
    let mut first = observation(
        "revision-a",
        "bfcl",
        "tools",
        "official",
        90.0,
        SourceKind::Independent,
        900,
    );
    first.model_revision = "rev-a".to_string();
    let mut second = first.clone();
    second.id = "revision-b".to_string();
    second.model_revision = "rev-b".to_string();

    let projection = project(&route(), &profile, &[first, second], &[], 1_000).unwrap();

    assert_eq!(projection.researched_score, None);
    assert_eq!(projection.coverage, 0);
    assert_eq!(
        projection
            .exclusions
            .iter()
            .filter(|item| item.code == ExclusionCode::AmbiguousRevision)
            .count(),
        2
    );
}

#[test]
fn hash_y_resultado_no_dependen_del_orden_de_entrada() {
    let profile = profile("tools", vec![weight("bfcl", "tools", Some("2026-08"), 100)]);
    let one = observation(
        "one",
        "bfcl",
        "tools",
        "official",
        70.0,
        SourceKind::Independent,
        900,
    );
    let two = observation(
        "two",
        "bfcl",
        "tools",
        "official",
        90.0,
        SourceKind::LocalEvaluation,
        910,
    );

    let forward = project(&route(), &profile, &[one.clone(), two.clone()], &[], 1_000).unwrap();
    let reverse = project(&route(), &profile, &[two, one], &[], 1_000).unwrap();

    assert_eq!(forward.researched_score, Some(80.0));
    assert_eq!(forward, reverse);
}

#[test]
fn cada_dimension_incompatible_tiene_un_codigo_estable() {
    let profile = profile("tools", vec![weight("bfcl", "tools", Some("2026-08"), 100)]);
    let base = observation(
        "base",
        "bfcl",
        "tools",
        "official",
        80.0,
        SourceKind::Independent,
        900,
    );
    let mut cases = Vec::new();
    let mut changed = base.clone();
    changed.route = RouteRef::from_str("dsh/other/model").unwrap();
    cases.push((changed, ExclusionCode::RouteMismatch));
    let mut changed = base.clone();
    changed.benchmark = "other".to_string();
    cases.push((changed, ExclusionCode::BenchmarkMismatch));
    let mut changed = base.clone();
    changed.benchmark_version = "v2".to_string();
    cases.push((changed, ExclusionCode::BenchmarkVersionMismatch));
    let mut changed = base.clone();
    changed.scenario = "other".to_string();
    cases.push((changed, ExclusionCode::ScenarioMismatch));
    let mut changed = base.clone();
    changed.configuration = "other".to_string();
    cases.push((changed, ExclusionCode::ConfigurationMismatch));
    let mut changed = base.clone();
    changed.scaffold = "other".to_string();
    cases.push((changed, ExclusionCode::ScaffoldMismatch));
    let mut changed = base.clone();
    changed.metric = "other".to_string();
    cases.push((changed, ExclusionCode::MetricMismatch));
    let mut changed = base;
    changed.model_revision = "other".to_string();
    cases.push((changed, ExclusionCode::RevisionMismatch));

    for (index, (mut observation, expected)) in cases.into_iter().enumerate() {
        observation.id = format!("case-{index}");
        let projection = project(&route(), &profile, &[observation], &[], 1_000).unwrap();
        assert_eq!(projection.exclusions[0].code, expected);
        assert!(!projection.exclusions[0].field.is_empty());
        assert!(!projection.exclusions[0].details.is_empty());
    }
}

#[test]
fn evidencia_v1_solo_entra_mediante_migracion_explicita() {
    let legacy = BenchmarkObservationV1 {
        schema_version: 1,
        id: "legacy".to_string(),
        route: route(),
        benchmark: "bfcl".to_string(),
        benchmark_version: "v1".to_string(),
        scenario: "tools".to_string(),
        configuration: "official".to_string(),
        compatibility_group: "legacy-scaffold".to_string(),
        model_revision: "2026-08".to_string(),
        metric: "pass_rate".to_string(),
        normalized_score: 75.0,
        source_url: "https://example.test/legacy".to_string(),
        observed_at: 900,
        source_kind: SourceKind::Independent,
    };

    let migrated = legacy.migrate().unwrap();
    assert_eq!(migrated.schema_version, 2);
    assert_eq!(migrated.scaffold, "legacy-scaffold");
}

#[test]
fn pesos_que_no_suman_cien_se_rechazan() {
    let error = ActionProfile::new(
        "implementation",
        vec![weight("swe-bench", "verified", Some("2026-08"), 90)],
        50,
        1_000,
    )
    .unwrap_err();

    assert!(error.to_string().contains("100"));
}
