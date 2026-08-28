//! La evidencia de batuta, consultable.
//!
//! §1 de `docs/FASE5_PANEL.md`: la **Evidencia** es la segunda de tres capas
//! —qué funcionó de verdad, y cuándo—, y sólo la produce `batuta canary`. Este
//! crate no produce nada: sólo lee lo que ya está en
//! `~/.local/state/batuta/recibos/` y responde una pregunta muy concreta,
//! ¿hay un recibo verde reciente para este modelo, con este manifiesto?
//!
//! No toca procesos, no toca leases, no decide qué usar —eso es
//! `batuta-policy`—. Sólo lee.

pub mod error;
pub mod store;

pub use error::StoreError;
pub use store::{DEFAULT_TTL, LatestGreen, Lookup, ReceiptStore, Unreadable};
