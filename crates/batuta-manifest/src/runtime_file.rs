//! Ficheros de configuración que batuta materializa **antes** de arrancar el
//! proceso del proveedor.
//!
//! Nacen de una medición: en dsh el modelo no viaja en `argv`. Va en un
//! documento de settings que gana a la capa de composición, y sin escribirlo no
//! hay forma de fijar qué modelo corre. Se probó con `--patch` solo: el árbol
//! cambiaba y la corrida seguía yendo a otro modelo.
//!
//! El núcleo no sabe qué es una «capa de parche de cordis». Sabe escribir
//! ficheros que un manifiesto describe. Por eso el campo es genérico, y la
//! prueba de que lo es de verdad es que `providers/abacus.toml` no declara
//! ninguno.

use std::path::{Path, PathBuf};

use batuta_contract::DocumentFormat;

/// Un documento de configuración es una lista o un mapa. No hay una tercera
/// forma, y declarar las dos —o ninguna— es error de carga: batuta no adivina
/// la forma por la pinta de la tabla.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeDocument {
    /// Documento que es una lista de entradas (`[[runtime_files.entry]]`).
    List(Vec<toml::Value>),
    /// Documento que es un mapa (`[runtime_files.content]`).
    Map(toml::Table),
}

/// Un fichero que batuta escribe por corrida.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeFile {
    _path: PathBuf,
    _format: DocumentFormat,
    _document: RuntimeDocument,
}

impl RuntimeFile {
    /// Ruta, **siempre relativa al directorio de corrida**.
    ///
    /// Absoluta, con `..`, o apuntando dentro del worktree: error de carga. Un
    /// fichero de configuración de la corrida no es material del encargo y no
    /// debe aparecer jamás en el `git diff` que batuta calcula.
    pub fn path(&self) -> &Path {
        todo!()
    }

    /// Formato en que se serializa.
    pub fn format(&self) -> DocumentFormat {
        todo!()
    }

    /// El documento, ya con su forma decidida.
    pub fn document(&self) -> &RuntimeDocument {
        todo!()
    }

    /// Todas las llaves `{...}` que aparecen en el documento, en orden de
    /// aparición y sin repetir.
    ///
    /// Es lo que la carga contrasta contra las admitidas.
    pub fn placeholders(&self) -> Vec<String> {
        todo!("recorrer el documento entero, no sólo el primer nivel")
    }
}
