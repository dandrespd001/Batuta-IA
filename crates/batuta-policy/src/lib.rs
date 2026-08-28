//! El fichero de elección de batuta.
//!
//! §1 de `docs/FASE5_PANEL.md` separa tres capas y este crate es sólo la
//! tercera: **Declaración** (qué ofrece un proveedor, en `providers/*.toml`),
//! **Evidencia** (qué funcionó de verdad, en los recibos de `batuta canary`) y
//! **Elección** (qué queremos usar, aquí). Un modelo puede estar declarado y
//! tener evidencia y aun así no ser el que queremos: separar las tres es lo
//! que permite decir eso sin mentir en ningún fichero.
//!
//! Deliberadamente pequeño: no toca procesos, no toca leases, no sabe qué es
//! un canario. Sólo lee y escribe un TOML con una elección por modelo (R3, y
//! `tests/estructura.rs` lo hace cumplir contra este mismo `Cargo.toml`).

pub mod error;
pub mod politica;

pub use error::PoliticaError;
pub use politica::{EleccionModelo, Politica};
