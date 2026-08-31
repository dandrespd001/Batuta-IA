//! Referencia inequívoca a una ruta ejecutable.

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `harness/provider/model[/revision]`: una ruta exacta, no un modelo suelto.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouteRef {
    harness: String,
    provider: String,
    model: String,
    revision: Option<String>,
}

impl RouteRef {
    /// Harness que ejecuta la ruta.
    pub fn harness(&self) -> &str {
        &self.harness
    }

    /// Proveedor o adaptador dentro del harness.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Modelo exacto dentro de la ruta.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Revisión exacta, cuando el catálogo la declara.
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
}

/// Texto que no cumple el contrato `harness/provider/model[/revision]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRefError {
    value: String,
}

impl fmt::Display for RouteRefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid route_ref '{}': expected harness/provider/model[/revision] with three or four non-empty safe segments",
            self.value
        )
    }
}

impl core::error::Error for RouteRefError {}

impl FromStr for RouteRef {
    type Err = RouteRefError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('/');
        let harness = parts.next();
        let provider = parts.next();
        let model = parts.next();
        let revision = parts.next();
        if parts.next().is_some()
            || !harness.is_some_and(valid_segment)
            || !provider.is_some_and(valid_segment)
            || !model.is_some_and(valid_segment)
            || revision.is_some_and(|value| !valid_segment(value))
        {
            return Err(RouteRefError {
                value: value.to_owned(),
            });
        }
        Ok(Self {
            harness: harness.expect("validated above").to_owned(),
            provider: provider.expect("validated above").to_owned(),
            model: model.expect("validated above").to_owned(),
            revision: revision.map(ToOwned::to_owned),
        })
    }
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment.len() <= 128
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

impl fmt::Display for RouteRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.harness, self.provider, self.model)?;
        if let Some(revision) = &self.revision {
            write!(f, "/{revision}")?;
        }
        Ok(())
    }
}

impl Serialize for RouteRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for RouteRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}
