//! Ejecutar de verdad: el entorno que llega, y el límite del proceso.
//!
//! Todo con proveedores de prueba que apuntan a binarios del sistema. Un canario
//! contra dsh cuesta red, cuota y minutos, y no sirve para ejercitar los casos
//! feos —matar a mitad, agotar el límite, comprobar qué variables llegaron—.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use batuta_exec::{build_env, run};
use batuta_manifest::ProviderManifest;

fn fixture(nombre: &str) -> ProviderManifest {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(nombre);
    ProviderManifest::load(&ruta).unwrap_or_else(|e| panic!("{nombre} debe cargar: {e}"))
}

fn programa(manifiesto: &ProviderManifest) -> PathBuf {
    manifiesto.executable().program().to_path_buf()
}

#[test]
fn el_eco_devuelve_lo_que_se_le_da_y_deja_stderr_vacio() {
    let eco = fixture("eco.toml");
    let salida = run(
        &programa(&eco),
        &["hola-desde-batuta".to_string()],
        &build_env(eco.env()),
        Path::new("/tmp"),
        Duration::from_secs(10),
    )
    .expect("echo debe lanzarse");

    assert_eq!(salida.exit_code, Some(0));
    assert!(
        salida.stdout.contains("hola-desde-batuta"),
        "{:?}",
        salida.stdout
    );
    assert!(salida.stderr.is_empty(), "stderr: {:?}", salida.stderr);
    assert!(!salida.timed_out);
    assert_eq!(salida.argv, vec!["hola-desde-batuta".to_string()]);
}

/// R5 no se comprueba leyendo la allowlist: se comprueba mirando **qué le llegó
/// al hijo**.
#[test]
fn al_hijo_solo_le_llega_lo_permitido() {
    let entorno = fixture("entorno.toml");

    // Dos variables en el entorno del padre: una permitida y otra que no lo está.
    unsafe {
        std::env::set_var("BATUTA_TESTIGO", "presente");
        std::env::set_var("BATUTA_INTRUSA", "no-deberia-pasar");
    }

    let salida = run(
        &programa(&entorno),
        &[],
        &build_env(entorno.env()),
        Path::new("/tmp"),
        Duration::from_secs(10),
    )
    .expect("env debe lanzarse");

    assert!(
        salida.stdout.contains("BATUTA_TESTIGO="),
        "permitida ausente"
    );
    assert!(
        !salida.stdout.contains("BATUTA_INTRUSA"),
        "una variable no nombrada llegó al hijo: nada se hereda sin nombrarlo"
    );
    assert!(
        salida.stdout.contains("BATUTA_FIJADA=valor-fijado"),
        "lo que el manifiesto fija tiene que llegar"
    );

    unsafe {
        std::env::remove_var("BATUTA_TESTIGO");
        std::env::remove_var("BATUTA_INTRUSA");
    }
}

/// `deny` gana sobre `allow`, y no es un capricho: hay variables que el proveedor
/// lee para decidir su **propia contención**. Heredar una movería la jaula sin
/// que nadie lo pidiera.
#[test]
fn lo_denegado_no_pasa_aunque_este_permitido() {
    let entorno = fixture("entorno.toml");
    unsafe {
        std::env::set_var("BATUTA_DOBLE", "en-allow-y-en-deny");
    }

    let construido = build_env(entorno.env());
    assert!(
        !construido.iter().any(|(k, _)| k == "BATUTA_DOBLE"),
        "estaba en las dos listas y debe ganar deny: {construido:?}"
    );

    unsafe {
        std::env::remove_var("BATUTA_DOBLE");
    }
}

/// El recibo lleva **nombres** de variables, nunca valores (R10).
#[test]
fn la_salida_declara_nombres_de_entorno_y_ningun_valor() {
    let entorno = fixture("entorno.toml");
    let salida = run(
        &programa(&entorno),
        &[],
        &build_env(entorno.env()),
        Path::new("/tmp"),
        Duration::from_secs(10),
    )
    .expect("env debe lanzarse");

    assert!(salida.env_names.contains(&"BATUTA_FIJADA".to_string()));
    assert!(
        !salida.env_names.iter().any(|n| n.contains("valor-fijado")),
        "un valor se coló entre los nombres: {:?}",
        salida.env_names
    );
}

/// **R6, la mitad de los procesos.**
///
/// El fallo que la paga: `TaskStop` dejaba el hijo vivo gastando cuota. Aquí el
/// hijo directo tiene dos nietos y espera por ellos, así que matar sólo al hijo
/// no bastaría: hay que matar el grupo.
#[test]
fn agotar_el_limite_mata_el_arbol_entero() {
    let dormilon = fixture("dormilon.toml");
    let argv: Vec<String> = dormilon.invoke().argv().to_vec();

    let salida = run(
        &programa(&dormilon),
        &argv,
        &build_env(dormilon.env()),
        Path::new("/tmp"),
        Duration::from_millis(700),
    )
    .expect("sh debe lanzarse");

    assert!(salida.timed_out, "tenía que agotar el límite");
    assert_eq!(
        salida.exit_code, None,
        "lo mató una señal, que no es lo mismo que salir con error"
    );
    assert!(
        salida.duration < Duration::from_secs(10),
        "no esperó a que terminara: {:?}",
        salida.duration
    );

    // Observacional: los nietos llevan una duración irrepetible a propósito.
    std::thread::sleep(Duration::from_millis(400));
    let vivos = Command::new("pgrep")
        .args(["-f", "sleep 61.5"])
        .output()
        .expect("pgrep");
    let cuenta = String::from_utf8_lossy(&vivos.stdout).lines().count();
    assert_eq!(
        cuenta, 0,
        "quedaron {cuenta} nietos vivos: matar la tarea tiene que matar el árbol"
    );
}
