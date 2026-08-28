// generado: deepseek-v4-flash - revisado: Arquitecto
//! Ejecutar el proceso del proveedor, y **poseer su límite**.
//!
//! R6 en una frase: *el proceso es el límite; matar la tarea mata el árbol y
//! libera el lease*. El fallo que la paga: `TaskStop` dejaba el hijo vivo
//! gastando cuota, y su lease de repositorio bloqueando a cualquier otro modelo.
//!
//! La mitad difícil no necesita dependencias: `CommandExt::process_group(0)` es
//! biblioteca estándar y lanza al hijo como líder de su propio grupo. Comprobado
//! con sonda antes de escribir una línea: matar el grupo dejó cero nietos. Sólo
//! `killpg` viene de fuera.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use batuta_manifest::EnvPolicy;

use crate::error::ExecError;

/// Lo que la corrida produjo. **Hechos, no juicio.**
///
/// Va entero al recibo, que es quien concluye. `exit_code` es `Option` porque
/// `None` —lo mató una señal— y `Some(1)` son cosas distintas, y el diagnóstico
/// tiene que poder distinguirlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// El `argv` real con el que se lanzó, ya sustituido.
    pub argv: Vec<String>,
    /// Los **nombres** de las variables que se pasaron. Nunca los valores (R10).
    pub env_names: Vec<String>,
    /// Código de salida; `None` si murió por señal.
    pub exit_code: Option<i32>,
    /// Salida estándar completa.
    pub stdout: String,
    /// Error estándar **íntegro**, aunque el proceso saliera con cero.
    pub stderr: String,
    /// Cuánto duró de pared.
    pub duration: Duration,
    /// Si se agotó el límite y hubo que matar el grupo.
    pub timed_out: bool,
}

/// Construye el entorno del hijo **desde cero** (R5).
///
/// Nada se hereda sin nombrarlo: se parte de vacío, se copian sólo las variables
/// de `allow` que existan en el entorno actual, se retiran las de `deny` —que
/// gana siempre, porque hay variables que el proveedor lee para decidir su propia
/// contención— y se aplican las de `set`.
///
/// El fallo que lo paga: `--approval-mode auto` dio `auto_accept` 1 y 0 en dos
/// canarios seguidos para la misma clase de llamada. Contención determinista
/// significa que dos corridas iguales ven lo mismo.
pub fn build_env(policy: &EnvPolicy) -> Vec<(String, String)> {
    // Se parte de vacío: sólo lo que la allowlist nombra y existe ahora mismo.
    let mut variables: Vec<(String, String)> = policy
        .allow()
        .iter()
        .filter_map(|nombre| {
            std::env::var(nombre.as_str())
                .ok()
                .map(|valor| (nombre.as_str().to_string(), valor))
        })
        .collect();

    // Lo que el manifiesto fija llega siempre, estuviera o no en el entorno.
    for (nombre, valor) in policy.set() {
        match variables
            .iter_mut()
            .find(|(clave, _)| clave.as_str() == nombre.as_str())
        {
            Some((_, destino)) => destino.clone_from(valor),
            None => variables.push((nombre.as_str().to_string(), valor.clone())),
        }
    }

    // `deny` gana al final, sobre `allow` y sobre `set`: hay variables que el
    // proveedor lee para decidir su propia contención.
    variables.retain(|(nombre, _)| {
        !policy
            .deny()
            .iter()
            .any(|denegado| denegado.as_str() == nombre.as_str())
    });

    variables
}

/// Lanza el proceso, espera su límite, y si vence mata **el grupo entero**.
///
/// Los dos tubos se drenan en hilos aparte **mientras** el proceso corre: si se
/// esperara a que terminara antes de leer, un hijo que escriba más que el tamaño
/// del tubo se bloquearía escribiendo y nadie lo desbloquearía. Al vencer el
/// límite se mata el grupo con `killpg` —el hijo es su líder, `process_group(0)`
/// hace que su pid sea el pgid— y después se cosecha el proceso.
///
/// # Errors
///
/// [`ExecError::Spawn`] si el programa no se pudo lanzar. Que el proceso salga
/// con error **no** es un error de esta función: es un hecho que va al recibo.
///
/// # Panics
///
/// Sólo si `spawn` tuvo éxito pero el hijo no trajera los tubos que se pidieron
/// con `Stdio::piped`, o si su pid no cupiera en un `i32` — ninguna de las dos
/// puede ocurrir en un Unix real.
pub fn run(
    program: &Path,
    argv: &[String],
    env: &[(String, String)],
    cwd: &Path,
    timeout: Duration,
) -> Result<RunOutcome, ExecError> {
    let inicio = Instant::now();

    let mut comando = Command::new(program);
    // El entorno del hijo es el de `build_env` y ningún otro: sin `env_clear`,
    // `envs` sólo añadiría encima de lo heredado, y una variable no nombrada se
    // colaría (R5).
    comando
        .env_clear()
        .args(argv)
        .envs(
            env.iter()
                .map(|(nombre, valor)| (nombre.as_str(), valor.as_str())),
        )
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut hijo = comando.spawn().map_err(|source| ExecError::Spawn {
        program: program.to_path_buf(),
        source,
    })?;

    // Drenar los dos tubos desde el primer momento, en hilos aparte, y unirlos
    // al final: sin esto, un hijo que escupa megabytes se bloquea a mitad.
    let mut stdout = hijo
        .stdout
        .take()
        .expect("stdout: se pidió Stdio::piped al lanzar");
    let mut stderr = hijo
        .stderr
        .take()
        .expect("stderr: se pidió Stdio::piped al lanzar");
    let hilo_stdout = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let hilo_stderr = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    });

    let mut timed_out = false;

    let estado = loop {
        if let Some(estado) = hijo.try_wait().map_err(|source| ExecError::Spawn {
            program: program.to_path_buf(),
            source,
        })? {
            break estado;
        }
        if inicio.elapsed() >= timeout {
            timed_out = true;
            let grupo = nix::unistd::Pid::from_raw(
                i32::try_from(hijo.id()).expect("el pid de un hijo cabe en i32"),
            );
            // El grupo pudo ya no existir si el hijo salió justo al vencer el
            // límite; el `wait()` de abajo cosecha el estado que haya.
            let _ = nix::sys::signal::killpg(grupo, nix::sys::signal::Signal::SIGKILL);
            break hijo.wait().map_err(|source| ExecError::Spawn {
                program: program.to_path_buf(),
                source,
            })?;
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let stdout = hilo_stdout.join().unwrap_or_default();
    let stderr = hilo_stderr.join().unwrap_or_default();

    let mut env_names: Vec<String> = env.iter().map(|(nombre, _)| nombre.clone()).collect();
    env_names.sort();

    Ok(RunOutcome {
        argv: argv.to_vec(),
        env_names,
        exit_code: estado.code(),
        stdout,
        stderr,
        duration: inicio.elapsed(),
        timed_out,
    })
}

/// Resuelve el programa contra las rutas de `resolve` del manifiesto.
///
/// Existe aparte de `ProviderManifest::verify_executable` porque aquí se admite
/// una raíz distinta para las pruebas: los fixtures apuntan a `/bin/echo` y a
/// `/bin/sleep`, que no tienen `sha256` fijado ni falta que hace.
pub fn resolve_program(candidates: &[String]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|ruta| ruta.is_file())
}
