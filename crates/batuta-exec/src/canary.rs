//! El canario: la corrida más pequeña que demuestra que un proveedor responde.
//!
//! **Observacional, nunca por subcadena sobre juicio propio.** R3 se paga aquí:
//! `provider-canary` devolvió `QUOTA_UNAVAILABLE` en 126 ms sin tocar la red,
//! porque leyó el `status` del mismo fichero que él debía informar. Aquí se
//! genera un token irrepetible, se pide que lo devuelva, y se comprueba que
//! **volvió ése**.
//!
//! Es también donde las cinco piezas se juntan por primera vez: manifiesto,
//! admisión, sustitución, ejecución y recibo.
// generado: deepseek-v4-flash - revisado: Arquitecto

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use batuta_contract::{CanaryExpectation, ProvenanceSource, WriteMode};
use batuta_lease::{LeaseSpace, LeaseStore};
use batuta_manifest::{ModelEntry, ProviderManifest};
use batuta_receipt::{Receipt, RunFacts};

use crate::error::ExecError;
use crate::materialize::materialize;
use crate::provenance::{read_after, read_stderr, sessions_dir, snapshot};
use crate::run::{build_env, run};
use crate::substitution::{RunContext, resolve, resolve_argv};

/// Lo que hace falta para lanzar un canario.
#[derive(Debug, Clone)]
pub struct CanaryRequest {
    /// Dónde trabaja el proceso. Para un canario basta un directorio temporal:
    /// es de sólo lectura y no hay diff que calcular.
    pub workdir: PathBuf,
    /// Dónde se materializan los ficheros de corrida. **Fuera del workdir.**
    pub run_dir: PathBuf,
    /// Dónde viven los leases.
    pub state_dir: PathBuf,
    /// La raíz del proveedor donde buscar su registro de sesión.
    pub dsh_home: PathBuf,
    /// Límite de pared.
    pub timeout: Duration,
    /// Identificador del encargo, para que el lease ocupado diga quién lo tiene.
    pub task_id: String,
}

/// Un token irrepetible para el canario.
///
/// De `/dev/urandom`, sin dependencia nueva. **Un token predecible dejaría de ser
/// observacional**: si se pudiera adivinar, un proveedor que devolviera texto
/// plausible sin llamar a nadie pasaría el canario, que es exactamente el fallo
/// de la puerta circular.
///
/// # Errors
///
/// Si `/dev/urandom` no se puede leer.
pub fn generate_token() -> std::io::Result<String> {
    // 16 bytes de entropía del sistema: la forma es `batuta-canario-` más el
    // hexadecimal en minúscula, para que un log diga de dónde salió el token.
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;

    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(format!("batuta-canario-{hex}"))
}

/// Ejecuta el canario entero y devuelve su recibo.
///
/// Toma los dos leases —por modelo y por repositorio— **antes** de arrancar y los
/// suelta al terminar. Es la otra mitad de R6: matar la tarea mata el árbol *y
/// libera el lease*, y el fallo que la paga dejaba un lease de repositorio
/// bloqueando a cualquier otro modelo.
///
/// El recibo sale sellado, verde o rojo. **Que salga rojo no es un error de esta
/// función**: es su respuesta.
///
/// # Errors
///
/// [`ExecError::Admission`] si otro encargo tiene los leases, y los de
/// sustitución, materialización o lanzamiento. Nada de eso es un veredicto: son
/// las cosas que impiden llegar a tener uno.
pub fn run_canary(
    manifest: &ProviderManifest,
    model: &ModelEntry,
    request: &CanaryRequest,
) -> Result<Receipt, ExecError> {
    // Admisión: los dos leases, por modelo y por repositorio, antes de arrancar.
    // Se guardan en variables **nombradas** y vivas hasta el final de la función:
    // son RAII, y al soltarse borran su fichero. Un `let _ =` las soltaría nada
    // más tomarlas y el lease desaparecería antes de la corrida.
    let store =
        LeaseStore::open(&request.state_dir).map_err(|source| ExecError::Admission { source })?;
    let _modelo = store
        .acquire(LeaseSpace::Model, model.id().as_str(), &request.task_id)
        .map_err(|source| ExecError::Admission { source })?;
    let _repo = store
        .acquire(
            LeaseSpace::Repository,
            request.workdir.to_string_lossy().as_ref(),
            &request.task_id,
        )
        .map_err(|source| ExecError::Admission { source })?;

    // El token irrepetible: sin él no hay nada que observar.
    let token = generate_token().map_err(|source| ExecError::TokenSource { source })?;

    // Huevo-y-gallina del prompt del canario: lleva `{token}`, y sustituir pide
    // un contexto. Se construye el contexto con el prompt vacío, se resuelve el
    // prompt contra él, y se mete el resultado en el contexto.
    let mut contexto = RunContext {
        model: model.id().clone(),
        route_model: model.route_model().clone(),
        route_provider: model.route_provider().map(str::to_string),
        workdir: request.workdir.clone(),
        run_dir: request.run_dir.clone(),
        prompt: String::new(),
        token: token.clone(),
        write_mode: WriteMode::ReadOnly,
    };
    contexto.prompt = resolve(
        manifest.canary().prompt(),
        "canary.prompt",
        manifest,
        &contexto,
    )?;

    // Los ficheros de corrida, fuera del worktree; `materialize` ya crea los
    // directorios que hagan falta.
    let ficheros = materialize(manifest, &contexto)?;

    let argv = resolve_argv(manifest, &contexto)?;

    // La resolución es la del manifiesto y no una propia: entiende `~` y `$PATH`
    // y comprueba el `sha256` (R11). Una búsqueda aparte no habría encontrado el
    // binario de dsh, cuyo `resolve` empieza por `~`.
    let programa = manifest
        .verify_executable()
        .map_err(|source| ExecError::Executable {
            source: Box::new(source),
        })?;

    let entorno = build_env(manifest.env());

    // La procedencia, antes y después: la diferencia es la sesión de esta corrida.
    let sesiones = sessions_dir(&request.dsh_home, &request.workdir);
    let antes = if manifest.provenance() == ProvenanceSource::SessionLog {
        snapshot(&sesiones)
    } else {
        BTreeSet::new()
    };

    let salida = run(
        &programa,
        &argv,
        &entorno,
        &request.workdir,
        request.timeout,
    )?;

    // Para `declared` el `Err` no es un fallo: `seal` ni lo mira con esa fuente,
    // y `model_confirmed` sale `false`, que es lo que hay que decir. Nunca se
    // fabrica un `Ok` con el modelo pedido.
    let observada = match manifest.provenance() {
        ProvenanceSource::SessionLog => read_after(&sesiones, &antes),
        // El patrón es obligatorio con esta fuente y la carga lo exige, así que
        // aquí no puede faltar. Si faltara, se dice: no se inventa.
        ProvenanceSource::StderrPattern => manifest.provenance_pattern().map_or_else(
            || Err("el manifiesto no declara `provenance.pattern`".to_string()),
            |patron| read_stderr(&salida.stderr, patron),
        ),
        ProvenanceSource::Declared => {
            Err("el proveedor declara su modelo: no deja registro que leer".to_string())
        }
    };

    let expected_token = if manifest.canary().expect() == CanaryExpectation::TokenEcho {
        Some(token)
    } else {
        None
    };

    Ok(Receipt::seal(RunFacts {
        provider: manifest.id().as_str().to_string(),
        model_requested: model.id().as_str().to_string(),
        route_model: model.route_model().as_str().to_string(),
        observed_as: model.observed_as().map(str::to_string),
        provenance_source: manifest.provenance(),
        manifest: manifest.origin().to_path_buf(),
        manifest_sha256: manifest.source_sha256().to_string(),
        argv: salida.argv,
        cwd: request.workdir.clone(),
        env_names: salida.env_names,
        runtime_files: ficheros,
        exit_code: salida.exit_code,
        stdout: salida.stdout,
        stderr: salida.stderr,
        duration: salida.duration,
        observed: observada,
        expected_token,
        declared_tools: Vec::new(),
        scope_violations: Vec::new(),
    }))
}
