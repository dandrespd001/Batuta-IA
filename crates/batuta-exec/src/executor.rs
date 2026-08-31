//! Interfaz normalizada de exactamente una invocación de harness.

use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use batuta_contract::{RouteRef, TaskSpec};
use serde::{Deserialize, Serialize};

use crate::run;

/// Petición completa para una única llamada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationRequestV2 {
    /// Identificador durable de la ejecución.
    pub run_id: String,
    /// Ruta exacta ya autorizada.
    pub route: RouteRef,
    /// Entrada de esta llamada; no incluye historial implícito.
    pub objective: String,
    /// Contrato completo de la tarea.
    pub task: TaskSpec,
    /// Máximo de bytes conservados de stdout.
    pub max_output_bytes: u64,
    /// Límite de pared.
    pub timeout_ms: u64,
}

impl InvocationRequestV2 {
    /// Comprueba los límites y los campos no cubiertos por sus tipos.
    ///
    /// # Errors
    ///
    /// Si el ID, objetivo o límites son inválidos.
    pub fn validate(&self) -> Result<(), ExecutorError> {
        if self.run_id.is_empty()
            || self.run_id.len() > 128
            || !self
                .run_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            || self.objective.trim().is_empty()
            || self.max_output_bytes == 0
            || self.timeout_ms == 0
        {
            return Err(ExecutorError::InvalidRequest);
        }
        Ok(())
    }
}

/// Uso confirmado por el adaptador o fake.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenUsage {
    /// Tokens de entrada.
    pub input_tokens: u64,
    /// Tokens de salida.
    pub output_tokens: u64,
}

/// Taxonomía cerrada derivada de la respuesta, nunca de consultar una cuenta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationFailure {
    /// Rate limit, con plazo cuando el harness lo entregó.
    RateLimited {
        /// Espera indicada en milisegundos.
        retry_after_ms: Option<u64>,
    },
    /// Cuota agotada.
    Quota,
    /// Autenticación rechazada.
    Authentication,
    /// Saldo insuficiente comunicado por la llamada.
    Balance,
    /// Fallo transitorio.
    Transient,
    /// Tiempo agotado.
    Timeout,
    /// Fallo permanente no recuperable automáticamente.
    Permanent,
}

/// Resultado acotado de una única llamada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedInvocationResult {
    /// Salida acotada.
    pub output: String,
    /// Uso observado.
    pub usage: TokenUsage,
    /// Latencia de pared.
    pub latency_ms: u64,
    /// Procedencia observada, si el harness la hizo disponible.
    pub provenance: Option<String>,
    /// Hash del manifest de proveedor realmente materializado.
    pub manifest_hash: Option<String>,
    /// Fallo estructurado; `None` significa resultado conocido exitoso.
    pub failure: Option<InvocationFailure>,
}

/// Una implementación realiza exactamente una llamada por invocación del método.
pub trait HarnessExecutor: Send + Sync {
    /// Invoca una vez sin operaciones auxiliares.
    ///
    /// # Errors
    ///
    /// Devuelve [`ExecutorError`] cuando no puede iniciarse o completarse esa
    /// única invocación. El llamador debe tratar el resultado como ambiguo si
    /// ya había registrado de forma durable el inicio de la operación.
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError>;
}

/// Fake determinista e instrumentado.
#[derive(Debug)]
pub struct FakeHarnessExecutor {
    result: NormalizedInvocationResult,
    calls: AtomicU64,
}

impl FakeHarnessExecutor {
    /// Devuelve siempre el resultado indicado.
    pub const fn new(result: NormalizedInvocationResult) -> Self {
        Self {
            result,
            calls: AtomicU64::new(0),
        }
    }

    /// Número exacto de llamadas observadas.
    pub fn invocation_count(&self) -> u64 {
        self.calls.load(Ordering::Acquire)
    }
}

impl HarnessExecutor for FakeHarnessExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        request.validate()?;
        self.calls.fetch_add(1, Ordering::AcqRel);
        let mut result = self.result.clone();
        result.output = truncate_utf8(result.output, request.max_output_bytes)?;
        Ok(result)
    }
}

/// Adaptador del ejecutor de procesos existente, configurado por el servicio.
#[derive(Debug, Clone)]
pub struct ProcessHarnessExecutor {
    program: PathBuf,
    argv: Vec<String>,
    env: Vec<(String, String)>,
    cwd: PathBuf,
}

impl ProcessHarnessExecutor {
    /// Fija programa, argumentos base, entorno allowlisted y directorio.
    pub const fn new(
        program: PathBuf,
        argv: Vec<String>,
        env: Vec<(String, String)>,
        cwd: PathBuf,
    ) -> Self {
        Self {
            program,
            argv,
            env,
            cwd,
        }
    }
}

impl HarnessExecutor for ProcessHarnessExecutor {
    fn invoke(
        &self,
        request: &InvocationRequestV2,
    ) -> Result<NormalizedInvocationResult, ExecutorError> {
        request.validate()?;
        let mut argv = self.argv.clone();
        argv.push(request.objective.clone());
        let outcome = run(
            &self.program,
            &argv,
            &self.env,
            &self.cwd,
            Duration::from_millis(request.timeout_ms),
        )
        .map_err(ExecutorError::Exec)?;
        let failure = if outcome.timed_out {
            Some(InvocationFailure::Timeout)
        } else if outcome.exit_code == Some(0) {
            None
        } else {
            Some(InvocationFailure::Permanent)
        };
        let latency_ms = u64::try_from(outcome.duration.as_millis()).unwrap_or(u64::MAX);
        Ok(NormalizedInvocationResult {
            output: truncate_utf8(outcome.stdout, request.max_output_bytes)?,
            usage: TokenUsage::default(),
            latency_ms,
            provenance: None,
            manifest_hash: None,
            failure,
        })
    }
}

pub(crate) fn truncate_utf8(mut value: String, maximum: u64) -> Result<String, ExecutorError> {
    let maximum = usize::try_from(maximum).map_err(|_| ExecutorError::InvalidRequest)?;
    if value.len() <= maximum {
        return Ok(value);
    }
    let mut boundary = maximum;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    Ok(value)
}

/// Fallo de configuración o lanzamiento del adaptador.
#[derive(Debug)]
pub enum ExecutorError {
    /// Petición fuera del contrato.
    InvalidRequest,
    /// Configuración activa o ruta confiable inválidas.
    Configuration(String),
    /// El ejecutor de procesos no pudo lanzar o supervisar.
    Exec(crate::ExecError),
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest => f.write_str("invalid harness invocation request"),
            Self::Configuration(message) => f.write_str(message),
            Self::Exec(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExecutorError {}
