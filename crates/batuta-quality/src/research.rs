use std::collections::BTreeSet;
use std::fmt;

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

use crate::hash::hash_json;
use crate::{BenchmarkObservation, QualityError};

/// Propuesta inmutable escrita en staging por una actualización bajo demanda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchProposal {
    /// Versión del documento.
    pub schema_version: u16,
    /// Identificador de staging.
    pub id: String,
    /// Fecha UTC como segundos Unix.
    pub created_at: u64,
    /// Ruta que realizó la investigación.
    pub researcher_route: RouteRef,
    /// Nuevas observaciones propuestas.
    pub observations: Vec<BenchmarkObservation>,
    /// Foto activa sobre la que se investigó.
    pub expected_active_hash: String,
    /// Sello del resto del documento.
    pub proposal_hash: String,
}

#[derive(Serialize)]
struct ProposalBody<'a> {
    schema_version: u16,
    id: &'a str,
    created_at: u64,
    researcher_route: &'a RouteRef,
    observations: &'a [BenchmarkObservation],
    expected_active_hash: &'a str,
}

impl ResearchProposal {
    /// Construye y sella una propuesta sin tocar la evidencia activa.
    ///
    /// # Errors
    ///
    /// Si una observación no valida o el documento no se puede sellar.
    pub fn new(
        id: impl Into<String>,
        created_at: u64,
        researcher_route: RouteRef,
        observations: Vec<BenchmarkObservation>,
        expected_active_hash: impl Into<String>,
    ) -> Result<Self, QualityError> {
        let mut result = Self {
            schema_version: 1,
            id: id.into(),
            created_at,
            researcher_route,
            observations,
            expected_active_hash: expected_active_hash.into(),
            proposal_hash: String::new(),
        };
        for observation in &result.observations {
            observation.validate()?;
            if observation.route == result.researcher_route {
                return Err(QualityError::InvalidField {
                    field: "researcher_route",
                    message: format!(
                        "route '{}' cannot certify observations about itself",
                        result.researcher_route
                    ),
                });
            }
        }
        result.proposal_hash = result.calculate_hash()?;
        Ok(result)
    }

    fn calculate_hash(&self) -> Result<String, QualityError> {
        hash_json(&ProposalBody {
            schema_version: self.schema_version,
            id: &self.id,
            created_at: self.created_at,
            researcher_route: &self.researcher_route,
            observations: &self.observations,
            expected_active_hash: &self.expected_active_hash,
        })
    }
}

/// Foto activa de evidencia. Aplicar devuelve otra foto; nunca muta ésta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveEvidence {
    schema_version: u16,
    observations: Vec<BenchmarkObservation>,
    evidence_hash: String,
}

impl ActiveEvidence {
    /// Construye una foto canónica ordenada por identificador.
    ///
    /// # Errors
    ///
    /// Si hay observaciones inválidas o identificadores duplicados.
    pub fn new(mut observations: Vec<BenchmarkObservation>) -> Result<Self, QualityError> {
        for observation in &observations {
            observation.validate()?;
        }
        observations.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in observations.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(QualityError::DuplicateObservation {
                    id: pair[0].id.clone(),
                });
            }
        }
        let evidence_hash = hash_json(&(1_u16, &observations))?;
        Ok(Self {
            schema_version: 1,
            observations,
            evidence_hash,
        })
    }

    /// Observaciones activas; el staging no aparece aquí.
    pub fn observations(&self) -> &[BenchmarkObservation] {
        &self.observations
    }

    /// Hash de la foto activa.
    pub fn evidence_hash(&self) -> &str {
        &self.evidence_hash
    }

    pub(crate) fn revalidate(&self) -> Result<Self, QualityError> {
        if self.schema_version != 1 {
            return Err(QualityError::SchemaVersion {
                document: "active_evidence",
                received: self.schema_version,
                supported: 1,
            });
        }
        let rebuilt = Self::new(self.observations.clone())?;
        if rebuilt.evidence_hash != self.evidence_hash {
            return Err(QualityError::InvalidField {
                field: "evidence_hash",
                message: "does not match active observations".to_string(),
            });
        }
        Ok(rebuilt)
    }

    /// Aplica una propuesta confirmada y sellada, creando una foto nueva.
    ///
    /// # Errors
    ///
    /// Si falta confirmación, cambió un hash, hay conflicto o los datos son
    /// inválidos.
    pub fn apply(
        &self,
        proposal: &ResearchProposal,
        confirmed: bool,
    ) -> Result<Self, ProposalError> {
        if !confirmed {
            return Err(ProposalError::NotConfirmed);
        }
        if proposal.schema_version != 1 {
            return Err(ProposalError::SchemaVersion {
                received: proposal.schema_version,
            });
        }
        if proposal.calculate_hash().map_err(ProposalError::Quality)? != proposal.proposal_hash {
            return Err(ProposalError::HashMismatch);
        }
        if proposal.expected_active_hash != self.evidence_hash {
            return Err(ProposalError::ActiveChanged);
        }
        let existing: BTreeSet<&str> = self
            .observations
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        if let Some(conflict) = proposal
            .observations
            .iter()
            .find(|item| existing.contains(item.id.as_str()))
        {
            return Err(ProposalError::ObservationConflict {
                id: conflict.id.clone(),
            });
        }
        let mut combined = self.observations.clone();
        combined.extend(proposal.observations.clone());
        Self::new(combined).map_err(ProposalError::Quality)
    }
}

/// Rechazo al intentar activar una propuesta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalError {
    /// Falta la confirmación humana visible.
    NotConfirmed,
    /// El documento cambió después de sellarse.
    HashMismatch,
    /// La evidencia activa cambió desde que se creó staging.
    ActiveChanged,
    /// Versión desconocida.
    SchemaVersion {
        /// Versión recibida.
        received: u16,
    },
    /// La propuesta intenta reemplazar una observación inmutable.
    ObservationConflict {
        /// Identificador existente.
        id: String,
    },
    /// Propuesta ausente en staging.
    NotFound {
        /// Identificador pedido.
        id: String,
    },
    /// Fallo de E/S persistiendo una foto o propuesta.
    Io {
        /// Ruta afectada.
        path: String,
        /// Error del sistema.
        message: String,
    },
    /// JSON persistido ilegible.
    Json {
        /// Ruta afectada.
        path: String,
        /// Error de parseo.
        message: String,
    },
    /// Error de contrato de calidad.
    Quality(QualityError),
}

impl fmt::Display for ProposalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfirmed => f.write_str("proposal_not_confirmed"),
            Self::HashMismatch => f.write_str("proposal_hash_mismatch"),
            Self::ActiveChanged => f.write_str("active_evidence_changed"),
            Self::SchemaVersion { received } => {
                write!(
                    f,
                    "invalid proposal schema_version {received}; supported: 1"
                )
            }
            Self::ObservationConflict { id } => {
                write!(f, "observation '{id}' already exists and is immutable")
            }
            Self::NotFound { id } => write!(f, "research proposal '{id}' is not staged"),
            Self::Io { path, message } => write!(f, "cannot access '{path}': {message}"),
            Self::Json { path, message } => {
                write!(f, "invalid research JSON in '{path}': {message}")
            }
            Self::Quality(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ProposalError {}
