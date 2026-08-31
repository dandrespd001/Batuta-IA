//! Grants de ejecución inmutables y revocaciones append-only.

#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Operación externa que un grant puede autorizar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantOperation {
    /// Selección sin invocación.
    Route,
    /// Ejecución de trabajo.
    Run,
    /// Investigación síncrona.
    Research,
    /// Canario de capacidad.
    Canary,
}

/// Límites positivos máximos de un grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantLimits {
    /// Número total de invocaciones.
    pub requests: u64,
    /// Tokens de entrada.
    pub input_tokens: u64,
    /// Tokens de salida.
    pub output_tokens: u64,
    /// Tiempo total de pared.
    pub wall_time_ms: u64,
}

impl GrantLimits {
    /// Comprueba que todos los límites sean estrictamente positivos.
    pub fn validate(self) -> Result<(), GrantError> {
        if [
            self.requests,
            self.input_tokens,
            self.output_tokens,
            self.wall_time_ms,
        ]
        .contains(&0)
        {
            return Err(GrantError::Invalid(
                "all grant limits must be strictly positive".to_string(),
            ));
        }
        Ok(())
    }
}

/// Autorización sellada para efectos externos acotados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGrantDraftV1 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Identificador único.
    pub id: String,
    /// Emisión Unix UTC.
    pub issued_at: u64,
    /// Caducidad Unix UTC exclusiva.
    pub expires_at: u64,
    /// Manifest vigente revisado por el operador.
    pub manifest_hash: String,
    /// Rutas exactas autorizadas.
    pub routes: BTreeSet<RouteRef>,
    /// Acciones exactas autorizadas.
    pub actions: BTreeSet<String>,
    /// Operaciones autorizadas.
    pub operations: BTreeSet<GrantOperation>,
    /// Presupuesto máximo.
    pub limits: GrantLimits,
}

impl ExecutionGrantDraftV1 {
    /// Valida y añade el sello; el cliente nunca suministra `grant_hash`.
    ///
    /// # Errors
    ///
    /// Si la versión, identidad, vigencia, alcance o presupuesto son inválidos.
    pub fn seal(self) -> Result<ExecutionGrantV1, GrantError> {
        if self.schema_version != 1 {
            return Err(GrantError::Invalid(
                "grant schema_version must be 1".to_string(),
            ));
        }
        ExecutionGrantV1::new(
            self.id,
            self.issued_at,
            self.expires_at,
            self.manifest_hash,
            self.routes,
            self.actions,
            self.operations,
            self.limits,
        )
    }
}

/// Autorización sellada para efectos externos acotados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGrantV1 {
    /// Versión cerrada.
    pub schema_version: u16,
    /// Identificador único.
    pub id: String,
    /// Emisión Unix UTC.
    pub issued_at: u64,
    /// Caducidad Unix UTC exclusiva.
    pub expires_at: u64,
    /// Manifest que sirvió de base.
    pub manifest_hash: String,
    /// Rutas exactas autorizadas.
    pub routes: BTreeSet<RouteRef>,
    /// Acciones exactas autorizadas.
    pub actions: BTreeSet<String>,
    /// Operaciones autorizadas.
    pub operations: BTreeSet<GrantOperation>,
    /// Presupuesto máximo.
    pub limits: GrantLimits,
    /// Hash canónico del documento.
    pub grant_hash: String,
}

#[derive(Serialize)]
struct GrantBody<'a> {
    schema_version: u16,
    id: &'a str,
    issued_at: u64,
    expires_at: u64,
    manifest_hash: &'a str,
    routes: &'a BTreeSet<RouteRef>,
    actions: &'a BTreeSet<String>,
    operations: &'a BTreeSet<GrantOperation>,
    limits: GrantLimits,
}

impl ExecutionGrantV1 {
    /// Construye, valida y sella un grant.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        issued_at: u64,
        expires_at: u64,
        manifest_hash: String,
        routes: BTreeSet<RouteRef>,
        actions: BTreeSet<String>,
        operations: BTreeSet<GrantOperation>,
        limits: GrantLimits,
    ) -> Result<Self, GrantError> {
        let mut grant = Self {
            schema_version: 1,
            id,
            issued_at,
            expires_at,
            manifest_hash,
            routes,
            actions,
            operations,
            limits,
            grant_hash: String::new(),
        };
        grant.validate_body()?;
        grant.grant_hash = grant.calculate_hash()?;
        Ok(grant)
    }

    /// Revalida estructura, sello y vigencia.
    pub fn validate_at(&self, now: u64) -> Result<(), GrantError> {
        self.validate_seal()?;
        if now < self.issued_at || now >= self.expires_at {
            return Err(GrantError::NotActive {
                now,
                issued_at: self.issued_at,
                expires_at: self.expires_at,
            });
        }
        Ok(())
    }

    /// Revalida la estructura y el sello sin exigir vigencia temporal.
    ///
    /// Sirve para verificar documentos históricos, que deben seguir siendo
    /// auditables después de que el grant caduque o sea revocado.
    pub fn validate_seal(&self) -> Result<(), GrantError> {
        self.validate_body()?;
        if self.calculate_hash()? != self.grant_hash {
            return Err(GrantError::HashMismatch);
        }
        Ok(())
    }

    /// Comprueba ruta, acción y operación exactas.
    pub fn permits(&self, route: &RouteRef, action: &str, operation: GrantOperation) -> bool {
        self.routes.contains(route)
            && self.actions.contains(action)
            && self.operations.contains(&operation)
    }

    fn validate_body(&self) -> Result<(), GrantError> {
        if self.schema_version != 1 {
            return Err(GrantError::Invalid(
                "grant schema_version must be 1".to_string(),
            ));
        }
        validate_id(&self.id)?;
        validate_hash(&self.manifest_hash)?;
        if self.issued_at >= self.expires_at {
            return Err(GrantError::Invalid(
                "expires_at must be later than issued_at".to_string(),
            ));
        }
        if self.routes.is_empty() || self.actions.is_empty() || self.operations.is_empty() {
            return Err(GrantError::Invalid(
                "routes, actions and operations must be non-empty".to_string(),
            ));
        }
        for action in &self.actions {
            validate_id(action)?;
        }
        self.limits.validate()
    }

    fn calculate_hash(&self) -> Result<String, GrantError> {
        let bytes = serde_json::to_vec(&GrantBody {
            schema_version: self.schema_version,
            id: &self.id,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            manifest_hash: &self.manifest_hash,
            routes: &self.routes,
            actions: &self.actions,
            operations: &self.operations,
            limits: self.limits,
        })
        .map_err(GrantError::Json)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

/// Hecho de revocación append-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revocation {
    /// Grant revocado.
    pub grant_id: String,
    /// Instante UTC Unix.
    pub revoked_at: u64,
    /// Actor que confirmó la revocación.
    pub actor: String,
}

/// Vista de autorización actual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantStatus {
    /// Grant inmutable.
    pub grant: ExecutionGrantV1,
    /// Revocación más temprana, si existe.
    pub revocation: Option<Revocation>,
}

/// Almacén durable de grants y revocaciones.
#[derive(Debug, Clone)]
pub struct GrantStore {
    root: PathBuf,
}

impl GrantStore {
    /// Abre el directorio sin crearlo.
    pub const fn open(root: PathBuf) -> Self {
        Self { root }
    }

    /// Añade un grant inmutable.
    pub fn append(&self, grant: &ExecutionGrantV1) -> Result<(), GrantError> {
        grant.validate_at(grant.issued_at)?;
        let grants = self.root.join("objects");
        std::fs::create_dir_all(&grants).map_err(GrantError::Io)?;
        let path = grants.join(format!("{}.json", grant.id));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(GrantError::Io)?;
        let mut bytes = serde_json::to_vec(grant).map_err(GrantError::Json)?;
        bytes.push(b'\n');
        file.write_all(&bytes).map_err(GrantError::Io)?;
        file.sync_all().map_err(GrantError::Io)?;
        File::open(&grants)
            .and_then(|directory| directory.sync_all())
            .map_err(GrantError::Io)
    }

    /// Carga un grant y comprueba vigencia y revocación.
    pub fn authorize(&self, id: &str, now: u64) -> Result<ExecutionGrantV1, GrantError> {
        let status = self.status(id)?;
        status.grant.validate_at(now)?;
        if status.revocation.is_some() {
            return Err(GrantError::Revoked(id.to_string()));
        }
        Ok(status.grant)
    }

    /// Devuelve grant y revocación sin exigir vigencia temporal.
    pub fn status(&self, id: &str) -> Result<GrantStatus, GrantError> {
        validate_id(id)?;
        let bytes = std::fs::read(self.root.join("objects").join(format!("{id}.json")))
            .map_err(GrantError::Io)?;
        let grant: ExecutionGrantV1 = serde_json::from_slice(&bytes).map_err(GrantError::Json)?;
        grant.validate_body()?;
        if grant.calculate_hash()? != grant.grant_hash {
            return Err(GrantError::HashMismatch);
        }
        let revocation = self
            .revocations()?
            .into_iter()
            .filter(|item| item.grant_id == id)
            .min_by_key(|item| item.revoked_at);
        Ok(GrantStatus { grant, revocation })
    }

    /// Añade una revocación sincronizada.
    pub fn revoke(&self, id: &str, revoked_at: u64, actor: &str) -> Result<(), GrantError> {
        let _ = self.status(id)?;
        validate_id(actor)?;
        std::fs::create_dir_all(&self.root).map_err(GrantError::Io)?;
        let revocation = Revocation {
            grant_id: id.to_string(),
            revoked_at,
            actor: actor.to_string(),
        };
        let mut bytes = serde_json::to_vec(&revocation).map_err(GrantError::Json)?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("revocations.jsonl"))
            .map_err(GrantError::Io)?;
        file.write_all(&bytes).map_err(GrantError::Io)?;
        file.sync_all().map_err(GrantError::Io)
    }

    fn revocations(&self) -> Result<Vec<Revocation>, GrantError> {
        let path = self.root.join("revocations.jsonl");
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(GrantError::Io(error)),
        };
        text.lines()
            .map(|line| serde_json::from_str(line).map_err(GrantError::Json))
            .collect()
    }
}

fn validate_id(id: &str) -> Result<(), GrantError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(GrantError::Invalid(format!("invalid identifier '{id}'")));
    }
    Ok(())
}

fn validate_hash(hash: &str) -> Result<(), GrantError> {
    let Some(hex) = hash.strip_prefix("sha256:") else {
        return Err(GrantError::Invalid(
            "manifest_hash must be sha256".to_string(),
        ));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GrantError::Invalid(
            "manifest_hash must be sha256".to_string(),
        ));
    }
    Ok(())
}

/// Error de contrato o persistencia de grants.
#[derive(Debug)]
pub enum GrantError {
    /// Documento inválido.
    Invalid(String),
    /// Sello alterado.
    HashMismatch,
    /// Fuera de la ventana temporal.
    NotActive {
        /// Instante comprobado.
        now: u64,
        /// Emisión.
        issued_at: u64,
        /// Caducidad.
        expires_at: u64,
    },
    /// Revocado.
    Revoked(String),
    /// E/S.
    Io(std::io::Error),
    /// JSON.
    Json(serde_json::Error),
}

impl fmt::Display for GrantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::HashMismatch => f.write_str("grant_hash_mismatch"),
            Self::NotActive {
                now,
                issued_at,
                expires_at,
            } => write!(
                f,
                "grant_not_active: {now} not in [{issued_at}, {expires_at})"
            ),
            Self::Revoked(id) => write!(f, "grant_revoked: {id}"),
            Self::Io(error) => write!(f, "grant I/O failed: {error}"),
            Self::Json(error) => write!(f, "grant JSON failed: {error}"),
        }
    }
}

impl std::error::Error for GrantError {}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
