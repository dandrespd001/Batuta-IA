//! T2 (`docs/FASE5_PANEL.md`) — `Politica::{cargar, guardar}` sobre disco.

use batuta_contract::ReasoningEffort;
use batuta_policy::{EleccionModelo, Politica, PoliticaError};

fn id(texto: &str) -> batuta_contract::ModelId {
    texto.parse().expect("id de prueba válido")
}

fn ruta_temporal(nombre: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join("batuta-policy-tests");
    std::fs::create_dir_all(&base).expect("directorio temporal");
    base.join(format!("{nombre}-{}.toml", std::process::id()))
}

/// Guardar y volver a cargar da lo mismo: es la única garantía que un
/// fichero de estado necesita para ser de fiar.
#[test]
fn guardar_y_cargar_da_lo_mismo() {
    let ruta = ruta_temporal("ida-y-vuelta");

    let mut politica = Politica::vacia();
    politica.fijar(
        id("dsh-deepseek-v4-flash"),
        EleccionModelo {
            habilitado: true,
            esfuerzo: Some(ReasoningEffort::High),
        },
    );
    politica.fijar(
        id("abacus-routellm"),
        EleccionModelo {
            habilitado: false,
            esfuerzo: None,
        },
    );

    politica.guardar(&ruta).expect("se guarda");
    let recargada = Politica::cargar(&ruta).expect("se recarga");

    assert_eq!(politica, recargada);
    let _ = std::fs::remove_file(&ruta);
}

/// La decisión que T2 exige documentar «por escrito»: un modelo que la
/// política no menciona nace apagado. Nada se enruta sin una elección
/// explícita.
#[test]
fn un_modelo_no_mencionado_nace_apagado() {
    let politica = Politica::vacia();
    assert!(!politica.esta_habilitado(&id("dsh-deepseek-v4-flash")));
    assert_eq!(politica.esfuerzo(&id("dsh-deepseek-v4-flash")), None);
}

/// `habilitado` no es opcional: una entrada que no lo trae no es una
/// política incompleta que se rellena en silencio, es un fichero que no
/// carga.
#[test]
fn una_entrada_sin_habilitado_no_carga() {
    let ruta = ruta_temporal("sin-habilitado");
    std::fs::write(
        &ruta,
        "schema_version = 1\n\n[modelos.\"dsh-deepseek-v4-flash\"]\nesfuerzo = \"high\"\n",
    )
    .expect("se escribe");

    let error = Politica::cargar(&ruta).expect_err("falta habilitado");
    assert!(matches!(error, PoliticaError::Parse { .. }), "{error:?}");
    let _ = std::fs::remove_file(&ruta);
}

/// Una versión de esquema que batuta no conoce falla al cargar (R1), igual
/// que un manifiesto de proveedor.
#[test]
fn una_version_de_esquema_no_soportada_no_carga() {
    let ruta = ruta_temporal("version-mala");
    std::fs::write(&ruta, "schema_version = 99\n").expect("se escribe");

    let error = Politica::cargar(&ruta).expect_err("versión no soportada");
    assert!(
        matches!(error, PoliticaError::SchemaVersion(_)),
        "{error:?}"
    );
    let _ = std::fs::remove_file(&ruta);
}

/// Un fichero de política vacío -sólo la versión- es válido: es la política
/// del primer arranque, antes de que nadie habilite nada.
#[test]
fn una_politica_recien_estrenada_carga_sin_modelos() {
    let ruta = ruta_temporal("vacia");
    std::fs::write(&ruta, "schema_version = 1\n").expect("se escribe");

    let politica = Politica::cargar(&ruta).expect("una política vacía es válida");
    assert_eq!(politica, Politica::vacia());
    let _ = std::fs::remove_file(&ruta);
}
