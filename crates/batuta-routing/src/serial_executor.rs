//! Compuerta de exclusión para la invocación real del harness.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

/// Admite como máximo una llamada activa; nunca especula en paralelo.
#[derive(Debug)]
pub struct SerialExecutionGate {
    active: AtomicBool,
}

impl SerialExecutionGate {
    /// Crea una compuerta libre.
    pub const fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }

    /// Ejecuta la invocación sólo si ninguna otra está activa.
    ///
    /// La marca se libera también si la clausura entra en pánico durante el
    /// desenrollado.
    ///
    /// # Errors
    ///
    /// Si otra ruta o reintento ya ocupa la única ranura.
    pub fn run<T>(&self, invoke: impl FnOnce() -> T) -> Result<T, ExecutionBusy> {
        self.active
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| ExecutionBusy)?;
        let _guard = ActiveGuard { gate: self };
        Ok(invoke())
    }
}

impl Default for SerialExecutionGate {
    fn default() -> Self {
        Self::new()
    }
}

struct ActiveGuard<'a> {
    gate: &'a SerialExecutionGate,
}

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.gate.active.store(false, Ordering::Release);
    }
}

/// Ya existe una invocación en curso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionBusy;

impl fmt::Display for ExecutionBusy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("another route invocation is already active")
    }
}

impl std::error::Error for ExecutionBusy {}
