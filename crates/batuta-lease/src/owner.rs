//! La prueba de vida del dueño de un lease.
//!
//! **Aquí vive la decisión que separa a batuta de lo que hace todo el mundo.** El
//! problema es viejo: un cerrojo cuyo dueño se murió bloquea a todos para
//! siempre. La solución habitual es caducarlo por antigüedad, y es mala, porque
//! un dueño lento y un dueño muerto se parecen.
//!
//! dsh, ante lo mismo, decidió no reclamar nunca: *«a contender times out without
//! removing the existing lock because age cannot distinguish a crashed owner from
//! a paused live writer; orphan recovery is an operator action»*. **Tienen razón
//! sobre la antigüedad.** Por eso batuta no la usa: pregunta si el proceso existe.
//!
//! El par `(pid, start_time)` es lo que hace la pregunta contestable. Sólo con el
//! `pid` no bastaría —los PID se reutilizan, y un lease huérfano podría parecer
//! vivo porque otro proceso heredó su número—; con el instante de arranque, dos
//! procesos distintos nunca coinciden.
//!
//! Es R3 aplicada a la admisión: no se decide por heurística, se decide mirando
//! el hecho.

use serde::{Deserialize, Serialize};

/// Quién tiene un lease, con lo justo para poder demostrar si sigue vivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    /// Identificador del proceso.
    pub pid: u32,
    /// Su grupo de procesos: lo que `killpg` mata entero (R6).
    pub pgid: u32,
    /// Instante de arranque, en tics desde el arranque del sistema.
    ///
    /// Es el campo 22 de `/proc/<pid>/stat`. Sin él, la reutilización de PID
    /// haría pasar por vivo a un dueño muerto.
    pub start_time: u64,
}

impl Owner {
    /// Los datos del proceso que llama.
    ///
    /// # Errors
    ///
    /// Si `/proc` no se puede leer.
    pub fn current() -> std::io::Result<Self> {
        todo!("pid, pgid y el campo 22 de /proc/self/stat")
    }

    /// ¿Sigue vivo **este mismo** proceso?
    ///
    /// No pregunta «¿existe el pid?», que es la pregunta equivocada: pregunta si
    /// el proceso que hay en ese pid es el que tomó el lease.
    pub fn is_alive(&self) -> bool {
        todo!("leer /proc/<pid>/stat y comparar start_time; ausente = muerto")
    }
}
