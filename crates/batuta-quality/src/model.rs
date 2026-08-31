use std::collections::BTreeMap;
use std::fmt;

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

/// Procedencia de una observación de benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Resultado publicado por una organización distinta del fabricante.
    Independent,
    /// Resultado publicado por el fabricante o proveedor de la ruta.
    Manufacturer,
    /// Evaluación reproducible ejecutada sobre esta ruta exacta.
    LocalEvaluation,
}

/// Resultado bruto de un benchmark sobre una ruta exacta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkObservation {
    /// Versión del documento.
    pub schema_version: u16,
    /// Identificador inmutable de la observación.
    pub id: String,
    /// Harness, proveedor y modelo exactos del resultado.
    pub route: RouteRef,
    /// Benchmark.
    pub benchmark: String,
    /// Versión del benchmark.
    pub benchmark_version: String,
    /// Escenario o subconjunto.
    pub scenario: String,
    /// Configuración publicada.
    pub configuration: String,
    /// Scaffold o agente con el que se obtuvo el resultado.
    pub scaffold: String,
    /// Revisión concreta del modelo, si el publicador la expone.
    pub model_revision: String,
    /// Nombre de la métrica original.
    pub metric: String,
    /// Métrica normalizada explícitamente a `0..100`.
    pub normalized_score: f64,
    /// Fuente primaria del dato.
    pub source_url: String,
    /// Fecha UTC como segundos Unix.
    pub observed_at: u64,
    /// Tipo de fuente.
    pub source_kind: SourceKind,
}

impl BenchmarkObservation {
    pub(crate) fn validate(&self) -> Result<(), QualityError> {
        if self.schema_version != 2 {
            return Err(QualityError::SchemaVersion {
                document: "benchmark_observation",
                received: self.schema_version,
                supported: 2,
            });
        }
        for (field, value) in [
            ("id", self.id.as_str()),
            ("benchmark", self.benchmark.as_str()),
            ("benchmark_version", self.benchmark_version.as_str()),
            ("scenario", self.scenario.as_str()),
            ("configuration", self.configuration.as_str()),
            ("scaffold", self.scaffold.as_str()),
            ("model_revision", self.model_revision.as_str()),
            ("metric", self.metric.as_str()),
        ] {
            validate_text(field, value)?;
        }
        validate_score("normalized_score", self.normalized_score)?;
        if !(self.source_url.starts_with("https://") || self.source_url.starts_with("http://")) {
            return Err(QualityError::InvalidField {
                field: "source_url",
                message: "must be an absolute http(s) URL".to_string(),
            });
        }
        Ok(())
    }
}

/// Documento v1 conservado sólo para una migración explícita.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkObservationV1 {
    /// Versión legada, necesariamente uno.
    pub schema_version: u16,
    /// Identificador inmutable.
    pub id: String,
    /// Ruta medida.
    pub route: RouteRef,
    /// Benchmark.
    pub benchmark: String,
    /// Versión del benchmark.
    pub benchmark_version: String,
    /// Escenario.
    pub scenario: String,
    /// Configuración.
    pub configuration: String,
    /// Antiguo nombre del scaffold.
    pub compatibility_group: String,
    /// Revisión observada.
    pub model_revision: String,
    /// Métrica.
    pub metric: String,
    /// Puntaje normalizado.
    pub normalized_score: f64,
    /// Fuente primaria.
    pub source_url: String,
    /// Instante observado.
    pub observed_at: u64,
    /// Tipo de fuente.
    pub source_kind: SourceKind,
}

impl BenchmarkObservationV1 {
    /// Convierte de forma explícita un documento v1 al contrato v2.
    ///
    /// # Errors
    ///
    /// Si no es realmente v1 o el documento resultante incumple el contrato.
    pub fn migrate(self) -> Result<BenchmarkObservation, QualityError> {
        if self.schema_version != 1 {
            return Err(QualityError::SchemaVersion {
                document: "benchmark_observation_v1",
                received: self.schema_version,
                supported: 1,
            });
        }
        let migrated = BenchmarkObservation {
            schema_version: 2,
            id: self.id,
            route: self.route,
            benchmark: self.benchmark,
            benchmark_version: self.benchmark_version,
            scenario: self.scenario,
            configuration: self.configuration,
            scaffold: self.compatibility_group,
            model_revision: self.model_revision,
            metric: self.metric,
            normalized_score: self.normalized_score,
            source_url: self.source_url,
            observed_at: self.observed_at,
            source_kind: self.source_kind,
        };
        migrated.validate()?;
        Ok(migrated)
    }
}

/// Peso de un escenario dentro de la cesta de una acción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkWeight {
    /// Benchmark.
    pub benchmark: String,
    /// Escenario.
    pub scenario: String,
    /// Versión compatible.
    pub benchmark_version: String,
    /// Configuración exacta compatible.
    pub configuration: String,
    /// Scaffold exacto compatible.
    pub scaffold: String,
    /// Métrica exacta compatible.
    pub metric: String,
    /// Revisión esperada; `None` sólo cuando la ruta tampoco la fija.
    pub expected_model_revision: Option<String>,
    /// Peso porcentual entero.
    pub weight: u8,
}

impl BenchmarkWeight {
    /// Construye un componente validado de la cesta.
    ///
    /// # Errors
    ///
    /// Si un identificador está vacío o el peso no pertenece a `1..=100`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        benchmark: impl Into<String>,
        scenario: impl Into<String>,
        benchmark_version: impl Into<String>,
        configuration: impl Into<String>,
        scaffold: impl Into<String>,
        metric: impl Into<String>,
        expected_model_revision: Option<&str>,
        weight: u8,
    ) -> Result<Self, QualityError> {
        let result = Self {
            benchmark: benchmark.into(),
            scenario: scenario.into(),
            benchmark_version: benchmark_version.into(),
            configuration: configuration.into(),
            scaffold: scaffold.into(),
            metric: metric.into(),
            expected_model_revision: expected_model_revision.map(str::to_string),
            weight,
        };
        for (field, value) in [
            ("benchmark", result.benchmark.as_str()),
            ("scenario", result.scenario.as_str()),
            ("benchmark_version", result.benchmark_version.as_str()),
            ("configuration", result.configuration.as_str()),
            ("scaffold", result.scaffold.as_str()),
            ("metric", result.metric.as_str()),
        ] {
            validate_text(field, value)?;
        }
        if let Some(revision) = result.expected_model_revision.as_deref() {
            validate_text("expected_model_revision", revision)?;
        }
        if !(1..=100).contains(&result.weight) {
            return Err(QualityError::InvalidField {
                field: "weight",
                message: format!("expected 1..=100, received {}", result.weight),
            });
        }
        Ok(result)
    }
}

/// Cesta y requisitos de producción para una acción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionProfile {
    /// Identificador de la acción.
    pub action: String,
    /// Componentes cuyos pesos suman 100.
    pub basket: Vec<BenchmarkWeight>,
    /// Cobertura mínima para considerar verificada la calidad.
    pub minimum_coverage: u8,
    /// Antigüedad máxima de una observación, en segundos.
    pub max_age_seconds: u64,
}

impl ActionProfile {
    /// Construye un perfil y comprueba que los pesos sumen exactamente 100.
    ///
    /// # Errors
    ///
    /// Si la acción, pesos, cobertura o antigüedad incumplen el esquema.
    pub fn new(
        action: impl Into<String>,
        basket: Vec<BenchmarkWeight>,
        minimum_coverage: u8,
        max_age_seconds: u64,
    ) -> Result<Self, QualityError> {
        let result = Self {
            action: action.into(),
            basket,
            minimum_coverage,
            max_age_seconds,
        };
        validate_text("action", &result.action)?;
        let sum: u16 = result
            .basket
            .iter()
            .map(|item| u16::from(item.weight))
            .sum();
        if result.basket.is_empty() || sum != 100 {
            return Err(QualityError::InvalidWeights { sum });
        }
        if result.minimum_coverage > 100 {
            return Err(QualityError::InvalidField {
                field: "minimum_coverage",
                message: format!("expected 0..=100, received {}", result.minimum_coverage),
            });
        }
        if result.max_age_seconds == 0 {
            return Err(QualityError::InvalidField {
                field: "max_age_seconds",
                message: "must be greater than zero".to_string(),
            });
        }
        Ok(result)
    }

    pub(crate) fn validate(&self) -> Result<(), QualityError> {
        Self::new(
            self.action.clone(),
            self.basket.clone(),
            self.minimum_coverage,
            self.max_age_seconds,
        )
        .map(|_| ())
    }
}

/// Cestas iniciales del SPEC, devueltas como datos ordinarios y editables.
///
/// # Errors
///
/// Si la cobertura o antigüedad solicitadas no forman perfiles válidos.
pub fn initial_action_profiles(
    minimum_coverage: u8,
    max_age_seconds: u64,
) -> Result<BTreeMap<String, ActionProfile>, QualityError> {
    let weight = |benchmark: &str, scenario: &str, value| {
        BenchmarkWeight::new(
            benchmark,
            scenario,
            "v1",
            "official",
            "official",
            "pass_rate",
            None,
            value,
        )
    };
    let definitions = [
        (
            "implementation",
            vec![
                weight("swe-bench", "verified", 80)?,
                weight("local-route", "implementation", 20)?,
            ],
        ),
        (
            "repair",
            vec![
                weight("swe-bench", "verified", 70)?,
                weight("local-route", "repair", 30)?,
            ],
        ),
        (
            "code_generation",
            vec![weight("livecodebench", "generation", 100)?],
        ),
        (
            "code_execution",
            vec![weight("livecodebench", "execution-repair", 100)?],
        ),
        ("tools", vec![weight("bfcl", "tools", 100)?]),
        (
            "web_research",
            vec![
                weight("gaia", "agentic", 60)?,
                weight("bfcl", "agentic", 40)?,
            ],
        ),
        ("review", vec![weight("local-route", "review", 100)?]),
        (
            "documentation",
            vec![weight("local-route", "documentation", 100)?],
        ),
    ];
    definitions
        .into_iter()
        .map(|(action, basket)| {
            ActionProfile::new(action, basket, minimum_coverage, max_age_seconds)
                .map(|profile| (action.to_string(), profile))
        })
        .collect()
}

/// Operación de un evento de override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideOperation {
    /// Activa o sustituye el override efectivo.
    Set,
    /// Retira el override y recupera el puntaje investigado.
    Clear,
}

/// Evento append-only que modifica el override efectivo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideEvent {
    /// Identificador inmutable del evento.
    pub id: String,
    /// Operación auditada.
    pub operation: OverrideOperation,
    /// Puntaje manual; sólo existe en `set`.
    pub score: Option<f64>,
    /// Motivo revisable.
    pub reason: String,
    /// Fecha UTC como segundos Unix.
    pub recorded_at: u64,
    /// Persona o proceso autorizado que lo registró.
    pub author: String,
    /// Puntaje investigado conservado en el momento del cambio.
    pub researched_original: Option<f64>,
}

impl OverrideEvent {
    /// Construye un evento `set` sin destruir el valor investigado.
    ///
    /// # Errors
    ///
    /// Si el puntaje no está en `0..100` o razón/autor están vacíos.
    pub fn set(
        id: impl Into<String>,
        score: f64,
        reason: impl Into<String>,
        recorded_at: u64,
        author: impl Into<String>,
        researched_original: Option<f64>,
    ) -> Result<Self, QualityError> {
        validate_score("score", score)?;
        if let Some(original) = researched_original {
            validate_score("researched_original", original)?;
        }
        let result = Self {
            id: id.into(),
            operation: OverrideOperation::Set,
            score: Some(score),
            reason: reason.into(),
            recorded_at,
            author: author.into(),
            researched_original,
        };
        validate_text("id", &result.id)?;
        validate_text("reason", &result.reason)?;
        validate_text("author", &result.author)?;
        Ok(result)
    }

    /// Construye un evento `clear` que conserva el historial anterior.
    ///
    /// # Errors
    ///
    /// Si identificador, razón o autor están vacíos.
    pub fn clear(
        id: impl Into<String>,
        reason: impl Into<String>,
        recorded_at: u64,
        author: impl Into<String>,
    ) -> Result<Self, QualityError> {
        let result = Self {
            id: id.into(),
            operation: OverrideOperation::Clear,
            score: None,
            reason: reason.into(),
            recorded_at,
            author: author.into(),
            researched_original: None,
        };
        validate_text("id", &result.id)?;
        validate_text("reason", &result.reason)?;
        validate_text("author", &result.author)?;
        Ok(result)
    }

    pub(crate) fn validate(&self) -> Result<(), QualityError> {
        validate_text("id", &self.id)?;
        validate_text("reason", &self.reason)?;
        validate_text("author", &self.author)?;
        match (self.operation, self.score) {
            (OverrideOperation::Set, Some(score)) => validate_score("score", score)?,
            (OverrideOperation::Set, None) => {
                return Err(QualityError::InvalidField {
                    field: "score",
                    message: "set override requires a score".to_string(),
                });
            }
            (OverrideOperation::Clear, Some(_)) => {
                return Err(QualityError::InvalidField {
                    field: "score",
                    message: "clear override cannot contain a score".to_string(),
                });
            }
            (OverrideOperation::Clear, None) => {}
        }
        if let Some(original) = self.researched_original {
            validate_score("researched_original", original)?;
        }
        Ok(())
    }
}

/// Error de contrato o cálculo de calidad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityError {
    /// Versión que este binario no conoce.
    SchemaVersion {
        /// Documento rechazado.
        document: &'static str,
        /// Versión recibida.
        received: u16,
        /// Única versión admitida.
        supported: u16,
    },
    /// Campo inválido.
    InvalidField {
        /// Campo rechazado.
        field: &'static str,
        /// Regla incumplida.
        message: String,
    },
    /// Los pesos de la cesta no suman cien.
    InvalidWeights {
        /// Suma recibida.
        sum: u16,
    },
    /// Un identificador aparece dos veces.
    DuplicateObservation {
        /// Identificador duplicado.
        id: String,
    },
    /// Falló la serialización canónica.
    Serialization {
        /// Error del serializador.
        message: String,
    },
}

impl fmt::Display for QualityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaVersion {
                document,
                received,
                supported,
            } => write!(
                f,
                "invalid schema_version {received} for {document}; supported: {supported}"
            ),
            Self::InvalidField { field, message } => {
                write!(f, "invalid {field}: {message}")
            }
            Self::InvalidWeights { sum } => {
                write!(f, "benchmark weights must sum exactly 100; received {sum}")
            }
            Self::DuplicateObservation { id } => {
                write!(f, "duplicate benchmark observation id '{id}'")
            }
            Self::Serialization { message } => write!(f, "cannot serialize evidence: {message}"),
        }
    }
}

impl std::error::Error for QualityError {}

fn validate_text(field: &'static str, value: &str) -> Result<(), QualityError> {
    if value.trim().is_empty() || value.len() > 128 {
        return Err(QualityError::InvalidField {
            field,
            message: "must contain 1..=128 non-whitespace bytes".to_string(),
        });
    }
    Ok(())
}

fn validate_score(field: &'static str, value: f64) -> Result<(), QualityError> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(QualityError::InvalidField {
            field,
            message: format!("expected finite 0..=100, received {value}"),
        });
    }
    Ok(())
}
