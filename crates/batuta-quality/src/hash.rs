use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::QualityError;

pub(crate) fn hash_json(value: &impl Serialize) -> Result<String, QualityError> {
    let bytes = serde_json::to_vec(value).map_err(|error| QualityError::Serialization {
        message: error.to_string(),
    })?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}
