//! Dónde va el tiempo de batuta, medido sobre trabajo real.
//!
//! Existe por una regla del brief: **nada baja a C «por eficiencia» sin un
//! benchmark previo que enseñe el coste en el perfil**. Este banco no decide: da
//! los números para decidir, y la hipótesis que viene a refutar es que *nada*
//! justifica bajar a C.
//!
//! Sin `criterion`. Para distinguir microsegundos de segundos no hace falta
//! análisis estadístico, y una dependencia de banco es una dependencia igual.
//!
//! ```sh
//! cargo run --release --example coste
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::time::{Duration, Instant};

use batuta_contract::{ModelId, RouteModel, WriteMode};
use batuta_exec::{RunContext, materialize, parse_log, resolve_argv};
use batuta_lease::{LeaseSpace, LeaseStore};
use batuta_manifest::ProviderManifest;

/// Lo que tardó el canario real de dsh, medido el 2026-08-28
/// (`docs/medidas/CANARIOS.md`). Es la vara: todo lo de aquí es una fracción de
/// esto, o no lo es y hay algo que mirar.
const CANARIO_DSH: Duration = Duration::from_millis(2581);

fn main() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let providers = raiz.join("providers");

    println!("# Coste de batuta\n");
    println!("Vara: el canario real de dsh tardó {CANARIO_DSH:?} de pared.\n");
    println!("| Tramo | Entrada | Repeticiones | Total | Por vuelta | % del canario |");
    println!("|---|---|---:|---:|---:|---:|");

    medir(
        "load_dir de providers/",
        "2 manifiestos, ~9 KB",
        200,
        || {
            let _ = ProviderManifest::load_dir(&providers).expect("providers/");
        },
    );

    // El resto necesita un manifiesto ya cargado: se mide el trabajo por corrida,
    // no la carga otra vez.
    let dsh = {
        let ruta = providers.join("dsh.toml");
        let texto = std::fs::read_to_string(&ruta).expect("dsh.toml");
        ProviderManifest::parse(&texto, &ruta).expect("dsh.toml")
    };
    let base = std::env::temp_dir().join("batuta-coste");
    let _ = std::fs::remove_dir_all(&base);
    let contexto = RunContext {
        model: ModelId::from_str("dsh-deepseek-v4-flash").expect("id"),
        route_model: RouteModel::from_str("deepseek-v4-flash").expect("ruta"),
        route_provider: Some("deepseek-official".to_string()),
        workdir: base.join("arbol"),
        run_dir: base.join("corrida"),
        prompt: "Responde exactamente con: batuta-canario-0123456789abcdef".to_string(),
        token: "batuta-canario-0123456789abcdef".to_string(),
        write_mode: WriteMode::ReadOnly,
    };

    medir("resolve_argv de dsh", "5 argumentos", 2000, || {
        let _ = resolve_argv(&dsh, &contexto).expect("argv");
    });

    medir(
        "materialize de dsh",
        "2 documentos JSON a disco",
        200,
        || {
            let _ = materialize(&dsh, &contexto).expect("materializar");
        },
    );

    // El tramo con orden de magnitud sospechoso: descomprimir y recorrer un
    // registro de sesión de verdad. Se coge el más grande que haya.
    if let Some((bytes, ruta)) = registro_mas_grande() {
        let comprimido = std::fs::read(&ruta).expect("registro");
        let crudo = zstd::decode_all(&comprimido[..]).expect("zstd");
        let texto = String::from_utf8_lossy(&crudo).to_string();
        let lineas = texto.lines().count();
        let id = vec![
            ruta.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        ];

        medir(
            "zstd::decode_all del registro",
            &format!("{} KB comprimidos", bytes / 1024),
            20,
            || {
                let _ = zstd::decode_all(&comprimido[..]).expect("zstd");
            },
        );
        medir(
            "parse_log del registro",
            &format!("{lineas} líneas, {} KB crudos", crudo.len() / 1024),
            20,
            || {
                let _ = parse_log(&texto, &id);
            },
        );
    } else {
        println!("| parse_log | *sin registro de sesión en `~/.dsh`* | — | — | — | — |");
    }

    // R9 es una promesa de latencia: se mide con leases de verdad en disco.
    let estado = base.join("estado");
    let store = LeaseStore::open(&estado).expect("almacén");
    let mut guardas = Vec::new();
    for i in 0..50 {
        guardas.push(
            store
                .acquire(LeaseSpace::Model, &format!("modelo-{i}"), "coste")
                .expect("lease"),
        );
    }
    medir("LeaseStore::list", "50 leases vivos", 200, || {
        let _ = store.list(LeaseSpace::Model).expect("listar");
    });
    drop(guardas);

    let _ = std::fs::remove_dir_all(&base);
    println!("\nMedido con `cargo run --release --example coste`.");
}

fn medir(nombre: &str, entrada: &str, vueltas: u32, mut trabajo: impl FnMut()) {
    // Una vuelta en vacío antes de medir: la primera paga el disco frío y las
    // páginas que aún no están, y eso no es el coste del tramo.
    trabajo();

    let reloj = Instant::now();
    for _ in 0..vueltas {
        trabajo();
    }
    let total = reloj.elapsed();
    let por_vuelta = total / vueltas;
    let fraccion = por_vuelta.as_secs_f64() / CANARIO_DSH.as_secs_f64() * 100.0;

    println!(
        "| {nombre} | {entrada} | {vueltas} | {total:.2?} | {por_vuelta:.2?} | {fraccion:.4} % |"
    );
}

/// El registro de sesión más grande que haya bajo `~/.dsh/sessions`.
///
/// Se coge el más grande y no uno cualquiera: lo que interesa medir es el peor
/// caso que este sistema ha producido de verdad, no un caso cómodo.
fn registro_mas_grande() -> Option<(u64, PathBuf)> {
    let dsh_home = std::env::var_os("DSH_HOME").map_or_else(
        || {
            let hogar = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(hogar).join(".dsh")
        },
        PathBuf::from,
    );
    let sesiones = dsh_home.join("sessions");

    let mut mayor: Option<(u64, PathBuf)> = None;
    let mut pendientes: Vec<PathBuf> = vec![sesiones];
    let mut vistos = BTreeSet::new();

    while let Some(dir) = pendientes.pop() {
        if !vistos.insert(dir.clone()) {
            continue;
        }
        let Ok(entradas) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entrada in entradas.flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                pendientes.push(ruta);
            } else if ruta.file_name().is_some_and(|n| n == "session.jsonl.zstd")
                && let Ok(meta) = entrada.metadata()
                && mayor.as_ref().is_none_or(|(mayor, _)| meta.len() > *mayor)
            {
                mayor = Some((meta.len(), ruta));
            }
        }
    }
    mayor
}
