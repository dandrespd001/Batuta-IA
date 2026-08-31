//! Worker único y acotado de la vista Execution.

use std::io::Cursor;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

use crate::{ApiErrorV2, Layout, RunCommand, execute_run_command};

/// Trabajo cerrado admitido por el único worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiExecutionJob {
    /// Ejecuta un `RunRequestV2` ya previsualizado.
    Run {
        /// JSON cerrado conservado por el formulario.
        request_json: String,
    },
    /// Reanuda por ID durable.
    Resume {
        /// Identificador exacto.
        id: String,
    },
}

/// Un solo hilo con cola acotada para mantener reactiva la terminal.
pub struct TuiExecutionWorker {
    sender: Option<SyncSender<TuiExecutionJob>>,
    results: Receiver<Result<String, ApiErrorV2>>,
    handle: Option<JoinHandle<()>>,
}

impl TuiExecutionWorker {
    /// Crea exactamente un worker con un manejador inyectable.
    pub fn spawn(
        handler: impl Fn(TuiExecutionJob) -> Result<String, ApiErrorV2> + Send + 'static,
    ) -> Self {
        let (sender, jobs) = mpsc::sync_channel(8);
        let (result_sender, results) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                if result_sender.send(handler(job)).is_err() {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            results,
            handle: Some(handle),
        }
    }

    /// Worker de producción: comparte Layout y la misma API que la CLI.
    pub fn for_layout(layout: Layout) -> Self {
        Self::spawn(move |job| match job {
            TuiExecutionJob::Run { request_json } => execute_run_command(
                &layout,
                &RunCommand::Start { file: None },
                &mut Cursor::new(request_json),
            ),
            TuiExecutionJob::Resume { id } => execute_run_command(
                &layout,
                &RunCommand::Resume { id },
                &mut Cursor::new(Vec::<u8>::new()),
            ),
        })
    }

    /// Encola sin bloquear la interfaz; aplica backpressure si la cola está llena.
    ///
    /// # Errors
    ///
    /// Si la cola está llena o el worker ya terminó.
    pub fn submit(&self, job: TuiExecutionJob) -> Result<(), ApiErrorV2> {
        let sender = self.sender.as_ref().ok_or_else(worker_closed)?;
        sender.try_send(job).map_err(|error| match error {
            TrySendError::Full(_) => ApiErrorV2::new(
                "worker_busy",
                "run",
                "the execution worker queue is full",
                serde_json::Value::Null,
            ),
            TrySendError::Disconnected(_) => worker_closed(),
        })
    }

    /// Consulta el resultado sin bloquear el redibujado.
    pub fn poll(&self) -> Option<Result<String, ApiErrorV2>> {
        self.results.try_recv().ok()
    }
}

impl Drop for TuiExecutionWorker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn worker_closed() -> ApiErrorV2 {
    ApiErrorV2::new(
        "worker_closed",
        "run",
        "the execution worker is not available",
        serde_json::Value::Null,
    )
}
