//! Ventana exacta de salud y publicación CAS sin perder observaciones.

use std::collections::BTreeMap;
use std::str::FromStr as _;
use std::sync::{Arc, Barrier};
use std::time::{SystemTime, UNIX_EPOCH};

use batuta_contract::RouteRef;
use batuta_routing::{
    CapabilityIndexV2, CatalogStateV2, EvidenceStateV2, ExecutionPolicyV2, HealthObservationV2,
    HealthOutcomeV2, HealthStateV2, PolicyStateV2, RouteHealth, StateComponentsV2, StateStore,
};

fn root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("batuta-health-window-{nonce}"))
}

fn observation(at: u64, outcome: HealthOutcomeV2, latency_ms: u64) -> HealthObservationV2 {
    HealthObservationV2 {
        at,
        outcome,
        latency_ms,
    }
}

#[test]
fn conserva_veinte_incluye_exitos_ambiguos_y_calcula_p95_por_rango_proximo() {
    let mut health = RouteHealth::healthy();
    for at in 1..=21 {
        let outcome = if at == 1 {
            HealthOutcomeV2::KnownSuccess
        } else if at == 21 {
            HealthOutcomeV2::Ambiguous
        } else {
            HealthOutcomeV2::KnownFailure
        };
        health.record(observation(at, outcome, at));
    }
    assert_eq!(health.observations.len(), 20);
    assert_eq!(health.observations[0].at, 2);
    assert!(health.recent_success_rate.abs() < f64::EPSILON);
    assert_eq!(health.latency_p95_ms, 20);

    let mut successes = RouteHealth::healthy();
    for latency in 1..=20 {
        successes.record(observation(latency, HealthOutcomeV2::KnownSuccess, latency));
    }
    assert!((successes.recent_success_rate - 1.0).abs() < f64::EPSILON);
    assert_eq!(successes.latency_p95_ms, 19);
}

#[test]
fn commit_cas_concurrente_no_pierde_actualizaciones_de_salud() {
    let root = root();
    let route = RouteRef::from_str("dsh/minimax/model-v1").unwrap();
    let store = StateStore::open(root.clone());
    store
        .commit(&StateComponentsV2 {
            catalog: CatalogStateV2 {
                schema_version: 2,
                routes: BTreeMap::new(),
            },
            policy: PolicyStateV2 {
                schema_version: 2,
                execution: ExecutionPolicyV2::new(3, 30_000, 2).unwrap(),
                profiles: BTreeMap::new(),
                routes: BTreeMap::new(),
            },
            evidence: EvidenceStateV2 {
                schema_version: 2,
                projections: vec![],
            },
            health: HealthStateV2 {
                schema_version: 2,
                routes: BTreeMap::from([(route.clone(), RouteHealth::healthy())]),
            },
            capabilities: CapabilityIndexV2 {
                schema_version: 2,
                routes: BTreeMap::new(),
            },
        })
        .unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let mut threads = Vec::new();
    for worker in 0..4_u64 {
        let store = store.clone();
        let route = route.clone();
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            for item in 0..5_u64 {
                store
                    .record_health_observation(
                        &route,
                        &observation(
                            worker * 10 + item,
                            HealthOutcomeV2::KnownSuccess,
                            worker * 10 + item + 1,
                        ),
                    )
                    .unwrap();
            }
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    let loaded = store.load().unwrap();
    let health = &loaded.components.health.routes[&route];
    assert_eq!(health.observations.len(), 20);
    assert!((health.recent_success_rate - 1.0).abs() < f64::EPSILON);
    std::fs::remove_dir_all(root).unwrap();
}
