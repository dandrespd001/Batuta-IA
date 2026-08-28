//! Por qué una corrida no se pudo ni intentar.
//!
//! Ojo con la distinción, porque es la que separa este error del veredicto del
//! recibo: **aquí sólo viven los fallos que impiden ejecutar**. Una corrida que
//! arrancó y salió mal no es un `ExecError`: es un recibo en rojo con su motivo.
//! Confundirlos haría que un modelo que contesta mal y un manifiesto roto se
//! parecieran, y son cosas distintas para quien lee.

use std::fmt;
use std::path::PathBuf;

/// La corrida no llegó a empezar.
#[derive(Debug)]
pub enum ExecError {
    /// Una llave `{...}` que no es incorporada ni declarada.
    UnknownPlaceholder {
        /// Dónde apareció: `invoke.argv` o la ruta del documento.
        field: String,
        /// La llave que no se reconoce.
        placeholder: String,
        /// Todas las admitidas (R8).
        expected: Vec<String>,
    },
    /// Un fichero de corrida caería **dentro del worktree**.
    ///
    /// Es la comprobación que `batuta-manifest` no podía hacer, porque `parse()`
    /// es puro y el worktree no existe cuando se carga un manifiesto. Aquí sí se
    /// conocen las dos rutas, y por eso vive aquí. Se rechaza **antes** de
    /// ejecutar: un fichero de configuración de batuta dentro del árbol del
    /// encargo aparecería en el diff como si fuera trabajo del modelo.
    RuntimeFileInsideWorktree {
        /// La ruta que caería dentro.
        path: PathBuf,
        /// El worktree que invade.
        worktree: PathBuf,
    },
    /// No se pudo escribir un fichero de corrida.
    Materialize {
        /// Ruta implicada.
        path: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
    /// El proceso no se pudo lanzar.
    Spawn {
        /// El programa que se intentó.
        program: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
}

impl fmt::Display for ExecError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("los mensajes los fijan los tests de este crate")
    }
}

impl std::error::Error for ExecError {}
