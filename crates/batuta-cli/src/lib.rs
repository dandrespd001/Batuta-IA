//! La línea de órdenes de batuta.
//!
//! La lógica vive aquí y no en `main.rs` para que se pueda probar desde fuera
//! sin lanzar un proceso: un test que sólo mira el código de salida de un binario
//! no puede decir **por qué** falló, y «falló» no es un diagnóstico.

pub mod args;
pub mod command;
pub mod error;
pub mod panel;
pub mod paths;

pub use args::{CANARY_FLAGS, CANARY_SWITCHES, COMMANDS, Command, PANEL_FLAGS, USAGE, parse};
pub use command::{CanaryOutcome, canary, canary_all};
pub use error::CliError;
pub use panel::{Fila, filas, tabla};
pub use paths::Layout;
