//! El recibo de una corrida.
//!
//! **La regla que lo ordena todo: se anota lo observado, no lo pedido.** Se pidió
//! `deepseek-v4-flash` tres veces y corrió otro modelo las tres, porque el modelo
//! lo decidía un fichero que batuta no controlaba. Un recibo que hubiera anotado
//! la petición habría mentido sobre lo único que le da valor.

pub mod receipt;
pub mod verdict;

pub use receipt::{MaterializedFile, ObservedProvenance, Receipt, RunFacts};
pub use verdict::{RedReason, Verdict};
