//! Reloj y espera inyectables del coordinador.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Reloj inyectable para que tests y `resume` compartan las mismas reglas.
pub trait RunClock: Send + Sync {
    /// Instante Unix UTC en milisegundos.
    fn now_millis(&self) -> u64;
}

/// Espera inyectable; no consulta ni produce otros efectos.
pub trait RunSleeper: Send + Sync {
    /// Duerme el plazo ya reservado.
    fn sleep_millis(&self, millis: u64);
}

#[derive(Debug)]
pub(super) struct SystemRunClock;

impl RunClock for SystemRunClock {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| {
                u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
            })
    }
}

#[derive(Debug)]
pub(super) struct SystemRunSleeper;

impl RunSleeper for SystemRunSleeper {
    fn sleep_millis(&self, millis: u64) {
        std::thread::sleep(Duration::from_millis(millis));
    }
}

pub(super) static SYSTEM_CLOCK: SystemRunClock = SystemRunClock;
pub(super) static SYSTEM_SLEEPER: SystemRunSleeper = SystemRunSleeper;
