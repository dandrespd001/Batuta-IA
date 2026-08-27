//! Carga y validación de manifiestos de proveedor.
//!
//! **R1 en una frase: un manifiesto irresoluble falla al cargar, no al
//! ejecutar.** El fallo que lo paga costó una corrida entera cada vez.
//!
//! A diferencia de [`batuta_contract`], este crate sí toca el disco —para eso
//! está—, pero la validación está partida en dos a propósito:
//! [`ProviderManifest::parse`] es pura y se prueba sin ficheros, y
//! [`ProviderManifest::load`] añade lo que sólo se puede saber mirando la
//! máquina.

// `ManifestError` es grande porque lleva fichero, línea, campo y la lista de
// valores válidos: eso es precisamente lo que R1 y R8 exigen de un error de
// carga. Meterlo en un `Box` encogería el `Result` y a cambio volvería
// incómodo el `matches!` de cada prueba, que es donde el mensaje se fija. La
// carga de un manifiesto no es camino caliente: se paga el tamaño a sabiendas.
#![allow(clippy::result_large_err)]

pub mod error;
pub mod manifest;
pub mod runtime_file;
pub mod substitution;

pub use error::{ManifestError, SourceLocation};
pub use manifest::{Auth, Canary, EnvPolicy, Executable, Invoke, ModelEntry, ProviderManifest};
pub use runtime_file::{RuntimeDocument, RuntimeFile};
pub use substitution::{BUILTIN_PLACEHOLDERS, Substitutions};
