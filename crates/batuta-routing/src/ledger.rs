//! Ledger durable de reservas y consumo por grant.

#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use batuta_lease::{LeaseSpace, LeaseStore};
use serde::{Deserialize, Serialize};

use crate::snapshot_store::atomic_write;
use crate::{ExecutionGrantV1, GrantLimits};

/// Cantidad en las cuatro dimensiones presupuestarias.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetAmount {
    /// Solicitudes.
    pub requests: u64,
    /// Tokens de entrada.
    pub input_tokens: u64,
    /// Tokens de salida.
    pub output_tokens: u64,
    /// Tiempo de pared.
    pub wall_time_ms: u64,
}

impl BudgetAmount {
    fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            requests: self.requests.checked_add(other.requests)?,
            input_tokens: self.input_tokens.checked_add(other.input_tokens)?,
            output_tokens: self.output_tokens.checked_add(other.output_tokens)?,
            wall_time_ms: self.wall_time_ms.checked_add(other.wall_time_ms)?,
        })
    }

    fn fits(self, limits: GrantLimits) -> bool {
        self.requests <= limits.requests
            && self.input_tokens <= limits.input_tokens
            && self.output_tokens <= limits.output_tokens
            && self.wall_time_ms <= limits.wall_time_ms
    }

    fn fits_within(self, maximum: Self) -> bool {
        self.requests <= maximum.requests
            && self.input_tokens <= maximum.input_tokens
            && self.output_tokens <= maximum.output_tokens
            && self.wall_time_ms <= maximum.wall_time_ms
    }
}

/// Reserva de una corrida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    /// Máximo reservado antes del efecto.
    pub reserved: BudgetAmount,
    /// Consumo confirmado cuando el resultado es conocido.
    pub confirmed: Option<BudgetAmount>,
    /// Hubo inicio durable pero no resultado durable.
    pub outcome_unknown: bool,
}

impl Reservation {
    fn charged(&self) -> BudgetAmount {
        if self.outcome_unknown {
            self.reserved
        } else {
            self.confirmed.unwrap_or(self.reserved)
        }
    }
}

/// Vista completa de presupuesto de un grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerStatus {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Grant asociado.
    pub grant_id: String,
    /// Reservas por run ID.
    pub reservations: BTreeMap<String, Reservation>,
    /// Total cargado actualmente.
    pub consumed: BudgetAmount,
}

/// Almacén RMW serializado por lease interproceso.
#[derive(Debug, Clone)]
pub struct LedgerStore {
    root: PathBuf,
    leases: PathBuf,
}

impl LedgerStore {
    /// Abre ubicaciones sin crear documentos.
    pub const fn open(root: PathBuf, leases: PathBuf) -> Self {
        Self { root, leases }
    }

    /// Reserva máximos antes de una invocación.
    pub fn reserve(
        &self,
        grant: &ExecutionGrantV1,
        run_id: &str,
        amount: BudgetAmount,
    ) -> Result<LedgerStatus, BudgetError> {
        self.reserve_many(grant, &[(run_id.to_string(), amount)])
    }

    /// Reserva varias partidas en una única actualización durable.
    ///
    /// Esto permite fijar espera y próximo intento antes de dormir sin dejar
    /// media operación publicada si la segunda partida no cabe.
    pub fn reserve_many(
        &self,
        grant: &ExecutionGrantV1,
        reservations: &[(String, BudgetAmount)],
    ) -> Result<LedgerStatus, BudgetError> {
        if reservations.is_empty() {
            return Err(BudgetError::Invalid(
                "at least one reservation is required".to_string(),
            ));
        }
        let mut ids = BTreeSet::new();
        for (id, amount) in reservations {
            validate_id(id)?;
            if !ids.insert(id) {
                return Err(BudgetError::DuplicateReservation(id.clone()));
            }
            if *amount == BudgetAmount::default() {
                return Err(BudgetError::Invalid(
                    "a reservation must charge at least one dimension".to_string(),
                ));
            }
        }
        self.update(grant, |status| {
            let mut next = status.consumed;
            for (id, amount) in reservations {
                if status.reservations.contains_key(id) {
                    return Err(BudgetError::DuplicateReservation(id.clone()));
                }
                next = next.checked_add(*amount).ok_or(BudgetError::Overflow)?;
                if !next.fits(grant.limits) {
                    return Err(BudgetError::Exceeded);
                }
            }
            for (id, amount) in reservations {
                status.reservations.insert(
                    id.clone(),
                    Reservation {
                        reserved: *amount,
                        confirmed: None,
                        outcome_unknown: false,
                    },
                );
            }
            Ok(())
        })
    }

    /// Confirma consumo conocido y libera sólo el remanente demostrado.
    pub fn confirm(
        &self,
        grant: &ExecutionGrantV1,
        run_id: &str,
        actual: BudgetAmount,
    ) -> Result<LedgerStatus, BudgetError> {
        self.update(grant, |status| {
            let reservation = status
                .reservations
                .get_mut(run_id)
                .ok_or_else(|| BudgetError::MissingReservation(run_id.to_string()))?;
            if !actual.fits_within(reservation.reserved) {
                return Err(BudgetError::ActualExceedsReservation);
            }
            reservation.confirmed = Some(actual);
            reservation.outcome_unknown = false;
            Ok(())
        })
    }

    /// Conserva íntegramente la reserva cuando el efecto es ambiguo.
    pub fn mark_outcome_unknown(
        &self,
        grant: &ExecutionGrantV1,
        run_id: &str,
    ) -> Result<LedgerStatus, BudgetError> {
        self.update(grant, |status| {
            let reservation = status
                .reservations
                .get_mut(run_id)
                .ok_or_else(|| BudgetError::MissingReservation(run_id.to_string()))?;
            reservation.confirmed = None;
            reservation.outcome_unknown = true;
            Ok(())
        })
    }

    /// Lee una vista y recalcula el total.
    pub fn status(&self, grant_id: &str) -> Result<LedgerStatus, BudgetError> {
        validate_id(grant_id)?;
        let bytes = std::fs::read(self.path(grant_id)).map_err(BudgetError::Io)?;
        let mut status: LedgerStatus = serde_json::from_slice(&bytes).map_err(BudgetError::Json)?;
        if status.schema_version != 1 || status.grant_id != grant_id {
            return Err(BudgetError::Invalid("ledger identity mismatch".to_string()));
        }
        status.consumed = sum(&status.reservations)?;
        Ok(status)
    }

    fn update(
        &self,
        grant: &ExecutionGrantV1,
        mutate: impl FnOnce(&mut LedgerStatus) -> Result<(), BudgetError>,
    ) -> Result<LedgerStatus, BudgetError> {
        let leases = LeaseStore::open(&self.leases)
            .map_err(|error| BudgetError::Lease(error.to_string()))?;
        let _guard = leases
            .acquire(
                LeaseSpace::Repository,
                &format!("ledger-{}", grant.id),
                &grant.id,
            )
            .map_err(|error| BudgetError::Lease(error.to_string()))?;
        let mut status = match std::fs::read(self.path(&grant.id)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(BudgetError::Json)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => LedgerStatus {
                schema_version: 1,
                grant_id: grant.id.clone(),
                reservations: BTreeMap::new(),
                consumed: BudgetAmount::default(),
            },
            Err(error) => return Err(BudgetError::Io(error)),
        };
        if status.schema_version != 1 || status.grant_id != grant.id {
            return Err(BudgetError::Invalid("ledger identity mismatch".to_string()));
        }
        status.consumed = sum(&status.reservations)?;
        mutate(&mut status)?;
        status.consumed = sum(&status.reservations)?;
        if !status.consumed.fits(grant.limits) {
            return Err(BudgetError::Exceeded);
        }
        let mut bytes = serde_json::to_vec(&status).map_err(BudgetError::Json)?;
        bytes.push(b'\n');
        atomic_write(&self.path(&grant.id), &bytes).map_err(BudgetError::Io)?;
        Ok(status)
    }

    fn path(&self, grant_id: &str) -> PathBuf {
        self.root.join(format!("{grant_id}.json"))
    }
}

fn sum(reservations: &BTreeMap<String, Reservation>) -> Result<BudgetAmount, BudgetError> {
    reservations
        .values()
        .try_fold(BudgetAmount::default(), |total, item| {
            total
                .checked_add(item.charged())
                .ok_or(BudgetError::Overflow)
        })
}

fn validate_id(id: &str) -> Result<(), BudgetError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(BudgetError::Invalid(format!("invalid identifier '{id}'")));
    }
    Ok(())
}

/// Error de reserva o persistencia.
#[derive(Debug)]
pub enum BudgetError {
    /// Entrada inválida.
    Invalid(String),
    /// Reserva repetida.
    DuplicateReservation(String),
    /// Reserva ausente.
    MissingReservation(String),
    /// Presupuesto agotado.
    Exceeded,
    /// Consumo real incoherente.
    ActualExceedsReservation,
    /// Desbordamiento.
    Overflow,
    /// Exclusión interproceso.
    Lease(String),
    /// E/S.
    Io(std::io::Error),
    /// JSON.
    Json(serde_json::Error),
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Lease(message) => f.write_str(message),
            Self::DuplicateReservation(id) => write!(f, "reservation already exists: {id}"),
            Self::MissingReservation(id) => write!(f, "reservation not found: {id}"),
            Self::Exceeded => f.write_str("grant budget exceeded"),
            Self::ActualExceedsReservation => f.write_str("actual usage exceeds reservation"),
            Self::Overflow => f.write_str("budget arithmetic overflow"),
            Self::Io(error) => write!(f, "ledger I/O failed: {error}"),
            Self::Json(error) => write!(f, "ledger JSON failed: {error}"),
        }
    }
}

impl std::error::Error for BudgetError {}
