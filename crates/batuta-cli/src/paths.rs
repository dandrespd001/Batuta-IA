//! Dónde vive el estado de batuta.
//!
//! Se decide **una vez**, se documenta, y no se dispersa. Elegirlo mal se paga
//! más tarde, cuando el MCP y la línea de órdenes tengan que compartirlo: dos
//! procesos que no coinciden en dónde están los leases no se excluyen entre sí,
//! y la admisión deja de admitir nada.

use std::path::{Path, PathBuf};

/// Los tres directorios del estado, colgando de una raíz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// El estado bajo una raíz dada. Es lo que usan las pruebas.
    pub fn under(root: PathBuf) -> Self {
        Self { root }
    }

    /// El estado del usuario: `$XDG_STATE_HOME/batuta`, o `~/.local/state/batuta`.
    ///
    /// `$XDG_STATE_HOME` es la variable correcta y no `$XDG_DATA_HOME`: leases y
    /// recibos son estado de una máquina, no datos que uno quiera sincronizar
    /// entre varias. Un lease sincronizado a otra máquina describiría un proceso
    /// que allí no existe.
    ///
    /// # Errors
    ///
    /// Si no hay ni `XDG_STATE_HOME` ni `HOME`, que es no tener dónde escribir.
    pub fn from_env() -> std::io::Result<Self> {
        todo!()
    }

    /// La raíz.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Los leases. Es la raíz que espera `LeaseStore::open`.
    pub fn leases(&self) -> PathBuf {
        self.root.join("leases")
    }

    /// Los recibos.
    pub fn receipts(&self) -> PathBuf {
        self.root.join("recibos")
    }

    /// Los árboles y ficheros de corrida.
    pub fn runs(&self) -> PathBuf {
        self.root.join("corridas")
    }
}
