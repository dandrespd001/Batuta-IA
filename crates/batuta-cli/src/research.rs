//! Conexión CLI de staging, estado y aplicación de investigación.

use std::fs::OpenOptions;
use std::io::Write as _;

use serde::Serialize;

use crate::{Layout, ResearchScope};

/// Deja una solicitud explícita para el perfil investigador; no toca activo.
///
/// # Errors
///
/// Devuelve un error si no puede crear y sincronizar la solicitud en staging.
pub fn queue_research_update(layout: &Layout, scope: &ResearchScope) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let id = format!("research-{now}-{}", std::process::id());
    let request = UpdateRequest {
        schema_version: 1,
        id: &id,
        created_at: now,
        scope,
        status: "staged_request",
        activation: "never_automatic",
    };
    let dir = layout.research().join("requests");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(format!("{id}.json"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(&mut file, &request).map_err(|error| error.to_string())?;
    file.write_all(b"\n").map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    Ok(id)
}

/// Estado JSON de evidencia activa, propuestas y solicitudes pendientes.
///
/// # Errors
///
/// Devuelve un error si el almacén no puede leerse o el estado no puede serializarse.
pub fn research_status_json(layout: &Layout) -> Result<String, String> {
    let store = batuta_quality::ResearchStore::open(layout.research());
    let status = store.status().map_err(|error| error.to_string())?;
    let mut requests = Vec::new();
    let dir = layout.research().join("requests");
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|error| error.to_string())?;
                let path = entry.path();
                if path.extension().and_then(std::ffi::OsStr::to_str) == Some("json")
                    && let Some(stem) = path.file_stem().and_then(std::ffi::OsStr::to_str)
                {
                    requests.push(stem.to_string());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    requests.sort();
    serde_json::to_string(&serde_json::json!({
        "schema_version": 1,
        "active_hash": status.active_hash,
        "active_observations": status.active_observations,
        "staged_proposals": status.staged,
        "staged_requests": requests
    }))
    .map_err(|error| error.to_string())
}

/// Aplica una propuesta ya producida por el perfil investigador.
///
/// # Errors
///
/// Devuelve un error si falta confirmación o falla la validación o persistencia.
pub fn apply_research(layout: &Layout, proposal: &str, confirm: bool) -> Result<String, String> {
    let store = batuta_quality::ResearchStore::open(layout.research());
    let active = store
        .apply(proposal, confirm)
        .map_err(|error| error.to_string())?;
    Ok(active.evidence_hash().to_string())
}

#[derive(Serialize)]
struct UpdateRequest<'a> {
    schema_version: u16,
    id: &'a str,
    created_at: u64,
    scope: &'a ResearchScope,
    status: &'static str,
    activation: &'static str,
}
