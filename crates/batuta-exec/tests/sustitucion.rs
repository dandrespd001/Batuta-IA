//! Rellenar las llaves del manifiesto, y fallar en voz alta cuando no se puede.

use std::path::{Path, PathBuf};

use batuta_contract::{ReasoningEffort, WriteMode};
use batuta_exec::{ExecError, RunContext, resolve, resolve_argv};
use batuta_manifest::ProviderManifest;

fn dsh() -> ProviderManifest {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../providers/dsh.toml");
    // `parse` y no `load`: `load` exige que el binario de dsh exista aquí, y en
    // una máquina que sólo compila no está. Lo que estas pruebas miran —el argv,
    // los ficheros de corrida, las sustituciones— es esquema, no máquina.
    let texto = std::fs::read_to_string(&ruta).expect("dsh.toml se lee");
    ProviderManifest::parse(&texto, &ruta).expect("dsh.toml debe interpretarse")
}

fn contexto(write_mode: WriteMode) -> RunContext {
    RunContext {
        model: "dsh-deepseek-v4-flash".parse().expect("id válido"),
        route_model: "deepseek-v4-flash".parse().expect("ruta válida"),
        route_provider: Some("deepseek-official".to_string()),
        workdir: PathBuf::from("/tmp/bt/arbol"),
        run_dir: PathBuf::from("/tmp/bt/corrida"),
        prompt: "Responde exactamente con: T-123".to_string(),
        token: "T-123".to_string(),
        write_mode,
        reasoning_effort: None,
    }
}

/// Lo que sale de aquí es el `argv` **real**, que es el que va al recibo. El del
/// manifiesto no sirve para reproducir nada.
#[test]
fn el_argv_sale_sin_ninguna_llave_pendiente() {
    let argv = resolve_argv(&dsh(), &contexto(WriteMode::ValidatedPatch)).expect("sustituye");

    for argumento in &argv {
        assert!(
            !argumento.contains('{') && !argumento.contains('}'),
            "quedó una llave sin sustituir: {argumento}"
        );
    }
    assert!(
        argv.iter().any(|a| a.contains("/tmp/bt/corrida")),
        "{argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "Responde exactamente con: T-123"),
        "el prompt tiene que estar entero: {argv:?}"
    );
}

/// La sustitución declarada es la que traduce el vocabulario de batuta al del
/// proveedor: `validated_patch` es `workspace-write` para dsh.
#[test]
fn una_llave_declarada_cambia_con_el_modo_de_escritura() {
    let manifiesto = dsh();

    let escritura = resolve(
        "{sandbox_mode}",
        "prueba",
        &manifiesto,
        &contexto(WriteMode::ValidatedPatch),
    )
    .expect("declarada");
    let lectura = resolve(
        "{sandbox_mode}",
        "prueba",
        &manifiesto,
        &contexto(WriteMode::ReadOnly),
    )
    .expect("declarada");

    assert_eq!(escritura, "workspace-write");
    assert_eq!(lectura, "read-only");
    assert_ne!(
        escritura, lectura,
        "el modo tiene que cambiar la traducción"
    );
}

/// Ni se deja pasar ni se sustituye por vacío. Una llave que sobrevive acabaría
/// en el `argv` de un proceso real; una sustituida por vacío desaparece sin que
/// nadie lo note, que es peor.
#[test]
fn una_llave_desconocida_falla_listando_las_admitidas() {
    let error = resolve(
        "algo {no_existe_esta_llave} mas",
        "invoke.argv[0]",
        &dsh(),
        &contexto(WriteMode::ValidatedPatch),
    )
    .expect_err("una llave inventada no puede pasar");

    match &error {
        ExecError::UnknownPlaceholder {
            field,
            placeholder,
            expected,
        } => {
            assert_eq!(field, "invoke.argv[0]");
            assert_eq!(placeholder, "no_existe_esta_llave");
            assert!(expected.contains(&"prompt".to_string()), "{expected:?}");
            assert!(
                expected.contains(&"sandbox_mode".to_string()),
                "las declaradas también cuentan: {expected:?}"
            );
        }
        otro => panic!("se esperaba UnknownPlaceholder: {otro:?}"),
    }
    assert!(
        error.to_string().contains("no_existe_esta_llave"),
        "{error}"
    );
}

/// Una plantilla sin llaves sale igual que entró.
#[test]
fn lo_que_no_lleva_llaves_no_se_toca() {
    let texto = "--profile headless --sin-llaves";
    let salida = resolve(
        texto,
        "prueba",
        &dsh(),
        &contexto(WriteMode::ValidatedPatch),
    )
    .expect("sin llaves no hay nada que resolver");
    assert_eq!(salida, texto);
}

/// La regla que el doc-comment de `resolve` ya enunciaba y ningún test fijaba:
/// **nunca se sustituye por vacío**.
///
/// `route_provider` es la única incorporada opcional. Rellenarla con la cadena
/// vacía era lo cómodo, y es la peor de las tres salidas: el vacío viaja hasta el
/// `argv` de un proceso real y nadie lo ve. El modelo al que se delegó el cuerpo
/// eligió el vacío porque no había ni variante de error ni prueba que lo
/// prohibiera —y lo dejó dicho en sus desviaciones—. Ahora lo hay.
#[test]
fn una_incorporada_opcional_sin_valor_para_en_vez_de_vaciarse() {
    let mut contexto = contexto(WriteMode::ValidatedPatch);
    contexto.route_provider = None;

    let error = resolve("{route_provider}", "prueba", &dsh(), &contexto)
        .expect_err("sin ruta declarada no hay nada que poner");

    match &error {
        ExecError::MissingBuiltin { field, placeholder } => {
            assert_eq!(field, "prueba");
            assert_eq!(placeholder, "route_provider");
        }
        otro => panic!("se esperaba MissingBuiltin: {otro:?}"),
    }
}

/// T1 (`docs/FASE5_PANEL.md`) — `{reasoning_effort}` es la segunda incorporada
/// opcional: si el encargo no trae un nivel, falla igual que `route_provider`,
/// no se sustituye por vacío.
#[test]
fn sin_esfuerzo_en_el_encargo_la_llave_de_esfuerzo_falla() {
    let error = resolve(
        "{reasoning_effort}",
        "prueba",
        &dsh(),
        &contexto(WriteMode::ReadOnly),
    )
    .expect_err("el encargo no trae esfuerzo");

    match &error {
        ExecError::MissingBuiltin { field, placeholder } => {
            assert_eq!(field, "prueba");
            assert_eq!(placeholder, "reasoning_effort");
        }
        otro => panic!("se esperaba MissingBuiltin: {otro:?}"),
    }
}

/// T1 — con un nivel en el encargo, la llave se resuelve al nombre que dsh
/// entiende: medido, el adaptador de `DeepSeek` sólo acepta `off`/`low`/`high`/
/// `max`. `xhigh` de batuta colapsa hacia ABAJO, a `high`, nunca al techo: `max`
/// está definido como «todo lo que el proveedor permita», y subir en vez de
/// bajar tiene detrás un fallo real (memoria: leccion-presupuesto-razonamiento).
#[test]
fn con_esfuerzo_en_el_encargo_la_llave_se_resuelve_al_valor_del_proveedor() {
    let contexto = RunContext {
        reasoning_effort: Some(ReasoningEffort::Xhigh),
        ..contexto(WriteMode::ReadOnly)
    };

    let valor = resolve("{reasoning_effort}", "prueba", &dsh(), &contexto).expect("declarado");
    assert_eq!(valor, "high");
}

/// T1 — cierra el hueco que la propia carga no puede cerrar: nada en el
/// manifiesto usa `{reasoning_effort}` todavía, así que `comprobar_placeholders`
/// nunca mira los VALORES de este mapa al cargar (sólo que estén completos). Sin
/// esta prueba, un valor inventado -`xhigh = "maxx"`, por ejemplo- pasaría los 27
/// tests del esquema en verde y sólo fallaría el día que T5 lo dispare en una
/// corrida real, con un `UNSUPPORTED_REASONING_EFFORT` de dsh. Es exactamente la
/// clase de fallo que `leccion-precondicion-inerte` ya cobró tres veces.
#[test]
fn el_mapa_de_esfuerzo_de_dsh_usa_solo_literales_medidos() {
    let manifiesto = dsh();
    let medidos = ["off", "low", "high", "max"];

    for nivel in ReasoningEffort::ALL {
        let Some(valor) = manifiesto.substitutions().resolve_reasoning_effort(*nivel) else {
            continue;
        };
        assert!(
            medidos.contains(&valor),
            "`{nivel}` resuelve a `{valor}`, que no es uno de los cuatro literales \
             que el adaptador de DeepSeek acepta: {medidos:?}"
        );
    }
}
