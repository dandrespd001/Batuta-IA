//! T6 (`docs/FASE5_PANEL.md`) — `nuevo-proveedor`, `nuevo-modelo`,
//! `quitar-modelo`.
//!
//! Primero las funciones puras de texto (sin tocar disco): la plantilla, el
//! anexado y la extracción. Los casos de extremo a extremo con fixtures van
//! después, en la misma suite (mismo patrón que `tests/eleccion.rs`, que
//! mezcla pruebas puras y de disco en un solo fichero).

use std::fs;
use std::path::{Path, PathBuf};

use batuta_cli::{
    anexar_modelo, nuevo_modelo, nuevo_proveedor, plantilla_proveedor, quitar_modelo,
    quitar_modelo_de,
};
use batuta_manifest::ProviderManifest;

/// Los fixtures compartidos, sin tocarlos: `nuevo-proveedor`/`nuevo-modelo`/
/// `quitar-modelo` escriben, así que cada prueba de disco trabaja sobre una
/// COPIA en un directorio temporal, nunca sobre `tests/fixtures/` en sí.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Un directorio de proveedores temporal, con copias de los fixtures
/// pedidos. Se limpia primero por si quedó basura de una corrida anterior.
fn proveedores_temporales(nombre: &str, ficheros: &[&str]) -> PathBuf {
    let raiz = std::env::temp_dir().join(format!("batuta-cli-declaracion-{nombre}"));
    let _ = fs::remove_dir_all(&raiz);
    fs::create_dir_all(&raiz).expect("se crea el directorio temporal");
    for fichero in ficheros {
        let origen = fixtures_dir().join(fichero);
        let destino = raiz.join(fichero);
        fs::copy(&origen, &destino).unwrap_or_else(|e| panic!("copiando {fichero}: {e}"));
    }
    raiz
}

/// Manifiesto de prueba, en memoria: dos modelos, sin tocar disco. Sirve para
/// las pruebas puras de `anexar_modelo` y `quitar_modelo`, que no necesitan
/// ningún fixture en `tests/fixtures/` — ese directorio lo comparten
/// `eleccion.rs` y `panel.rs`, y un fichero nuevo ahí cambiaría lo que ven sus
/// pruebas de enumeración.
const DOS_MODELOS: &str = r#"# Comentario de cabecera del manifiesto de prueba.
schema_version = 1
id             = "prueba"
kind           = "cli"

[executable]
program       = "/bin/echo"
version_pin   = "coreutils"
version_probe = ["--version"]
resolve       = ["/bin/echo"]

[invoke]
argv    = ["{prompt}"]
workdir = "worktree"
prompt  = { via = "argv" }

[env]
allow = ["HOME", "PATH"]

[auth]
method = "oauth_cli"

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id              = "modelo-uno"
route_model     = "Modelo Uno"
roles           = ["implementation"]
max_sensitivity = "internal"

[[models]]
id              = "modelo-dos"
route_model     = "Modelo Dos"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {token}"
expect = "token_echo"
"#;

/// La plantilla de un proveedor nuevo tiene que cargar tal cual con
/// `ProviderManifest::parse`. Esto se comprueba aquí explícitamente y no sólo
/// en el código que la usa: si alguien edita la plantilla después y deja de
/// parsear, este test lo revienta en CI, no en producción para un usuario.
#[test]
fn la_plantilla_de_un_proveedor_nuevo_parsea() {
    let texto = plantilla_proveedor("mi-proveedor-nuevo");
    ProviderManifest::parse(&texto, Path::new("plantilla.toml"))
        .expect("la plantilla tiene que parsear tal cual");
}

/// El texto devuelto por `anexar_modelo` SIEMPRE empieza con los bytes
/// exactos del texto original, sin modificarlos: se prueba con un texto que
/// NO termina en salto de línea, el caso que más fácil rompe una
/// implementación que reconstruye el fichero en vez de sólo añadir al final.
#[test]
fn anexar_modelo_preserva_el_texto_original_como_prefijo_exacto() {
    let texto_sin_salto_final = "linea uno\nlinea dos sin salto final al terminar";
    let resultado = anexar_modelo(texto_sin_salto_final, "modelo-nuevo", "Modelo Nuevo");

    assert!(
        resultado.starts_with(texto_sin_salto_final),
        "el texto original tiene que sobrevivir byte a byte como prefijo; salió: {resultado:?}"
    );
}

/// `anexar_modelo` añade al final: el resultado sigue parseando, conserva los
/// modelos anteriores en su mismo orden, y el nuevo aparece último.
#[test]
fn anexar_modelo_anade_al_final_y_conserva_los_anteriores() {
    let resultado = anexar_modelo(DOS_MODELOS, "modelo-tres", "Modelo Tres");

    let manifiesto = ProviderManifest::parse(&resultado, Path::new("prueba.toml"))
        .expect("el texto con el modelo añadido tiene que seguir parseando");
    let ids: Vec<&str> = manifiesto
        .models()
        .iter()
        .map(|m| m.id().as_str())
        .collect();
    assert_eq!(ids, vec!["modelo-uno", "modelo-dos", "modelo-tres"]);
}

/// Tres modelos, todos separados por una sola línea en blanco — sirve para
/// probar que quitar el primero, el del medio o el último no deja líneas en
/// blanco dobles (el bloque incluye naturalmente el blanco que lo separaba
/// del siguiente).
const TRES_MODELOS: &str = r#"schema_version = 1
id             = "prueba-tres"
kind           = "cli"

[executable]
program       = "/bin/echo"
version_pin   = "coreutils"
version_probe = ["--version"]
resolve       = ["/bin/echo"]

[invoke]
argv    = ["{prompt}"]
workdir = "worktree"
prompt  = { via = "argv" }

[env]
allow = ["HOME", "PATH"]

[auth]
method = "oauth_cli"

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id              = "modelo-a"
route_model     = "Modelo A"
roles           = ["implementation"]
max_sensitivity = "internal"

[[models]]
id              = "modelo-b"
route_model     = "Modelo B"
roles           = ["implementation"]
max_sensitivity = "internal"

[[models]]
id              = "modelo-c"
route_model     = "Modelo C"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {token}"
expect = "token_echo"
"#;

/// Dos modelos con un comentario suelto ENTRE los dos bloques, que
/// estructuralmente pertenece al primero (el algoritmo de límites se detiene
/// en la siguiente cabecera de tabla, no antes). Sirve para probar que
/// quitar el modelo que NO lo precede —`modelo-dos`— no lo pierde ni lo
/// confunde con una cabecera.
const DOS_MODELOS_CON_COMENTARIO_ENTRE: &str = r#"schema_version = 1
id             = "prueba-comentario"
kind           = "cli"

[executable]
program       = "/bin/echo"
version_pin   = "coreutils"
version_probe = ["--version"]
resolve       = ["/bin/echo"]

[invoke]
argv    = ["{prompt}"]
workdir = "worktree"
prompt  = { via = "argv" }

[env]
allow = ["HOME", "PATH"]

[auth]
method = "oauth_cli"

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id              = "modelo-uno"
route_model     = "Modelo Uno"
roles           = ["implementation"]
max_sensitivity = "internal"

# nota suelta entre los dos modelos, no pertenece a ninguno en particular
[[models]]
id              = "modelo-dos"
route_model     = "Modelo Dos"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {token}"
expect = "token_echo"
"#;

/// `quitar_modelo` sobre un texto con 2+ modelos: el restante sigue
/// parseando con sólo el que no se quitó, y el bloque devuelto lleva el id y
/// el `route_model` correctos.
#[test]
fn quitar_modelo_deja_intacto_el_restante_y_devuelve_el_bloque_correcto() {
    let (restante, bloque) =
        quitar_modelo(DOS_MODELOS, "modelo-uno").expect("modelo-uno está en el texto de prueba");

    assert!(
        bloque.contains("id              = \"modelo-uno\""),
        "{bloque}"
    );
    assert!(
        bloque.contains("route_model     = \"Modelo Uno\""),
        "{bloque}"
    );

    let manifiesto = ProviderManifest::parse(&restante, Path::new("prueba.toml"))
        .expect("el texto restante tiene que seguir parseando");
    let ids: Vec<&str> = manifiesto
        .models()
        .iter()
        .map(|m| m.id().as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["modelo-dos"],
        "el modelo restante tiene que seguir intacto"
    );
}

/// Un id que no aparece en el texto: `None`, no un error a medias.
#[test]
fn quitar_modelo_sobre_un_id_ausente_da_none() {
    assert!(quitar_modelo(DOS_MODELOS, "no-existe-en-este-texto").is_none());
}

/// El criterio de límites de bloque: quitar el primero, el del medio y el
/// último de un fixture con 3+ modelos y blanco de separación entre todos
/// no deja líneas en blanco dobles.
#[test]
fn quitar_modelo_no_deja_lineas_en_blanco_dobles_sea_primero_medio_o_ultimo() {
    for id in ["modelo-a", "modelo-b", "modelo-c"] {
        let (restante, _bloque) =
            quitar_modelo(TRES_MODELOS, id).unwrap_or_else(|| panic!("{id} está en el fixture"));
        assert!(
            !restante.contains("\n\n\n"),
            "quitar {id} dejó una línea en blanco doble:\n{restante}"
        );
    }
}

/// Un comentario `#...` entre dos bloques `[[models]]` no confunde el
/// escaneo de límites (no se trata como cabecera) ni se pierde al quitar el
/// modelo que no lo precede.
#[test]
fn un_comentario_entre_dos_bloques_no_confunde_el_escaneo_ni_se_pierde() {
    let (restante, bloque) = quitar_modelo(DOS_MODELOS_CON_COMENTARIO_ENTRE, "modelo-dos")
        .expect("modelo-dos está en el texto de prueba");

    assert!(bloque.contains("modelo-dos"), "{bloque}");
    assert!(
        restante.contains("# nota suelta entre los dos modelos"),
        "el comentario no debe perderse al quitar el modelo que no lo precede:\n{restante}"
    );
    assert!(
        restante.contains("id              = \"modelo-uno\""),
        "modelo-uno sigue intacto:\n{restante}"
    );
}

/// El test literal del checklist de T6: quitar un modelo, tomar el bloque
/// impreso (conceptualmente: volver a añadirlo con `anexar_modelo`, la
/// propia función de la orden `nuevo-modelo`), y comprobar que el CONJUNTO
/// de ids de modelo es igual al original. No se exige igualdad byte a byte
/// del fichero completo -el modelo legítimamente reaparece al final, no en
/// su posición original- sino que "carga igual", tal como dice el checklist.
#[test]
fn quitar_y_repegar_carga_con_el_mismo_conjunto_de_modelos() {
    let original = ProviderManifest::parse(DOS_MODELOS, Path::new("prueba.toml"))
        .expect("el fixture de partida tiene que parsear");
    let mut ids_originales: Vec<String> = original
        .models()
        .iter()
        .map(|m| m.id().as_str().to_string())
        .collect();
    ids_originales.sort();

    let (restante, _bloque) =
        quitar_modelo(DOS_MODELOS, "modelo-uno").expect("modelo-uno está en el texto de prueba");
    let repegado = anexar_modelo(&restante, "modelo-uno", "Modelo Uno");

    let final_manifiesto = ProviderManifest::parse(&repegado, Path::new("prueba.toml"))
        .expect("el texto quitado-y-repegado tiene que parsear");
    let mut ids_finales: Vec<String> = final_manifiesto
        .models()
        .iter()
        .map(|m| m.id().as_str().to_string())
        .collect();
    ids_finales.sort();

    assert_eq!(
        ids_originales, ids_finales,
        "quitar y repegar tiene que cargar con el mismo conjunto de modelos"
    );
}

// === Funciones de I/O: sobre copias de fixtures en un directorio temporal ===

/// `nuevo-proveedor` sobre un id nuevo escribe el fichero y el directorio
/// entero sigue cargando con `ProviderManifest::load_dir` después —el
/// ejecutable de la plantilla (`/bin/echo`) resuelve de verdad, así que
/// `load` (que también comprueba el ejecutable) no falla.
#[test]
fn nuevo_proveedor_escribe_el_fichero_y_carga_con_load_dir() {
    let providers_dir = proveedores_temporales("nuevo-proveedor-ok", &["eco.toml"]);

    let destino = nuevo_proveedor(&providers_dir, "mi-proveedor").expect("se crea el proveedor");
    assert_eq!(destino, providers_dir.join("mi-proveedor.toml"));
    assert!(destino.exists());

    let manifiestos = ProviderManifest::load_dir(&providers_dir).expect("el directorio carga");
    let creado = manifiestos
        .iter()
        .find(|m| m.id().as_str() == "mi-proveedor")
        .expect("el proveedor nuevo aparece en el directorio");
    assert_eq!(creado.models().len(), 1, "la plantilla trae un modelo");
}

/// `nuevo-proveedor` sobre un id que ya tiene fichero falla y no lo toca.
#[test]
fn nuevo_proveedor_sobre_un_id_existente_falla_y_no_sobrescribe() {
    let providers_dir = proveedores_temporales("nuevo-proveedor-duplicado", &["eco.toml"]);
    let ruta = providers_dir.join("eco.toml");
    let antes = fs::read(&ruta).expect("se lee el fichero existente");

    let error = nuevo_proveedor(&providers_dir, "eco").expect_err("eco ya tiene fichero");
    assert!(error.to_string().contains("eco"), "{error}");

    let despues = fs::read(&ruta).expect("se relee");
    assert_eq!(
        antes, despues,
        "nuevo-proveedor no puede tocar un fichero que ya existía"
    );
}

/// `nuevo-modelo` añade un modelo y el manifiesto conserva los anteriores,
/// con el nuevo al final.
#[test]
fn nuevo_modelo_anade_y_conserva_los_anteriores() {
    let providers_dir = proveedores_temporales("nuevo-modelo-ok", &["dos_modelos.toml"]);

    nuevo_modelo(&providers_dir, "dos-modelos", "eco-nuevo", "Eco Nuevo").expect("se añade");

    let manifiestos = ProviderManifest::load_dir(&providers_dir).expect("el directorio carga");
    let manifiesto = manifiestos
        .iter()
        .find(|m| m.id().as_str() == "dos-modelos")
        .expect("dos-modelos sigue estando");
    let ids: Vec<&str> = manifiesto
        .models()
        .iter()
        .map(|m| m.id().as_str())
        .collect();
    assert_eq!(ids, vec!["eco-rapido", "eco-lento", "eco-nuevo"]);
}

/// `nuevo-modelo` con un id que el proveedor ya declara falla y no modifica
/// el fichero.
#[test]
fn nuevo_modelo_con_id_duplicado_falla_y_no_modifica_el_fichero() {
    let providers_dir = proveedores_temporales("nuevo-modelo-duplicado", &["dos_modelos.toml"]);
    let ruta = providers_dir.join("dos_modelos.toml");
    let antes = fs::read(&ruta).expect("se lee");

    let error = nuevo_modelo(&providers_dir, "dos-modelos", "eco-rapido", "Otra Ruta")
        .expect_err("eco-rapido ya es de dos-modelos");
    assert!(error.to_string().contains("eco-rapido"), "{error}");

    let despues = fs::read(&ruta).expect("se relee");
    assert_eq!(
        antes, despues,
        "nuevo-modelo no puede tocar el fichero cuando falla"
    );
}

/// `quitar-modelo` sobre un proveedor con 2+ modelos: el fichero resultante
/// carga con un modelo menos, y lo que la orden real imprimiría (el `String`
/// que devuelve la función pública) contiene el bloque quitado.
#[test]
fn quitar_modelo_de_un_proveedor_con_varios_deja_uno_menos_e_imprime_el_bloque() {
    let providers_dir = proveedores_temporales("quitar-modelo-ok", &["dos_modelos.toml"]);

    let bloque =
        quitar_modelo_de(&providers_dir, "dos-modelos/eco-rapido").expect("se quita el modelo");
    assert!(bloque.contains("eco-rapido"), "{bloque}");

    let manifiestos = ProviderManifest::load_dir(&providers_dir).expect("el directorio carga");
    let manifiesto = manifiestos
        .iter()
        .find(|m| m.id().as_str() == "dos-modelos")
        .expect("dos-modelos sigue estando");
    let ids: Vec<&str> = manifiesto
        .models()
        .iter()
        .map(|m| m.id().as_str())
        .collect();
    assert_eq!(ids, vec!["eco-lento"]);
}

/// `quitar-modelo` sobre el único modelo de un proveedor falla con el error
/// dedicado y no toca el fichero.
#[test]
fn quitar_el_unico_modelo_de_un_proveedor_falla_con_el_error_dedicado_y_no_toca_nada() {
    let providers_dir = proveedores_temporales("quitar-modelo-unico", &["eco.toml"]);
    let ruta = providers_dir.join("eco.toml");
    let antes = fs::read(&ruta).expect("se lee");

    let error =
        quitar_modelo_de(&providers_dir, "eco/eco-modelo").expect_err("eco sólo tiene un modelo");
    assert!(error.to_string().contains("disable"), "{error}");

    let despues = fs::read(&ruta).expect("se relee");
    assert_eq!(
        antes, despues,
        "no se debió tocar el fichero al rechazar quitar el único modelo"
    );
}
