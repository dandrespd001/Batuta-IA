//! `nuevo-proveedor`, `nuevo-modelo`, `quitar-modelo`: la capa de Declaración,
//! editada como texto (T6, `docs/FASE5_PANEL.md`).
//!
//! §1 del brief separa tres capas —Declaración, Evidencia, Elección— y dice
//! que la Declaración (`providers/*.toml`) la escribe **una persona, a mano**.
//! Estas tres órdenes no lo contradicen: siguen editando el fichero como texto,
//! nunca deserializando-con-`toml`-y-reserializando, porque eso destruiría los
//! comentarios, y en este repositorio los comentarios llevan mediciones reales
//! (`USER` en el `allow` de abacus, la lista de variantes que dio el propio
//! proveedor). Un panel que reescribiera el TOML borraría lo que hace que el
//! manifiesto valga.
//!
//! Por eso el módulo separa dos capas internas:
//!
//! - **Funciones puras de texto** (`plantilla_proveedor`, `anexar_modelo`,
//!   `quitar_modelo`): sin I/O, se prueban directamente sobre `&str`.
//! - **Funciones de I/O** (`nuevo_proveedor`, `nuevo_modelo`,
//!   `quitar_modelo_de`): cargan el directorio de manifiestos, validan, y sólo
//!   entonces tocan el disco.

use std::path::{Path, PathBuf};

use batuta_contract::{ModelId, ProviderId, RouteModel};
use batuta_manifest::ProviderManifest;

use crate::error::CliError;

/// Plantilla comentada de un proveedor nuevo.
///
/// Usa `/bin/echo` como ejecutable de partida: es el mismo truco que
/// `tests/fixtures/eco.toml` — un binario que existe en cualquier máquina
/// Unix y que hace que `ProviderManifest::load` (no sólo `parse`) resuelva sin
/// que quien la usa tenga que tocar nada todavía. El hueco evidente no es el
/// ejecutable, es `route_model` del modelo de ejemplo: eso sí es opaco al
/// proveedor y no hay forma honesta de adivinarlo.
///
/// **Garantía:** el texto que devuelve carga tal cual con
/// [`ProviderManifest::parse`] — hay un test que lo comprueba explícitamente
/// (`la_plantilla_de_un_proveedor_nuevo_parsea`), para que un cambio futuro
/// que rompa la plantilla lo revienta en CI y no en producción.
pub fn plantilla_proveedor(id: &str) -> String {
    format!(
        r#"# Proveedor `{id}`, dado de alta con `batuta nuevo-proveedor`.
#
# Rellena lo que sepas y borra este comentario cuando lo hagas. El ejecutable
# apunta a /bin/echo de partida para que el manifiesto cargue sin tocar nada;
# cámbialo por el binario real en cuanto lo tengas medido (R11: versión Y hash).
schema_version = 1
id             = "{id}"
kind           = "cli"

[executable]
program       = "/bin/echo"
version_pin   = "coreutils"
version_probe = ["--version"]
resolve       = ["/bin/echo"]

[auth]
method = "oauth_cli"

[invoke]
argv    = ["{{prompt}}"]
workdir = "worktree"
prompt  = {{ via = "argv" }}

[env]
allow = ["HOME", "PATH"]

[response]
parser = "plain_text"

[provenance]
source = "declared"

# Primer modelo, de ejemplo. `route_model` es opaco al proveedor: es el
# nombre que él entiende, no el que batuta usa; no hay forma de adivinarlo,
# así que queda con un TODO literal en vez de un valor plausible que se pueda
# olvidar de cambiar.
[[models]]
id              = "modelo-nuevo"
route_model     = "TODO-nombre-en-el-proveedor"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {{token}}"
expect = "token_echo"
"#
    )
}

/// Añade un bloque `[[models]]` al final del texto — fin de fichero literal,
/// no «antes de `[canary]`»: TOML no exige orden de secciones, así que sigue
/// siendo válido, y la implementación es mucho más simple y segura que
/// insertar en medio.
///
/// **Garantía por construcción:** el texto devuelto SIEMPRE empieza con los
/// bytes exactos de `texto`, sin modificarlos — sólo se usa `push`/`push_str`
/// sobre una copia, nunca se reescribe nada anterior. «Los comentarios
/// previos sobreviven byte a byte» es así una propiedad estructural del
/// código, no algo que un test tenga que verificar con difs.
///
/// `roles` y `max_sensitivity` quedan con los valores por defecto de este
/// repositorio (`["implementation"]` / `"internal"`), con un comentario que
/// invita a revisarlos: la orden no tiene de dónde tomarlos, y elegir un
/// techo de sensibilidad en silencio sería peor que dejarlo escrito.
pub fn anexar_modelo(texto: &str, id: &str, route_model: &str) -> String {
    use std::fmt::Write as _;

    let mut salida = String::from(texto);

    if !salida.is_empty() {
        if !salida.ends_with('\n') {
            salida.push('\n');
        }
        salida.push('\n');
    }

    salida.push_str("[[models]]\n");
    let _ = writeln!(salida, "id              = \"{id}\"");
    let _ = writeln!(salida, "route_model     = \"{route_model}\"");
    salida.push_str("# roles/max_sensitivity puestos por defecto: revísalos si no aplican.\n");
    salida.push_str("roles           = [\"implementation\"]\n");
    salida.push_str("max_sensitivity = \"internal\"\n");

    salida
}

/// ¿Es esta línea una cabecera de tabla?
///
/// Cualquier línea cuya forma recortada (`trim_start`) empiece por `[` EN LA
/// PRIMERA POSICIÓN NO EN BLANCO. Esto excluye las líneas de comentario
/// (empiezan por `#`) y los valores como `roles = ["implementation"]` (no
/// empiezan por `[` tras el `trim_start`, empiezan por la letra de la clave).
fn es_cabecera_de_tabla(linea: &str) -> bool {
    linea.trim_start().starts_with('[')
}

/// El valor de `clave = "valor"` en esta línea, si la línea declara
/// exactamente esa clave (comparada ya recortada de espacios) y su valor es
/// una cadena básica entre comillas dobles. Las líneas de comentario nunca
/// cuentan, aunque su texto contenga la palabra.
fn valor_de_clave<'a>(linea: &'a str, clave: &str) -> Option<&'a str> {
    let recortada = linea.trim_start();
    if recortada.starts_with('#') {
        return None;
    }
    let (izquierda, derecha) = recortada.split_once('=')?;
    if izquierda.trim() != clave {
        return None;
    }
    let derecha = derecha.trim_start().strip_prefix('"')?;
    let fin = derecha.find('"')?;
    Some(&derecha[..fin])
}

/// Localiza el bloque `[[models]]` cuyo `id = "<model_id>"` coincide, lo
/// quita del texto y lo devuelve aparte. `None` si ese id no aparece.
///
/// Algoritmo de límites: el bloque de un `[[models]]` va desde su línea
/// cabecera hasta (sin incluir) la SIGUIENTE línea-cabecera de cualquier
/// tipo, o fin de fichero — incluyendo naturalmente cualquier línea en
/// blanco que lo separe del siguiente, así que quitar el rango completo deja
/// el fichero limpio sin lógica adicional de «tragar la línea en blanco
/// anterior».
///
/// **No decide** si un comentario que precede a un `[[models]]` «pertenece» a
/// ese modelo o es un comentario de sección general: eso es ambiguo y
/// arriesgado (podría borrar documentación de otro modelo). Cualquier
/// comentario precedente se deja intacto, aunque quede huérfano tras la
/// extracción — un desorden menor, no una pérdida de información.
pub fn quitar_modelo(texto: &str, model_id: &str) -> Option<(String, String)> {
    let lineas: Vec<&str> = texto.split_inclusive('\n').collect();

    let mut indice = 0;
    while indice < lineas.len() {
        if !lineas[indice].trim_start().starts_with("[[models]]") {
            indice += 1;
            continue;
        }

        // Fin del bloque: la siguiente cabecera de cualquier tipo, o EOF.
        let mut fin = indice + 1;
        while fin < lineas.len() && !es_cabecera_de_tabla(lineas[fin]) {
            fin += 1;
        }

        let coincide = lineas[indice..fin]
            .iter()
            .find_map(|linea| valor_de_clave(linea, "id"))
            .is_some_and(|id| id == model_id);

        if coincide {
            let bloque: String = lineas[indice..fin].concat();
            let restante: String = lineas[..indice]
                .iter()
                .chain(lineas[fin..].iter())
                .copied()
                .collect();
            return Some((restante, bloque.trim_end().to_string()));
        }

        indice = fin;
    }

    None
}

/// Escribe `providers/<id>.toml` a partir de la plantilla.
///
/// # Errors
///
/// [`CliError::InvalidProviderId`] si `id` no valida como [`ProviderId`];
/// [`CliError::ProviderAlreadyExists`] si el fichero destino ya existe —nunca
/// se sobrescribe un proveedor existente—; y [`CliError::Manifest`] si, por
/// algún fallo de la propia plantilla, el texto no llegara a parsear
/// (comprobado ANTES de escribir: si no parsea, no se escribe nada).
pub fn nuevo_proveedor(providers_dir: &Path, id: &str) -> Result<PathBuf, CliError> {
    let provider_id: ProviderId = id
        .parse()
        .map_err(|source| CliError::InvalidProviderId { source })?;

    let destino = providers_dir.join(format!("{}.toml", provider_id.as_str()));
    if destino.exists() {
        return Err(CliError::ProviderAlreadyExists {
            id: provider_id.into_string(),
            path: destino,
        });
    }

    let texto = plantilla_proveedor(provider_id.as_str());
    ProviderManifest::parse(&texto, &destino).map_err(|source| CliError::Manifest {
        source: Box::new(source),
    })?;

    std::fs::write(&destino, &texto).map_err(|source| CliError::Io {
        path: destino.clone(),
        source,
    })?;

    Ok(destino)
}

/// Añade un modelo a `providers/<provider>.toml`.
///
/// # Errors
///
/// [`CliError::InvalidModelId`] / [`CliError::InvalidRouteModel`] si `id` o
/// `route_model` no validan; los de `crate::command::cargar`/`hallar` si el
/// directorio no carga o el proveedor no existe; [`CliError::DuplicateModelId`]
/// si el proveedor ya declara ese id; y [`CliError::Manifest`] si el texto
/// resultante no llegara a parsear (comprobado antes de escribir).
pub fn nuevo_modelo(
    providers_dir: &Path,
    provider: &str,
    id: &str,
    route_model: &str,
) -> Result<(), CliError> {
    let model_id: ModelId = id
        .parse()
        .map_err(|source| CliError::InvalidModelId { source })?;
    let route_model: RouteModel = route_model
        .parse()
        .map_err(|source| CliError::InvalidRouteModel { source })?;

    let manifiestos = crate::command::cargar(providers_dir)?;
    let manifiesto = crate::command::hallar(&manifiestos, provider)?;

    if manifiesto.models().iter().any(|m| m.id() == &model_id) {
        return Err(CliError::DuplicateModelId {
            provider: manifiesto.id().as_str().to_string(),
            id: model_id.into_string(),
        });
    }

    let origen = manifiesto.origin().to_path_buf();
    let texto_actual = std::fs::read_to_string(&origen).map_err(|source| CliError::Io {
        path: origen.clone(),
        source,
    })?;

    let texto_nuevo = anexar_modelo(&texto_actual, model_id.as_str(), route_model.as_str());
    ProviderManifest::parse(&texto_nuevo, &origen).map_err(|source| CliError::Manifest {
        source: Box::new(source),
    })?;

    std::fs::write(&origen, &texto_nuevo).map_err(|source| CliError::Io {
        path: origen,
        source,
    })
}

/// Quita un modelo dado `<proveedor>/<modelo>` e imprime lo que borró.
///
/// # Errors
///
/// Los de [`crate::eleccion::resolver`] (proveedor o modelo desconocido,
/// referencia malformada); [`CliError::CannotRemoveLastModel`] si el
/// proveedor sólo declara ese modelo —comprobado ANTES de tocar el texto,
/// para no dejar que lo detecte un fallo de esquema después de escribir—; y
/// [`CliError::Io`] si el disco no coopera.
pub fn quitar_modelo_de(providers_dir: &Path, model_ref: &str) -> Result<String, CliError> {
    let manifiestos = crate::command::cargar(providers_dir)?;
    let (manifiesto, modelo) = crate::eleccion::resolver(&manifiestos, model_ref)?;

    if manifiesto.models().len() == 1 {
        return Err(CliError::CannotRemoveLastModel {
            provider: manifiesto.id().as_str().to_string(),
        });
    }

    let origen = manifiesto.origin().to_path_buf();
    let texto_actual = std::fs::read_to_string(&origen).map_err(|source| CliError::Io {
        path: origen.clone(),
        source,
    })?;

    let Some((texto_restante, bloque)) = quitar_modelo(&texto_actual, modelo.id().as_str()) else {
        return Err(CliError::ModelBlockNotFound {
            provider: manifiesto.id().as_str().to_string(),
            model: modelo.id().as_str().to_string(),
        });
    };

    std::fs::write(&origen, &texto_restante).map_err(|source| CliError::Io {
        path: origen,
        source,
    })?;

    Ok(bloque)
}
