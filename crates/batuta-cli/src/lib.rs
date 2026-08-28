//! La línea de órdenes de batuta.
//!
//! La lógica vive aquí y no en `main.rs` para que se pueda probar desde fuera
//! sin lanzar un proceso: un test que sólo mira el código de salida de un binario
//! no puede decir **por qué** falló, y «falló» no es un diagnóstico.

pub mod args;
pub mod command;
pub mod declaracion;
pub mod eleccion;
pub mod error;
pub mod panel;
pub mod paths;

pub use args::{CANARY_FLAGS, CANARY_SWITCHES, COMMANDS, Command, PANEL_FLAGS, USAGE, parse};
pub use command::{CanaryOutcome, canary, canary_all};
pub use declaracion::{
    anexar_modelo, nuevo_modelo, nuevo_proveedor, plantilla_proveedor, quitar_modelo,
    quitar_modelo_de,
};
pub use eleccion::{disable, effort, enable};
pub use error::CliError;
pub use panel::{Fila, escribir_html, filas, tabla, tabla_html};
pub use paths::Layout;
