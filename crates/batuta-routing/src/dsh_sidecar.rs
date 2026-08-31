//! Cliente acotado del sidecar de catálogo DSH por JSONL/stdio.

#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write as _;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{CatalogImportReport, DshCatalogBridge};

/// Proceso y límites configurados exclusivamente por el servicio.
#[derive(Debug, Clone)]
pub struct DshSidecarClient {
    program: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    timeout: Duration,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
}

impl DshSidecarClient {
    /// Fija comando, entorno allowlisted y límites positivos.
    pub fn new(
        program: PathBuf,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        timeout: Duration,
        max_stdout_bytes: usize,
        max_stderr_bytes: usize,
    ) -> Result<Self, DshSidecarError> {
        if timeout.is_zero() || max_stdout_bytes == 0 || max_stderr_bytes == 0 {
            return Err(DshSidecarError::Configuration);
        }
        Ok(Self {
            program,
            args,
            env,
            timeout,
            max_stdout_bytes,
            max_stderr_bytes,
        })
    }

    /// Ejecuta una captura de catálogo sin llamada de modelo.
    pub fn catalog_snapshot(&self, id: &str) -> Result<CatalogImportReport, DshSidecarError> {
        validate_id(id)?;
        let request = SidecarRequest {
            schema_version: 1,
            id,
            method: "catalog_snapshot",
        };
        let mut input = serde_json::to_vec(&request).map_err(DshSidecarError::Json)?;
        input.push(b'\n');
        let mut command = Command::new(&self.program);
        command
            .env_clear()
            .args(&self.args)
            .envs(&self.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = command.spawn().map_err(DshSidecarError::Io)?;
        let mut stdin = child.stdin.take().ok_or(DshSidecarError::Protocol(
            "sidecar stdin was not piped".to_string(),
        ))?;
        stdin.write_all(&input).map_err(DshSidecarError::Io)?;
        drop(stdin);
        let stdout = child.stdout.take().ok_or(DshSidecarError::Protocol(
            "sidecar stdout was not piped".to_string(),
        ))?;
        let stderr = child.stderr.take().ok_or(DshSidecarError::Protocol(
            "sidecar stderr was not piped".to_string(),
        ))?;
        let stdout_limit = self.max_stdout_bytes;
        let stderr_limit = self.max_stderr_bytes;
        let stdout_thread = std::thread::spawn(move || read_limited(stdout, stdout_limit));
        let stderr_thread = std::thread::spawn(move || read_limited(stderr, stderr_limit));
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(DshSidecarError::Io)? {
                break status;
            }
            if started.elapsed() >= self.timeout {
                let pid = nix::unistd::Pid::from_raw(i32::try_from(child.id()).map_err(|_| {
                    DshSidecarError::Protocol("sidecar pid did not fit i32".to_string())
                })?);
                let _ = nix::sys::signal::killpg(pid, nix::sys::signal::Signal::SIGKILL);
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(DshSidecarError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let (stdout, stdout_truncated) = stdout_thread
            .join()
            .map_err(|_| DshSidecarError::Protocol("stdout reader panicked".to_string()))?;
        let (stderr, stderr_truncated) = stderr_thread
            .join()
            .map_err(|_| DshSidecarError::Protocol("stderr reader panicked".to_string()))?;
        if stdout_truncated || stderr_truncated {
            return Err(DshSidecarError::Truncated);
        }
        if !status.success() {
            return Err(DshSidecarError::Exit {
                code: status.code(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            });
        }
        let text = std::str::from_utf8(&stdout)
            .map_err(|_| DshSidecarError::Protocol("stdout is not UTF-8".to_string()))?;
        let lines: Vec<_> = text.lines().collect();
        if lines.len() != 1 {
            return Err(DshSidecarError::Protocol(
                "sidecar must emit exactly one response line".to_string(),
            ));
        }
        let response: SidecarResponse =
            serde_json::from_str(lines[0]).map_err(DshSidecarError::Json)?;
        if response.schema_version != 1 || response.id != id {
            return Err(DshSidecarError::Protocol(
                "sidecar response correlation mismatch".to_string(),
            ));
        }
        if !response.ok {
            let error = response.error.ok_or_else(|| {
                DshSidecarError::Protocol("error response has no error".to_string())
            })?;
            return Err(DshSidecarError::Remote {
                code: error.code,
                field: error.field,
                message: error.message,
            });
        }
        let result = response.result.ok_or_else(|| {
            DshSidecarError::Protocol("success response has no result".to_string())
        })?;
        let json = serde_json::to_string(&result).map_err(DshSidecarError::Json)?;
        DshCatalogBridge::import_json(&json)
            .map_err(|error| DshSidecarError::Protocol(error.to_string()))
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct SidecarRequest<'a> {
    schema_version: u16,
    id: &'a str,
    method: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarResponse {
    schema_version: u16,
    id: String,
    ok: bool,
    result: Option<SidecarResult>,
    error: Option<SidecarRemoteError>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarResult {
    routes: Vec<SidecarRoute>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarRoute {
    provider: String,
    model: String,
    revision: Option<String>,
    modalities: Vec<String>,
    context_window: Option<u64>,
    reasoning_efforts: Vec<String>,
    cost: SidecarCost,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarCost {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SidecarRemoteError {
    code: String,
    field: String,
    message: String,
    #[serde(rename = "details")]
    _details: BTreeMap<String, serde_json::Value>,
}

fn read_limited(mut reader: impl std::io::Read, maximum: usize) -> (Vec<u8>, bool) {
    let mut retained = Vec::with_capacity(maximum.min(8 * 1024));
    let mut buffer = [0_u8; 8 * 1024];
    let mut truncated = false;
    while let Ok(read) = reader.read(&mut buffer) {
        if read == 0 {
            break;
        }
        let remaining = maximum.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    (retained, truncated)
}

fn validate_id(id: &str) -> Result<(), DshSidecarError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(DshSidecarError::Configuration);
    }
    Ok(())
}

/// Fallo tipado del proceso o protocolo sidecar.
#[derive(Debug)]
pub enum DshSidecarError {
    /// Configuración insegura.
    Configuration,
    /// Tiempo agotado; el árbol fue terminado.
    Timeout,
    /// Alguna salida superó su límite.
    Truncated,
    /// Protocolo inválido.
    Protocol(String),
    /// Respuesta error cerrada.
    Remote {
        /// Código estable.
        code: String,
        /// Campo afectado.
        field: String,
        /// Mensaje.
        message: String,
    },
    /// Salida no cero.
    Exit {
        /// Código o señal.
        code: Option<i32>,
        /// Stderr ya acotado.
        stderr: String,
    },
    /// E/S.
    Io(std::io::Error),
    /// JSON.
    Json(serde_json::Error),
}

impl fmt::Display for DshSidecarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration => f.write_str("invalid DSH sidecar configuration"),
            Self::Timeout => f.write_str("DSH sidecar timed out"),
            Self::Truncated => f.write_str("DSH sidecar output exceeded its limit"),
            Self::Protocol(message) => write!(f, "DSH sidecar protocol error: {message}"),
            Self::Remote {
                code,
                field,
                message,
            } => write!(f, "DSH sidecar {code} at {field}: {message}"),
            Self::Exit { code, stderr } => {
                write!(f, "DSH sidecar exited {code:?}: {stderr}")
            }
            Self::Io(error) => write!(f, "DSH sidecar I/O failed: {error}"),
            Self::Json(error) => write!(f, "DSH sidecar JSON failed: {error}"),
        }
    }
}

impl std::error::Error for DshSidecarError {}
