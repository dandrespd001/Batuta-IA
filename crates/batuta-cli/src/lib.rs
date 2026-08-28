//! La línea de órdenes de batuta.
//!
//! La lógica vive aquí y no en `main.rs` para que se pueda probar desde fuera
//! sin lanzar un proceso: un test que sólo mira el código de salida de un binario
//! no puede decir **por qué** falló, y «falló» no es un diagnóstico.

pub mod args;
pub mod command;
pub mod error;
pub mod paths;

pub use args::{CANARY_FLAGS, COMMANDS, Command, USAGE, parse};
pub use command::{CanaryOutcome, canary};
pub use error::CliError;
pub use paths::Layout;
