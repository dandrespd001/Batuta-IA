//! Adaptadores del binario, agrupados por superficie.

mod legacy;
mod operational;
mod routing;

pub(crate) use legacy::{
    ejecutar_canario, ejecutar_disable, ejecutar_effort, ejecutar_enable, ejecutar_nuevo_modelo,
    ejecutar_nuevo_proveedor, ejecutar_panel, ejecutar_quitar_modelo,
};
pub(crate) use operational::{ejecutar_grant, ejecutar_profile, ejecutar_run};
pub(crate) use routing::{
    ejecutar_catalog, ejecutar_mcp, ejecutar_research, ejecutar_route, ejecutar_tui,
};
