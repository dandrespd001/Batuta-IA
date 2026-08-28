//! Los siete criterios de aceptación de la Fase 2, uno por prueba.
//!
//! Cada uno nombra el fallo medido que lo paga. Ninguno comprueba la
//! implementación: comprueban lo que el manifiesto promete a quien lo lee.

use std::path::{Path, PathBuf};

use batuta_manifest::{ManifestError, ProviderManifest};

/// El directorio `providers/` del propio repositorio.
fn providers() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../providers")
}

/// Un manifiesto válido y mínimo, para mutarlo en cada prueba.
fn base() -> String {
    r#"
schema_version = 1
id             = "prueba"
kind           = "cli"

[executable]
program       = "/bin/echo"
version_pin   = "1.0"
version_probe = ["--version"]
resolve       = ["/bin/echo"]

[auth]
method = "oauth_cli"

[invoke]
argv    = ["--model", "{route_model}", "{prompt}"]
workdir = "worktree"
prompt  = { via = "argv" }

[env]
allow = ["HOME", "PATH"]

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id              = "prueba-modelo"
route_model     = "modelo-remoto"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {token}"
expect = "token_echo"
"#
    .to_string()
}

fn error_de(fuente: &str) -> ManifestError {
    ProviderManifest::parse(fuente, Path::new("prueba.toml"))
        .expect_err("este manifiesto no debería cargar")
}

/// El manifiesto base tiene que cargar, o el resto de pruebas no prueban nada.
#[test]
fn el_manifiesto_base_carga() {
    ProviderManifest::parse(&base(), Path::new("prueba.toml")).expect("el base es válido");
}

/// Criterio 1 — y la tesis entera del proyecto: dar de alta un proveedor es un
/// fichero. Si estos dos cargan, Abacus funciona sin haber parcheado el núcleo.
#[test]
fn los_dos_manifiestos_del_repositorio_cargan() {
    let cargados = ProviderManifest::load_dir(&providers()).expect("providers/ debe cargar");

    let ids: Vec<String> = cargados.iter().map(|m| m.id().to_string()).collect();
    assert_eq!(
        ids,
        vec!["abacus".to_string(), "dsh".to_string()],
        "{ids:?}"
    );
}

/// Y cargan con las diferencias que los hacen interesantes: dsh materializa dos
/// ficheros por corrida y abacus **ninguno**. Si el campo sólo lo usara quien lo
/// inspiró, sería un parche disfrazado de campo genérico.
#[test]
fn abacus_no_necesita_ficheros_de_corrida_y_dsh_necesita_dos() {
    let cargados = ProviderManifest::load_dir(&providers()).expect("providers/ debe cargar");

    let abacus = cargados
        .iter()
        .find(|m| m.id().as_str() == "abacus")
        .unwrap();
    let dsh = cargados.iter().find(|m| m.id().as_str() == "dsh").unwrap();

    assert_eq!(abacus.runtime_files().len(), 0);
    assert_eq!(dsh.runtime_files().len(), 2);
}

/// La procedencia no es decorativa: dice si el recibo puede comprobarse o sólo
/// creerse. Se pidió `deepseek-v4-flash` tres veces y corrió el modelo de otro las tres.
#[test]
fn la_procedencia_distingue_lo_observado_de_lo_prometido() {
    use batuta_contract::ProvenanceSource;

    let cargados = ProviderManifest::load_dir(&providers()).expect("providers/ debe cargar");
    let abacus = cargados
        .iter()
        .find(|m| m.id().as_str() == "abacus")
        .unwrap();
    let dsh = cargados.iter().find(|m| m.id().as_str() == "dsh").unwrap();

    assert_eq!(dsh.provenance(), ProvenanceSource::SessionLog);
    assert_eq!(abacus.provenance(), ProvenanceSource::Declared);
}

/// Criterio 2 (R1) — el mensaje tiene que ser accionable: fichero y línea.
#[test]
fn un_valor_fuera_de_vocabulario_falla_nombrando_fichero_linea_y_validos() {
    let roto = base().replace(r#"parser = "plain_text""#, r#"parser = "magia""#);
    let error = error_de(&roto);

    let sitio = error
        .location()
        .expect("un error de valor sabe dónde ocurrió");
    assert_eq!(sitio.file, Path::new("prueba.toml"));
    assert!(sitio.line > 0, "línea sin fijar");

    let mensaje = error.to_string();
    assert!(mensaje.contains("parser"), "{mensaje}");
    assert!(mensaje.contains("magia"), "{mensaje}");
    // R8: el error lista los valores válidos, todos.
    for valido in ["plain_text", "jsonl_last_text", "json_pointer", "plugin"] {
        assert!(mensaje.contains(valido), "falta {valido} en: {mensaje}");
    }
}

/// Criterio 3 — un fichero de corrida no es material del encargo y no puede
/// aparecer en el `git diff` que batuta calcula sobre el worktree.
#[test]
fn una_ruta_absoluta_en_un_fichero_de_corrida_no_se_admite() {
    let roto = format!(
        "{}\n[[runtime_files]]\npath = \"/etc/passwd\"\nformat = \"yaml\"\n[runtime_files.content]\nclave = \"valor\"\n",
        base()
    );
    let error = error_de(&roto);
    assert!(
        matches!(error, ManifestError::RuntimeFilePathAbsolute { .. }),
        "{error:?}"
    );
}

#[test]
fn una_ruta_que_se_sale_del_directorio_de_corrida_no_se_admite() {
    let roto = format!(
        "{}\n[[runtime_files]]\npath = \"../../fuera.yml\"\nformat = \"yaml\"\n[runtime_files.content]\nclave = \"valor\"\n",
        base()
    );
    let error = error_de(&roto);
    assert!(
        matches!(error, ManifestError::RuntimeFilePathEscapes { .. }),
        "{error:?}"
    );
}

/// Criterio 6 — un documento es lista o mapa. Batuta no adivina por la pinta.
#[test]
fn declarar_lista_y_mapa_a_la_vez_no_se_admite() {
    let roto = format!(
        "{}\n[[runtime_files]]\npath = \"doc.yml\"\nformat = \"yaml\"\n[[runtime_files.entry]]\nid = \"algo\"\n[runtime_files.content]\nclave = \"valor\"\n",
        base()
    );
    let error = error_de(&roto);
    assert!(
        matches!(error, ManifestError::DocumentShapeAmbiguous { .. }),
        "{error:?}"
    );
}

#[test]
fn no_declarar_ni_lista_ni_mapa_tampoco() {
    let roto = format!(
        "{}\n[[runtime_files]]\npath = \"doc.yml\"\nformat = \"yaml\"\n",
        base()
    );
    let error = error_de(&roto);
    assert!(
        matches!(error, ManifestError::DocumentShapeMissing { .. }),
        "{error:?}"
    );
}

/// Criterio 4 (R8) — una llave inventada lista las admitidas, incorporadas y
/// declaradas.
#[test]
fn una_llave_desconocida_lista_todas_las_admitidas() {
    let roto = base().replace("{route_model}", "{modelo_inventado}");
    let error = error_de(&roto);

    let mensaje = error.to_string();
    assert!(mensaje.contains("modelo_inventado"), "{mensaje}");
    for admitida in batuta_manifest::BUILTIN_PLACEHOLDERS {
        assert!(mensaje.contains(admitida), "falta {admitida} en: {mensaje}");
    }
}

/// Criterio 5 — el invariante que hace que añadir un `write_mode` rompa en voz
/// alta en vez de elegir en silencio.
#[test]
fn un_mapa_de_sustitucion_incompleto_nombra_la_variante_que_falta() {
    let roto = format!(
        "{}\n[substitutions.sandbox_mode]\nread_only = \"read-only\"\nvalidated_patch = \"workspace-write\"\n",
        base()
    );
    let error = error_de(&roto);

    match &error {
        ManifestError::SubstitutionIncomplete {
            key,
            vocabulary,
            missing,
            ..
        } => {
            assert_eq!(key, "sandbox_mode");
            assert_eq!(*vocabulary, "write_mode");
            assert_eq!(missing, &["validated_apply"]);
        }
        otro => panic!("se esperaba SubstitutionIncomplete: {otro:?}"),
    }
    assert!(error.to_string().contains("validated_apply"));
}

/// Una llave declarada por `[substitutions]` deja de ser desconocida.
#[test]
fn una_llave_declarada_se_admite_donde_la_incorporada_no_llegaba() {
    let bueno = format!(
        "{}\n[substitutions.sandbox_mode]\nread_only = \"read-only\"\nvalidated_patch = \"workspace-write\"\nvalidated_apply = \"workspace-write\"\n\n[[runtime_files]]\npath = \"doc.yml\"\nformat = \"yaml\"\n[runtime_files.content]\nmode = \"{{sandbox_mode}}\"\n",
        base()
    );
    let cargado = ProviderManifest::parse(&bueno, Path::new("prueba.toml"))
        .expect("una llave declarada es una llave válida");

    assert_eq!(
        cargado.substitutions().declared_keys(),
        vec!["sandbox_mode"]
    );
}

/// Criterio 2, la mitad que sólo se sabe mirando la máquina. **Es el fallo que
/// originó batuta**: un transporte declarado sin ejecutor, que moría después de
/// pagar la corrida.
#[test]
fn un_ejecutable_que_no_existe_falla_al_cargar_y_dice_donde_buscó() {
    let roto = base()
        .replace(
            r#"program       = "/bin/echo""#,
            r#"program       = "/no/existe/jamas""#,
        )
        .replace(
            r#"resolve       = ["/bin/echo"]"#,
            r#"resolve       = ["/no/existe/jamas"]"#,
        );

    let manifiesto =
        ProviderManifest::parse(&roto, Path::new("prueba.toml")).expect("la forma es válida");

    let error = manifiesto
        .verify_executable()
        .expect_err("un ejecutor irresoluble no puede pasar la carga");

    match &error {
        ManifestError::ExecutableNotFound { program, tried, .. } => {
            assert_eq!(program, Path::new("/no/existe/jamas"));
            assert!(!tried.is_empty(), "el error debe decir dónde buscó");
        }
        otro => panic!("se esperaba ExecutableNotFound: {otro:?}"),
    }
}

/// Un manifiesto sin modelos no enruta a ninguna parte, y decirlo al cargar es
/// más barato que descubrirlo al enrutar.
#[test]
fn un_manifiesto_sin_modelos_no_carga() {
    let roto = base().split("[[models]]").next().unwrap().to_string()
        + "\n[canary]\nprompt = \"Responde exactamente con: {token}\"\nexpect = \"token_echo\"\n";

    let error = error_de(&roto);
    assert!(matches!(error, ManifestError::NoModels { .. }), "{error:?}");
}

/// La doctrina del propio proyecto, aplicada al manifiesto.
///
/// `TaskSpecDraft` lleva `deny_unknown_fields` a proposito: convierte el acuerdo
/// sobre los campos en un fallo de carga en vez de un campo ignorado en
/// silencio. Un manifiesto no puede ser mas laxo que un encargo: `version_pinn`
/// en vez de `version_pin` dejaria el pin sin efecto y R11 sin red, sin que
/// nadie se entere hasta que el binario cambie por debajo.
#[test]
fn un_campo_desconocido_falla_al_cargar_y_lo_nombra() {
    let roto = base().replace(
        "version_pin   = \"1.0\"",
        "version_pin   = \"1.0\"\nversion_pinn  = \"1.0\"",
    );
    let error = error_de(&roto);
    let mensaje = error.to_string();
    assert!(mensaje.contains("version_pinn"), "{mensaje}");
}

/// Una version de esquema que batuta no sabe leer no es "TOML mal formado".
///
/// Mapearla a `Syntax` decia una cosa por otra a quien leyera el mensaje, que es
/// justo lo que R8 y R1 quieren evitar: el error tiene que nombrar el problema
/// real y lo que se admitia.
#[test]
fn una_version_de_esquema_no_soportada_tiene_su_propio_error() {
    let roto = base().replace("schema_version = 1", "schema_version = 999");
    let error = error_de(&roto);

    assert!(
        matches!(error, ManifestError::UnsupportedSchemaVersion { .. }),
        "{error:?}"
    );
    let mensaje = error.to_string();
    assert!(mensaje.contains("999"), "{mensaje}");
    assert!(error.location().is_some(), "tiene que decir donde");
}

/// Una ambigüedad no se resuelve eligiendo ganador: se rechaza.
///
/// Al implementar `build_env` apareció la pregunta de si `deny` gana sobre `set`
/// cuando los dos nombran la misma variable. Cualquiera de las dos respuestas
/// deja un manifiesto que dice dos cosas contrarias y una de ellas no se cumple,
/// en silencio. Es el mismo criterio que R1 aplica al ejecutor: incoherente
/// falla al **cargar**, no en la corrida.
#[test]
fn fijar_y_denegar_la_misma_variable_no_se_admite() {
    let roto = base().replace(
        "allow = [\"HOME\", \"PATH\"]",
        "allow = [\"HOME\", \"PATH\"]\ndeny  = [\"BATUTA_CONFLICTO\"]\nset   = { BATUTA_CONFLICTO = \"algo\" }",
    );
    let error = error_de(&roto);

    assert!(
        matches!(error, ManifestError::ConflictingEnvVar { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains("BATUTA_CONFLICTO"), "{error}");
}
