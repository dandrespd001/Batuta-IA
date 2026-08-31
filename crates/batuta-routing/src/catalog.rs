//! Normalización segura del catálogo descubierto por DSH.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr as _;

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Clase inicial de una ruta importada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogClass {
    /// Sólo sondas y pruebas hasta superar canarios exactos.
    ProbeTest,
    /// Promoción posterior y explícita.
    Production,
}

/// Coste publicado por DSH; `None` significa desconocido, nunca cero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CostComponents {
    /// Entrada por token.
    pub input: Option<f64>,
    /// Salida por token.
    pub output: Option<f64>,
    /// Lectura de caché.
    pub cache_read: Option<f64>,
    /// Escritura de caché.
    pub cache_write: Option<f64>,
}

impl CostComponents {
    fn proven_zero(self) -> bool {
        [self.input, self.output, self.cache_read, self.cache_write]
            .into_iter()
            .all(|component| component.is_some_and(|value| value.is_finite() && value == 0.0))
    }
}

/// Ruta segura normalizada desde DSH.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogRoute {
    /// Identidad exacta; el harness siempre es `dsh`.
    pub route: RouteRef,
    /// Coste declarado, conservando componentes desconocidos.
    pub cost: CostComponents,
    /// Una importación jamás promueve por sí sola.
    pub class: CatalogClass,
}

/// Catálogo ordenado, sin duplicados.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    schema_version: u16,
    routes: Vec<CatalogRoute>,
}

impl Catalog {
    pub(crate) const fn empty() -> Self {
        Self {
            schema_version: 2,
            routes: Vec::new(),
        }
    }

    /// Rutas en orden estable por identidad.
    pub fn routes(&self) -> &[CatalogRoute] {
        &self.routes
    }

    pub(crate) fn hash(&self) -> Result<String, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

/// Ruta rechazada antes de llegar a ejecución.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRejection {
    /// Código estable.
    pub code: String,
    /// Proveedor recibido.
    pub provider: String,
    /// Modelo recibido.
    pub model: String,
    /// Explicación sin datos sensibles.
    pub message: String,
}

/// Resultado revisable de una importación.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogImportReport {
    /// Catálogo normalizado que puede pasar a staging.
    pub catalog: Catalog,
    /// Entradas excluidas con razón estable.
    pub rejected: Vec<CatalogRejection>,
}

/// Adaptador puro de la API de descubrimiento DSH.
pub struct DshCatalogBridge;

impl DshCatalogBridge {
    /// Convierte JSON de descubrimiento en un catálogo sin material de cuenta.
    ///
    /// Los campos ajenos —incluidos credenciales, saldo o suscripción— nunca se
    /// copian al modelo normalizado.
    ///
    /// # Errors
    ///
    /// Si el documento no es JSON o no contiene una lista `routes` válida.
    pub fn import_json(input: &str) -> Result<CatalogImportReport, CatalogImportError> {
        let discovered: DiscoveryDocument =
            serde_json::from_str(input).map_err(CatalogImportError::Json)?;
        let mut routes = Vec::new();
        let mut rejected = Vec::new();
        let mut identities = BTreeSet::new();
        for item in discovered.routes {
            let cost = item.cost.unwrap_or_default();
            if item.provider == "opencode" && !cost.proven_zero() {
                rejected.push(rejection(
                    "opencode_cost_not_proven_zero",
                    &item,
                    "every OpenCode cost component must be known, finite and zero",
                ));
                continue;
            }
            let identity = if let Some(revision) = &item.revision {
                format!("dsh/{}/{}/{revision}", item.provider, item.model)
            } else {
                format!("dsh/{}/{}", item.provider, item.model)
            };
            let Ok(route) = RouteRef::from_str(&identity) else {
                rejected.push(rejection(
                    "invalid_route_ref",
                    &item,
                    "provider, model or revision is not a safe route segment",
                ));
                continue;
            };
            if !identities.insert(route.clone()) {
                rejected.push(rejection(
                    "duplicate_route_ref",
                    &item,
                    "DSH discovery returned the same exact route more than once",
                ));
                continue;
            }
            routes.push(CatalogRoute {
                route,
                cost,
                class: CatalogClass::ProbeTest,
            });
        }
        routes.sort_by(|left, right| left.route.cmp(&right.route));
        rejected.sort_by(|left, right| {
            (&left.provider, &left.model, &left.code).cmp(&(
                &right.provider,
                &right.model,
                &right.code,
            ))
        });
        Ok(CatalogImportReport {
            catalog: Catalog {
                schema_version: 2,
                routes,
            },
            rejected,
        })
    }
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    routes: Vec<DiscoveryRoute>,
}

#[derive(Deserialize)]
struct DiscoveryRoute {
    provider: String,
    model: String,
    #[serde(default)]
    revision: Option<String>,
    #[serde(default)]
    cost: Option<CostComponents>,
}

fn rejection(code: &str, route: &DiscoveryRoute, message: &str) -> CatalogRejection {
    CatalogRejection {
        code: code.to_string(),
        provider: route.provider.clone(),
        model: route.model.clone(),
        message: message.to_string(),
    }
}

/// Documento de descubrimiento ilegible.
#[derive(Debug)]
pub enum CatalogImportError {
    /// JSON inválido.
    Json(serde_json::Error),
}

impl fmt::Display for CatalogImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid DSH discovery JSON: {error}"),
        }
    }
}

impl std::error::Error for CatalogImportError {}
