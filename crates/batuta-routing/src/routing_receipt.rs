//! Recibo durable de selección, ejecución y relevo.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use batuta_contract::RouteRef;
use batuta_quality::QualityProjection;
use serde::{Deserialize, Serialize};

use crate::{HandoffCheckpoint, RouteDecision, RouteRequest};

/// Transición observada de la máquina de ejecución.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingTransition {
    /// Instante Unix UTC.
    pub at: u64,
    /// Estado anterior.
    pub from: String,
    /// Estado posterior.
    pub to: String,
    /// Ruta implicada, si existe.
    pub route: Option<RouteRef>,
}

impl RoutingTransition {
    /// Construye un hecho de transición compacto.
    pub fn new(at: u64, from: &str, to: &str, route: Option<RouteRef>) -> Self {
        Self {
            at,
            from: from.to_string(),
            to: to.to_string(),
            route,
        }
    }
}

/// Foto append-only de una decisión y su ejecución.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingReceipt {
    /// Versión del documento.
    pub schema_version: u16,
    /// Identificador único.
    pub id: String,
    /// Creación Unix UTC.
    pub created_at: u64,
    /// Petición ya resuelta contra el perfil local.
    pub request: RouteRequest,
    /// Todas las proyecciones evaluadas para esta acción.
    pub projections: Vec<QualityProjection>,
    /// Decisión exacta, incluidos descartes y autorizaciones.
    pub decision: RouteDecision,
    /// Hash histórico de política.
    pub policy_hash: String,
    /// Hash histórico de evidencia elegida.
    pub evidence_hash: String,
    /// Transiciones observadas, ordenadas.
    pub transitions: Vec<RoutingTransition>,
    /// Último checkpoint suficiente para continuar sin historial.
    pub checkpoint: Option<HandoffCheckpoint>,
}

impl RoutingReceipt {
    /// Sella un recibo coherente a partir de datos ya observados.
    ///
    /// # Errors
    ///
    /// Si identidad, hashes, proyección elegida o transiciones no concuerdan.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        created_at: u64,
        request: RouteRequest,
        projections: Vec<QualityProjection>,
        decision: RouteDecision,
        transitions: Vec<RoutingTransition>,
        checkpoint: Option<HandoffCheckpoint>,
    ) -> Result<Self, RoutingReceiptError> {
        let receipt = Self {
            schema_version: 2,
            id,
            created_at,
            policy_hash: decision.policy_hash.clone(),
            evidence_hash: decision.evidence_hash.clone(),
            request,
            projections,
            decision,
            transitions,
            checkpoint,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    fn validate(&self) -> Result<(), RoutingReceiptError> {
        validate_id(&self.id)?;
        if self.schema_version != 2
            || self.request.schema_version != 2
            || self.decision.schema_version != 2
        {
            return Err(RoutingReceiptError::SchemaVersion);
        }
        if self.policy_hash != self.decision.policy_hash
            || self.evidence_hash != self.decision.evidence_hash
        {
            return Err(RoutingReceiptError::HashMismatch);
        }
        let selected = self.projections.iter().filter(|projection| {
            projection.route == self.decision.route
                && projection.action == self.request.action
                && projection.evidence_hash == self.evidence_hash
        });
        if selected.count() != 1 {
            return Err(RoutingReceiptError::SelectedProjectionMismatch);
        }
        if self.transitions.is_empty()
            || self
                .transitions
                .windows(2)
                .any(|pair| pair[0].at > pair[1].at)
            || self
                .transitions
                .iter()
                .any(|transition| transition.from.is_empty() || transition.to.is_empty())
        {
            return Err(RoutingReceiptError::InvalidTransitions);
        }
        Ok(())
    }
}

/// Directorio append-only de recibos.
#[derive(Debug, Clone)]
pub struct RoutingReceiptStore {
    root: PathBuf,
}

impl RoutingReceiptStore {
    /// Abre el directorio sin crearlo todavía.
    pub const fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Añade un recibo nuevo y sincroniza fichero y directorio.
    ///
    /// # Errors
    ///
    /// Si el recibo no valida, el id existe o falla la persistencia.
    pub fn append(&self, receipt: &RoutingReceipt) -> Result<(), RoutingReceiptError> {
        receipt.validate()?;
        std::fs::create_dir_all(&self.root).map_err(RoutingReceiptError::Io)?;
        let path = self.root.join(format!("{}.json", receipt.id));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(RoutingReceiptError::Io)?;
        let mut bytes = serde_json::to_vec_pretty(receipt).map_err(RoutingReceiptError::Json)?;
        bytes.push(b'\n');
        file.write_all(&bytes).map_err(RoutingReceiptError::Io)?;
        file.flush().map_err(RoutingReceiptError::Io)?;
        file.sync_all().map_err(RoutingReceiptError::Io)?;
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(RoutingReceiptError::Io)
    }

    /// Carga y revalida un recibo después de reiniciar.
    ///
    /// # Errors
    ///
    /// Si el id es inválido, falta el fichero o su contenido fue alterado.
    pub fn load(&self, id: &str) -> Result<RoutingReceipt, RoutingReceiptError> {
        validate_id(id)?;
        let bytes =
            std::fs::read(self.root.join(format!("{id}.json"))).map_err(RoutingReceiptError::Io)?;
        let receipt: RoutingReceipt =
            serde_json::from_slice(&bytes).map_err(RoutingReceiptError::Json)?;
        receipt.validate()?;
        Ok(receipt)
    }
}

fn validate_id(id: &str) -> Result<(), RoutingReceiptError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(RoutingReceiptError::InvalidId(id.to_string()));
    }
    Ok(())
}

/// Fallo de sellado o almacenamiento.
#[derive(Debug)]
pub enum RoutingReceiptError {
    /// Identificador inseguro.
    InvalidId(String),
    /// Alguna versión no es v2.
    SchemaVersion,
    /// Los hashes de la decisión no coinciden.
    HashMismatch,
    /// Falta la proyección exacta elegida o está duplicada.
    SelectedProjectionMismatch,
    /// No hay transiciones o no están ordenadas.
    InvalidTransitions,
    /// E/S local.
    Io(std::io::Error),
    /// JSON inválido.
    Json(serde_json::Error),
}

impl fmt::Display for RoutingReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid routing receipt id '{id}'"),
            Self::SchemaVersion => f.write_str("routing receipt and request must be schema v2"),
            Self::HashMismatch => f.write_str("routing receipt hashes do not match its decision"),
            Self::SelectedProjectionMismatch => {
                f.write_str("routing receipt does not contain one exact selected projection")
            }
            Self::InvalidTransitions => f.write_str("routing receipt transitions are invalid"),
            Self::Io(error) => write!(f, "routing receipt I/O failed: {error}"),
            Self::Json(error) => write!(f, "routing receipt JSON failed: {error}"),
        }
    }
}

impl std::error::Error for RoutingReceiptError {}
