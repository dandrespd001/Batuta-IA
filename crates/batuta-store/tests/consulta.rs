//! T3 (`docs/FASE5_PANEL.md`) — la evidencia, consultable.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use batuta_contract::ProvenanceSource;
use batuta_receipt::{MaterializedFile, ObservedProvenance, Receipt, RunFacts};
use batuta_store::{LatestGreen, ReceiptStore};

fn hechos(model_requested: &str, manifest_sha256: &str) -> RunFacts {
    RunFacts {
        provider: "dsh".to_string(),
        model_requested: model_requested.to_string(),
        route_model: "deepseek-v4-flash".to_string(),
        observed_as: None,
        provenance_source: ProvenanceSource::SessionLog,
        manifest: PathBuf::from("providers/dsh.toml"),
        manifest_sha256: manifest_sha256.to_string(),
        argv: vec!["--profile".into(), "headless".into(), "PONG".into()],
        cwd: PathBuf::from("/tmp/corrida/arbol"),
        env_names: vec!["HOME".into(), "PATH".into()],
        runtime_files: vec![MaterializedFile::new(
            PathBuf::from("settings.yaml"),
            "agent-default-model:\n  model: deepseek-v4-flash\n".to_string(),
        )],
        exit_code: Some(0),
        stdout: "PONG-7F3A9\n".to_string(),
        stderr: String::new(),
        duration: Duration::from_secs(3),
        observed: Ok(ObservedProvenance::new(
            "deepseek-official".to_string(),
            "deepseek-v4-flash".to_string(),
            vec!["session-abc".to_string()],
            vec![("bash".to_string(), 4)],
            Some("workspace-write".to_string()),
            Some("batuta-escritura".to_string()),
        )),
        expected_token: Some("PONG-7F3A9".to_string()),
        declared_tools: vec!["bash".to_string()],
        demonstrated_capabilities: BTreeSet::new(),
        scope_violations: Vec::new(),
    }
}

fn directorio(nombre: &str) -> PathBuf {
    let base = std::env::temp_dir()
        .join("batuta-store-tests")
        .join(format!("{nombre}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("directorio de prueba");
    base
}

/// Escribe un recibo en el directorio, con un nombre único, y devuelve la ruta.
fn escribir(dir: &Path, receipt: &Receipt, nombre: &str) -> PathBuf {
    let ruta = dir.join(format!("{nombre}.json"));
    std::fs::write(&ruta, receipt.to_json().expect("serializa")).expect("se escribe");
    ruta
}

const HASH_ACTUAL: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_VIEJO: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// Sin ningún recibo, la respuesta es ausente, no un error.
#[test]
fn sin_ningun_recibo_el_resultado_es_ausente() {
    let dir = directorio("vacio");
    let store = ReceiptStore::open(dir);

    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("un directorio vacío no es un error");

    assert!(matches!(consulta.result, LatestGreen::Absent));
    assert!(consulta.unreadable.is_empty());
}

/// El caso feliz: un recibo verde, del modelo y el manifiesto correctos.
#[test]
fn el_recibo_verde_del_modelo_y_manifiesto_correctos_se_encuentra() {
    let dir = directorio("feliz");
    let recibo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
    escribir(&dir, &recibo, "uno");

    let store = ReceiptStore::open(dir);
    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("se lee");

    match consulta.result {
        LatestGreen::Fresh { receipt, sealed_at } => {
            assert_eq!(receipt.model_requested(), "dsh-deepseek-v4-flash");
            assert!(
                sealed_at <= SystemTime::now(),
                "se selló en el pasado, no en el futuro"
            );
        }
        otro => panic!("se esperaba Fresh: {otro:?}"),
    }
}

/// Invalidación por `manifest_sha256`: un recibo de otro manifiesto no cuenta,
/// aunque sea del modelo correcto y esté en verde.
#[test]
fn un_recibo_de_otro_manifest_sha256_no_cuenta() {
    let dir = directorio("hash-distinto");
    let recibo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_VIEJO));
    escribir(&dir, &recibo, "uno");

    let store = ReceiptStore::open(dir);
    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("se lee");

    assert!(
        matches!(consulta.result, LatestGreen::Absent),
        "{:?}",
        consulta.result
    );
}

/// Entre varios recibos válidos, se elige el más reciente.
#[test]
fn entre_varios_validos_se_elige_el_mas_reciente() {
    let dir = directorio("varios");
    let viejo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
    let ruta_vieja = escribir(&dir, &viejo, "viejo");
    // Se envejece explícitamente: dos escrituras seguidas pueden compartir el
    // mismo milisegundo de mtime, y la prueba no puede depender de que no lo
    // hagan.
    envejecer(&ruta_vieja, Duration::from_secs(120));

    let nuevo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
    escribir(&dir, &nuevo, "nuevo");

    let store = ReceiptStore::open(dir);
    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("se lee");

    match consulta.result {
        LatestGreen::Fresh { .. } => {}
        otro => panic!("se esperaba Fresh: {otro:?}"),
    }
}

/// Un recibo caducado dice **cuándo** caducó: no es lo mismo que no haber
/// encontrado ninguno.
#[test]
fn un_recibo_caducado_dice_desde_cuando() {
    let dir = directorio("caducado");
    let recibo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
    let ruta = escribir(&dir, &recibo, "uno");
    envejecer(&ruta, Duration::from_hours(48));

    let store = ReceiptStore::open(dir);
    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_hours(24),
        )
        .expect("se lee");

    match consulta.result {
        LatestGreen::Expired { at } => {
            assert!(
                at <= SystemTime::now(),
                "caducó en el pasado, no en el futuro"
            );
        }
        otro => panic!("se esperaba Expired: {otro:?}"),
    }
}

/// Un recibo ilegible no es un recibo ausente: se informa aparte, y el
/// escaneo sigue encontrando lo que sí se puede leer.
#[test]
fn un_recibo_ilegible_no_se_confunde_con_uno_ausente() {
    let dir = directorio("ilegible");
    let bueno = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
    escribir(&dir, &bueno, "bueno");
    let roto = dir.join("roto.json");
    std::fs::write(&roto, "esto no es json valido {{{").expect("se escribe basura");

    let store = ReceiptStore::open(dir);
    let consulta = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("un fichero roto no aborta el escaneo entero");

    assert!(matches!(consulta.result, LatestGreen::Fresh { .. }));
    assert_eq!(consulta.unreadable.len(), 1);
    assert_eq!(consulta.unreadable[0].path, roto);
}

/// R9: la inspección nunca hace cola. No hay ningún cerrojo que tomar para
/// leer recibos -son ficheros inmutables, uno por corrida-, y el aserto es de
/// tiempo, no de forma.
#[test]
fn leer_no_toma_cerrojo_y_es_rapido() {
    let dir = directorio("velocidad");
    for i in 0..200 {
        let recibo = Receipt::seal(hechos("dsh-deepseek-v4-flash", HASH_ACTUAL));
        escribir(&dir, &recibo, &format!("recibo-{i}"));
    }

    let store = ReceiptStore::open(dir);
    let reloj = Instant::now();
    let _ = store
        .latest_green(
            "dsh-deepseek-v4-flash",
            HASH_ACTUAL,
            Duration::from_secs(3600),
        )
        .expect("se lee");
    let tardanza = reloj.elapsed();

    assert!(
        tardanza < Duration::from_secs(1),
        "R9 es una promesa de latencia: tardó {tardanza:?}"
    );
}

/// Retrasa el `mtime` de un fichero para simular un recibo más antiguo, sin
/// depender de `sleep` en la propia prueba.
fn envejecer(ruta: &Path, hace: Duration) {
    let objetivo = SystemTime::now() - hace;
    let fichero = std::fs::OpenOptions::new()
        .write(true)
        .open(ruta)
        .expect("se abre para envejecer");
    fichero.set_modified(objetivo).expect("se fija el mtime");
}
