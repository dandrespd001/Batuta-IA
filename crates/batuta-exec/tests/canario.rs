//! El canario entero, y la admisión que lo rodea.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use batuta_exec::{CanaryRequest, generate_token, run_canary};
use batuta_lease::{LeaseSpace, LeaseStore};
use batuta_manifest::ProviderManifest;

fn fixture(nombre: &str) -> ProviderManifest {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(nombre);
    ProviderManifest::load(&ruta).unwrap_or_else(|e| panic!("{nombre}: {e}"))
}

fn peticion(base: &Path, timeout: Duration) -> CanaryRequest {
    fs::create_dir_all(base.join("arbol")).expect("árbol");
    CanaryRequest {
        workdir: base.join("arbol"),
        run_dir: base.join("corrida"),
        state_dir: base.join("estado"),
        dsh_home: base.join("dsh"),
        timeout,
        task_id: "canario-de-prueba".to_string(),
    }
}

/// Un token predecible dejaría de ser observacional: un proveedor que devolviera
/// texto plausible sin llamar a nadie pasaría el canario. Es el fallo de la
/// puerta circular, que devolvía su veredicto en 126 ms sin tocar la red.
#[test]
fn dos_tokens_de_canario_no_se_parecen() {
    let uno = generate_token().expect("urandom");
    let otro = generate_token().expect("urandom");

    assert_ne!(uno, otro);
    assert!(uno.len() >= 16, "token corto: {uno}");
}

/// **La cadena entera sin una sola llamada de red.**
///
/// Su valor no es la cobertura, es la atribución: cuando el canario contra un
/// proveedor real falle, éste dirá si el fallo es del proveedor o de batuta.
#[test]
fn el_canario_del_eco_sale_verde_sin_confirmar_el_modelo() {
    let base = std::env::temp_dir().join("batuta-canario-eco");
    let _ = fs::remove_dir_all(&base);

    let eco = fixture("eco.toml");
    let recibo = run_canary(
        &eco,
        &eco.models()[0],
        &peticion(&base, Duration::from_secs(10)),
    )
    .expect("el canario debe poder ejecutarse");

    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
    assert_eq!(recibo.exit_code(), Some(0));
    assert!(!recibo.argv().is_empty(), "el argv real va al recibo");

    // `eco.toml` declara `provenance.source = "declared"`: el transporte funciona
    // y el modelo no es comprobable. Las dos cosas a la vez, y dichas.
    assert!(
        !recibo.model_confirmed(),
        "un proveedor sin registro no puede confirmar nada"
    );

    let _ = fs::remove_dir_all(&base);
}

/// **Cierra los criterios 4 y 5 del spec a la vez.**
///
/// R6 tiene dos mitades —«matar el árbol *y liberar el lease*»— y R9 es una
/// promesa de latencia, no de forma: la inspección no puede hacer cola detrás de
/// una delegación. Las tres cosas sólo se ven con una corrida viva.
#[test]
fn el_canario_toma_los_leases_mientras_corre_los_suelta_al_acabar_y_no_estorba_a_quien_mira() {
    let base = std::env::temp_dir().join("batuta-canario-leases");
    let _ = fs::remove_dir_all(&base);
    let peticion = peticion(&base, Duration::from_secs(2));
    let estado = peticion.state_dir.clone();
    let antes = pids_con("sleep 62.5");

    let hilo = std::thread::spawn(move || {
        let dormilon = fixture("dormilon_canario.toml");
        run_canary(&dormilon, &dormilon.models()[0], &peticion)
    });

    // Con la corrida viva: los leases están tomados y mirarlos no cuesta esperar.
    std::thread::sleep(Duration::from_millis(500));
    let store = LeaseStore::open(&estado).expect("el canario ya creó el almacén");

    let reloj = Instant::now();
    let modelos = store.list(LeaseSpace::Model).expect("listar");
    let repos = store.list(LeaseSpace::Repository).expect("listar");
    let tardanza = reloj.elapsed();

    assert_eq!(
        modelos.len(),
        1,
        "el canario tiene que haber tomado el modelo"
    );
    assert_eq!(repos.len(), 1, "y el repositorio");
    assert!(
        tardanza < Duration::from_secs(1),
        "R9 es una promesa de latencia: listar tardó {tardanza:?}"
    );

    let recibo = hilo.join().expect("el hilo no debe entrar en pánico");
    let _ = recibo.expect("aunque salga rojo, el canario devuelve recibo");

    // Al terminar: ni leases ni nietos. Las dos mitades de R6.
    assert!(store.list(LeaseSpace::Model).expect("listar").is_empty());
    assert!(
        store
            .list(LeaseSpace::Repository)
            .expect("listar")
            .is_empty()
    );

    std::thread::sleep(Duration::from_millis(300));
    let sobrevivientes: Vec<_> = pids_con("sleep 62.5").difference(&antes).cloned().collect();
    assert!(
        sobrevivientes.is_empty(),
        "quedaron nietos vivos {sobrevivientes:?}"
    );

    let _ = fs::remove_dir_all(&base);
}

/// El canario materializa fuera del worktree, como manda el diseño.
#[test]
fn los_ficheros_de_corrida_no_acaban_en_el_arbol_del_encargo() {
    let base: PathBuf = std::env::temp_dir().join("batuta-canario-fuera");
    let _ = fs::remove_dir_all(&base);

    let eco = fixture("eco.toml");
    let peticion = peticion(&base, Duration::from_secs(10));
    let arbol = peticion.workdir.clone();

    run_canary(&eco, &eco.models()[0], &peticion).expect("canario");

    let dentro: Vec<_> = fs::read_dir(&arbol)
        .expect("el árbol existe")
        .filter_map(Result::ok)
        .collect();
    assert!(
        dentro.is_empty(),
        "el árbol del encargo tiene que quedar intacto: {dentro:?}"
    );

    let _ = fs::remove_dir_all(&base);
}

/// Los PID que ahora mismo casan con un marcador.
///
/// Se toma **antes y después**, y se compara la diferencia. Consultar la tabla de
/// procesos a secas es frágil: los `sleep` de una suite interrumpida sobreviven un
/// minuto, y la siguiente corrida los ve y falla por algo que no rompió nadie.
/// Es la misma disciplina que el lector de procedencia usa con las sesiones, por
/// el mismo motivo: lo que importa es lo que apareció, no lo que hay.
fn pids_con(marcador: &str) -> std::collections::BTreeSet<String> {
    let salida = Command::new("pgrep")
        .args(["-f", marcador])
        .output()
        .expect("pgrep");
    String::from_utf8_lossy(&salida.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// **El canario resuelve el binario como lo resuelve el manifiesto, no aparte.**
///
/// El primer canario tenía su propia búsqueda: la primera entrada de `resolve`
/// que fuera un fichero. Esa búsqueda no entiende `~` ni `$PATH`, y el `resolve`
/// de dsh empieza por `~`, así que el canario contra dsh no habría encontrado
/// nunca su binario. Habría fallado con «no se pudo lanzar», acusando al
/// proveedor de un fallo de batuta.
///
/// Se prueba con `$PATH` porque prueba lo mismo que `~` sin depender de dónde
/// esté instalado nada. Y de paso trae R11: la resolución del manifiesto
/// comprueba el `sha256`, y la propia no lo miraba.
#[test]
fn el_canario_encuentra_un_binario_que_solo_esta_en_el_path() {
    let base = std::env::temp_dir().join("batuta-canario-path");
    let _ = fs::remove_dir_all(&base);

    let en_path = fixture("eco_en_path.toml");
    let recibo = run_canary(
        &en_path,
        &en_path.models()[0],
        &peticion(&base, Duration::from_secs(10)),
    )
    .expect("`echo` está en el PATH de cualquier Unix");

    assert!(recibo.verdict().is_green(), "{:?}", recibo.verdict());
    assert_eq!(recibo.exit_code(), Some(0));

    let _ = fs::remove_dir_all(&base);
}
