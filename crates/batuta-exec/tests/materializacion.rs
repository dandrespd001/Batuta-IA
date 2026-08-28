//! Los ficheros de corrida se escriben **fuera** del árbol del encargo.

use std::fs;
use std::path::{Path, PathBuf};

use batuta_contract::WriteMode;
use batuta_exec::materialize::cae_dentro;
use batuta_exec::{ExecError, RunContext, materialize};
use batuta_manifest::ProviderManifest;

fn dsh() -> ProviderManifest {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../providers/dsh.toml");
    ProviderManifest::load(&ruta).expect("dsh.toml debe cargar")
}

fn contexto(base: &Path, run_dir: PathBuf) -> RunContext {
    RunContext {
        model: "dsh-deepseek-v4-flash".parse().expect("id"),
        route_model: "deepseek-v4-flash".parse().expect("ruta"),
        route_provider: Some("deepseek-official".to_string()),
        workdir: base.join("arbol"),
        run_dir,
        prompt: "Responde exactamente con: T-123".to_string(),
        token: "T-123".to_string(),
        write_mode: WriteMode::ValidatedPatch,
    }
}

#[test]
fn dsh_materializa_sus_dos_ficheros_con_el_contenido_sustituido() {
    let base = std::env::temp_dir().join("batuta-materializa-ok");
    let _ = fs::remove_dir_all(&base);
    let run_dir = base.join("corrida");
    fs::create_dir_all(base.join("arbol")).expect("crear árbol");

    let escritos = materialize(&dsh(), &contexto(&base, run_dir.clone())).expect("materializa");

    assert_eq!(escritos.len(), 2, "dsh necesita dos ficheros de corrida");
    let settings = fs::read_to_string(run_dir.join("settings.yaml")).expect("settings escrito");
    assert!(settings.contains("deepseek-v4-flash"), "{settings}");
    assert!(
        settings.contains("batuta-escritura"),
        "el preset sale del modo de escritura: {settings}"
    );
    assert!(
        !settings.contains('{'),
        "quedó una llave sin sustituir: {settings}"
    );

    let _ = fs::remove_dir_all(&base);
}

/// La comprobación que `batuta-manifest` no podía hacer, porque `parse()` es puro
/// y el worktree no existe cuando se carga un manifiesto.
///
/// Un fichero de configuración de batuta dentro del árbol del encargo aparecería
/// en el diff como si fuera trabajo del modelo.
#[test]
fn un_directorio_de_corrida_dentro_del_worktree_se_rechaza_antes_de_escribir() {
    let base = std::env::temp_dir().join("batuta-materializa-invasion");
    let _ = fs::remove_dir_all(&base);
    let arbol = base.join("arbol");
    fs::create_dir_all(&arbol).expect("crear árbol");

    let dentro = arbol.join("corrida");
    let error = materialize(&dsh(), &contexto(&base, dentro.clone()))
        .expect_err("no puede escribir dentro del worktree");

    assert!(
        matches!(error, ExecError::RuntimeFileInsideWorktree { .. }),
        "{error:?}"
    );
    assert!(
        !dentro.exists(),
        "se rechaza ANTES de escribir: no debe quedar rastro"
    );

    let _ = fs::remove_dir_all(&base);
}

/// Por componentes, no por prefijo de cadena.
///
/// Es el mismo error que `cubre()` evita en la allowlist del `TaskSpec`, donde
/// exige frontera de `/` para que `addons` no cuente como padre de
/// `addons_extra`.
#[test]
fn estar_dentro_se_decide_por_componentes_y_no_por_prefijo() {
    assert!(cae_dentro(
        Path::new("/tmp/arbol/corrida"),
        Path::new("/tmp/arbol")
    ));
    assert!(cae_dentro(Path::new("/tmp/arbol"), Path::new("/tmp/arbol")));
    assert!(
        !cae_dentro(Path::new("/tmp/arbolito"), Path::new("/tmp/arbol")),
        "'/tmp/arbolito' NO está dentro de '/tmp/arbol'"
    );
    assert!(!cae_dentro(
        Path::new("/tmp/otro/corrida"),
        Path::new("/tmp/arbol")
    ));
}
