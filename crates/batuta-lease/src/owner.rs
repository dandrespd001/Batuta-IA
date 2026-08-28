// generado: deepseek-v4-flash - revisado: Arquitecto
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

use std::io::{self, ErrorKind};

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

/// Los índices de `/proc/<pid>/stat` que nos importan, contados **después** del
/// `)` que cierra el nombre del ejecutable.
///
/// El nombre va entre paréntesis y puede contener espacios y paréntesis, así
/// que partir la línea por espacios da campos corridos. Lo único fiable es
/// buscar el último `)` y contar desde ahí: el campo 3 (`state`) queda en el
/// índice 0, el 4 (`ppid`) en el 1, el 5 (`pgrp`) en el 2 y el 22 (`starttime`)
/// en el 19.
const PGRP_INDICE: usize = 2;
const START_TIME_INDICE: usize = 19;

/// Uno de los campos numéricos del `stat`, con un error que dice cuál faltaba.
fn campo_numero(campos: &[&str], indice: usize, que: &str) -> io::Result<u64> {
    let texto = campos
        .get(indice)
        .copied()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, format!("campo {que} ausente")))?;
    texto.parse().map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("campo {que} no numérico en stat: `{texto}`"),
        )
    })
}

impl Owner {
    /// Los datos del proceso que llama.
    ///
    /// # Errors
    ///
    /// Si `/proc` no se puede leer o su `stat` no tiene la forma esperada.
    pub fn current() -> io::Result<Self> {
        let pid = std::process::id();
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let (_, despues) = stat.rsplit_once(')').ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "stat sin nombre entre paréntesis")
        })?;
        let campos: Vec<&str> = despues.split_whitespace().collect();
        let grupo = u32::try_from(campo_numero(&campos, PGRP_INDICE, "pgrp")?)
            .map_err(|_| io::Error::new(ErrorKind::InvalidData, "pgrp fuera de rango u32"))?;
        let start_time = campo_numero(&campos, START_TIME_INDICE, "starttime")?;
        Ok(Self {
            pid,
            pgid: grupo,
            start_time,
        })
    }

    /// ¿Sigue vivo **este mismo** proceso?
    ///
    /// No pregunta «¿existe el pid?», que es la pregunta equivocada: pregunta si
    /// el proceso que hay en ese pid es el que tomó el lease. El `stat` ausente
    /// es la prueba de la muerte; un `start_time` distinto es la prueba de que
    /// el pid fue reutilizado.
    pub fn is_alive(&self) -> bool {
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", self.pid)) else {
            return false; // ausente: no hay proceso, no hay dueño
        };
        let Some((_, despues)) = stat.rsplit_once(')') else {
            return false; // sin la forma esperada no se puede demostrar que vive
        };
        let Some(start_time) = despues.split_whitespace().nth(START_TIME_INDICE) else {
            return false;
        };
        start_time
            .parse::<u64>()
            .is_ok_and(|t| t == self.start_time)
    }
}
