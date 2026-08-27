// generado: deepseek-v4-flash - revisado: Arquitecto
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

use std::collections::BTreeSet;
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
    path: PathBuf,
    format: DocumentFormat,
    document: RuntimeDocument,
}

impl RuntimeFile {
    /// Construye un fichero de corrida ya validado. Sólo lo usa la carga.
    pub(crate) fn new(path: PathBuf, format: DocumentFormat, document: RuntimeDocument) -> Self {
        Self {
            path,
            format,
            document,
        }
    }

    /// Ruta, **siempre relativa al directorio de corrida**.
    ///
    /// Absoluta, con `..`, o apuntando dentro del worktree: error de carga. Un
    /// fichero de configuración de la corrida no es material del encargo y no
    /// debe aparecer jamás en el `git diff` que batuta calcula.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Formato en que se serializa.
    pub fn format(&self) -> DocumentFormat {
        self.format
    }

    /// El documento, ya con su forma decidida.
    pub fn document(&self) -> &RuntimeDocument {
        &self.document
    }

    /// Todas las llaves `{...}` que aparecen en el documento, en orden de
    /// aparición y sin repetir.
    ///
    /// Es lo que la carga contrasta contra las admitidas.
    pub fn placeholders(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        match &self.document {
            RuntimeDocument::List(items) => {
                for item in items {
                    collect_placeholders(item, &mut out, &mut seen);
                }
            }
            RuntimeDocument::Map(table) => {
                for value in table.values() {
                    collect_placeholders(value, &mut out, &mut seen);
                }
            }
        }
        out
    }
}

/// Las llaves `{...}` de un texto, en orden de aparición y sin repetir.
///
/// La comparten `argv` y los documentos de corrida: ambas son la frontera donde
/// batuta rellena por corrida.
pub(crate) fn extract_placeholders(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            if !key.is_empty() && seen.insert(key.to_string()) {
                out.push(key.to_string());
            }
            rest = &after[end + 1..];
        } else {
            // Una llave sin cierre no termina la búsqueda: puede haber más
            // `{...}` más adelante.
            rest = after;
        }
    }
    out
}

/// Recorre el documento entero, no sólo el primer nivel: las llaves viven en
/// cualquier profundidad de las tablas y listas de un documento de corrida.
fn collect_placeholders(value: &toml::Value, out: &mut Vec<String>, seen: &mut BTreeSet<String>) {
    match value {
        toml::Value::String(text) => {
            for key in extract_placeholders(text) {
                if seen.insert(key.clone()) {
                    out.push(key);
                }
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                collect_placeholders(item, out, seen);
            }
        }
        toml::Value::Table(table) => {
            for value in table.values() {
                collect_placeholders(value, out, seen);
            }
        }
        toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => {}
    }
}
