//! Lo que `batuta-exec` necesita leer de un manifiesto.
//!
//! Existe porque no existía. `ProviderManifest` tenía accesores para lo que
//! pedían los tests de carga —identificador, procedencia, ficheros de corrida,
//! modelos— y **ninguno para `invoke`, `env`, `canary` ni el parser**, que es
//! justo lo que hace falta para ejecutar. Escribí la superficie mirando las
//! pruebas, y las pruebas no miraban ahí.
//!
//! Es la misma regla que ya se cobró dos huecos en esta fase: **lo que no está en
//! un test no está en el contrato**. Aquí produjo una API ausente en vez de una
//! equivocada, que es la variante silenciosa.
//!
//! Todo se comprueba contra los **dos manifiestos reales** del repositorio. Un
//! manifiesto inventado para la ocasión probaría que los accesores compilan; sólo
//! los de verdad prueban que sirven para lo que se escribieron.

use std::path::{Path, PathBuf};

use batuta_contract::{CanaryExpectation, EnvVarName, ParserKind, PromptDelivery};
use batuta_manifest::ProviderManifest;

/// Un manifiesto real del repositorio, **interpretado sin preguntar a la máquina**.
///
/// `parse` y no `load`: `load` exige que el binario del proveedor exista aquí, lo
/// cual es correcto en la máquina que va a delegar y falso en la que sólo
/// compila. Ver la nota larga en `carga.rs`.
fn cargar(nombre: &str) -> ProviderManifest {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../providers")
        .join(nombre);
    let texto = std::fs::read_to_string(&ruta).unwrap_or_else(|e| panic!("{nombre}: {e}"));
    ProviderManifest::parse(&texto, &ruta).unwrap_or_else(|e| panic!("{nombre}: {e}"))
}

#[test]
fn el_argv_de_dsh_se_puede_leer_entero() {
    let dsh = cargar("dsh.toml");
    let argv = dsh.invoke().argv();

    assert_eq!(argv[0], "--profile");
    assert_eq!(argv[1], "headless");
    assert!(
        argv.iter().any(|a| a.contains("{run_dir}")),
        "el parche de composición viaja en argv: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "{prompt}"),
        "el prompt tiene que estar en argv: {argv:?}"
    );
}

/// Medido: dsh rechaza el prompt por entrada estándar y no tiene bandera de
/// fichero. El techo de sensibilidad de `argv` lo fija el contrato.
#[test]
fn dsh_recibe_el_prompt_por_argv_y_trabaja_en_el_worktree() {
    let dsh = cargar("dsh.toml");
    assert_eq!(dsh.invoke().prompt_via(), PromptDelivery::Argv);
    assert_eq!(dsh.invoke().workdir(), "worktree");
}

/// R5: nada se hereda sin nombrarlo, y lo que se deniega se deniega por algo.
#[test]
fn la_allowlist_de_entorno_de_dsh_es_explicita_y_deniega_la_telemetria() {
    let dsh = cargar("dsh.toml");
    let permitidas: Vec<&str> = dsh.env().allow().iter().map(EnvVarName::as_str).collect();
    let denegadas: Vec<&str> = dsh.env().deny().iter().map(EnvVarName::as_str).collect();

    for necesaria in ["HOME", "PATH"] {
        assert!(permitidas.contains(&necesaria), "falta {necesaria}");
    }
    // La composición base lee DSH_PERMISSION_MODE para decidir el modo de
    // sandbox. Una variable heredada que moviera la contención es el fallo que
    // R5 paga.
    for prohibida in [
        "DSH_PERMISSION_MODE",
        "DSH_TELEMETRY_MODE",
        "DSH_TELEMETRY_OTLP_URL",
    ] {
        assert!(denegadas.contains(&prohibida), "falta denegar {prohibida}");
    }
}

/// **Guardia de seguridad, no de estilo.** La ayuda de `abacusai` 2.6.11 ofrece
/// `--dangerously-skip-permissions`. batuta contiene por nombre; una bandera que
/// apaga la comprobación entera es lo contrario de contener, y el día que alguien
/// la añada «para desbloquear una corrida», este test lo para.
#[test]
fn ningun_manifiesto_emite_banderas_que_apaguen_la_contencion() {
    for nombre in ["dsh.toml", "abacus.toml"] {
        let manifiesto = cargar(nombre);
        for argumento in manifiesto.invoke().argv() {
            for prohibida in [
                "--dangerously-skip-permissions",
                "--yolo",
                "--no-sandbox",
                "--auto-accept-edits",
            ] {
                assert!(
                    argumento != prohibida,
                    "{nombre} emite {prohibida}, que apaga la contención"
                );
            }
        }
    }
}

#[test]
fn abacus_pide_la_contencion_que_su_cli_ofrece() {
    let abacus = cargar("abacus.toml");
    let argv = abacus.invoke().argv();

    assert!(argv.iter().any(|a| a == "--disallowed-tools"), "{argv:?}");
    assert!(argv.iter().any(|a| a == "*"), "{argv:?}");
    assert!(argv.iter().any(|a| a == "--no-agents-md"), "{argv:?}");

    let fijadas: Vec<&str> = abacus.env().set().iter().map(|(k, _)| k.as_str()).collect();
    assert!(fijadas.contains(&"ABACUSAI_NO_TELEMETRY"), "{fijadas:?}");
}

/// El canario compara con **su** token. Que el prompt lo lleve es lo que hace
/// posible la comparación observacional en vez del juicio por subcadena (R3).
#[test]
fn el_canario_de_los_dos_lleva_su_token_y_espera_el_eco() {
    for nombre in ["dsh.toml", "abacus.toml"] {
        let manifiesto = cargar(nombre);
        assert!(
            manifiesto.canary().prompt().contains("{token}"),
            "{nombre}: el prompt del canario no lleva token"
        );
        assert_eq!(manifiesto.canary().expect(), CanaryExpectation::TokenEcho);
    }
}

/// R11: el pin no basta sin hash cuando el binario se autoactualiza.
#[test]
fn los_dos_fijan_su_binario_por_version_y_por_hash() {
    for nombre in ["dsh.toml", "abacus.toml"] {
        let manifiesto = cargar(nombre);
        let ejecutable = manifiesto.executable();

        assert!(!ejecutable.version_pin().is_empty(), "{nombre} sin pin");
        let hash = ejecutable
            .sha256()
            .unwrap_or_else(|| panic!("{nombre}: R11 exige hash, no sólo versión"));
        assert_eq!(hash.len(), 64, "{nombre}: sha256 mal formado");
        assert!(!ejecutable.resolve().is_empty(), "{nombre} sin resolve");
        assert_ne!(ejecutable.program(), PathBuf::new());
    }
}

#[test]
fn el_parser_de_los_dos_es_texto_plano() {
    for nombre in ["dsh.toml", "abacus.toml"] {
        assert_eq!(cargar(nombre).parser(), ParserKind::PlainText, "{nombre}");
    }
}

/// El recibo lleva **qué manifiesto** gobernó la corrida y **con qué bytes**.
///
/// Sin la segunda mitad, editar un manifiesto invalida en silencio todos los
/// recibos anteriores sin que ninguno se entere: dirían `dsh.toml` y nadie podría
/// saber cuál. El resumen es del texto de origen, no del binario del proveedor —
/// son dos cosas distintas y el recibo lleva las dos.
#[test]
fn un_manifiesto_dice_de_donde_viene_y_con_que_bytes() {
    for nombre in ["dsh.toml", "abacus.toml"] {
        let manifiesto = cargar(nombre);

        assert!(
            manifiesto.origin().ends_with(nombre),
            "{nombre}: origen {:?}",
            manifiesto.origin()
        );
        assert_eq!(
            manifiesto.source_sha256().len(),
            64,
            "{nombre}: resumen mal formado"
        );
        assert!(
            manifiesto
                .source_sha256()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{nombre}: el resumen se escribe en hexadecimal minúsculo"
        );
    }

    // Dos manifiestos distintos no comparten resumen, o el resumen no distingue
    // nada.
    assert_ne!(
        cargar("dsh.toml").source_sha256(),
        cargar("abacus.toml").source_sha256()
    );
}

/// El manifiesto discrepante **carga bien**, y eso es lo que se prueba.
///
/// Su error no es de forma: es que su documento de settings fija un modelo y su
/// `[[models]]` pide otro. Ninguna validación estática puede verlo —el documento
/// de settings es texto opaco para batuta, que es justo por lo que hizo falta
/// leer la procedencia—, así que tiene que pasar la carga y morir en el recibo.
///
/// Si algún día falla al cargar, el criterio 2 de la Fase 3 se queda sin la
/// única prueba de que el recibo no miente.
#[test]
fn el_manifiesto_discrepante_carga_sin_quejarse_porque_su_error_no_es_de_forma() {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pruebas/discrepante/dsh.toml");
    let texto = std::fs::read_to_string(&ruta).expect("se lee");
    let manifiesto =
        ProviderManifest::parse(&texto, &ruta).expect("tiene que cargar: su error no es de forma");

    assert_eq!(manifiesto.models().len(), 1);
    assert_eq!(
        manifiesto.models()[0].route_model().as_str(),
        "deepseek-v4-flash-que-nadie-corrio",
        "lo que batuta cree pedir"
    );

    // Y su documento de settings fija otro —el real—, que es toda la trampa: la
    // corrida sale con éxito y el registro nombra un modelo distinto del pedido.
    let settings = manifiesto
        .runtime_files()
        .iter()
        .find(|f| f.path().ends_with("settings.yaml"))
        .expect("el documento de settings");
    let texto = format!("{:?}", settings.document());
    assert!(
        texto.contains("deepseek-v4-flash") && !texto.contains("que-nadie-corrio"),
        "el documento tiene que fijar el modelo que de verdad corre: {texto}"
    );
}
