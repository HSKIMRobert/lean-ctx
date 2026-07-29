//! Org key trust store — OSS stub (ADR-023).
//!
//! The full trust-pinning implementation lives in
//! `lean-ctx-enterprise/commercial-core`. This stub preserves the public
//! API. Without Enterprise, no org keys are pinned.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrustedKey {
    pub org: String,
    pub public_key: String,
    pub added_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrustStore {
    pub keys: Vec<TrustedKey>,
}

pub fn trust_path() -> Result<PathBuf, String> {
    let dir = crate::core::paths::data_dir()?;
    Ok(dir.join("org-trust.toml"))
}

pub fn load() -> Result<TrustStore, String> {
    Ok(TrustStore::default())
}

pub fn save(_store: &TrustStore) -> Result<(), String> {
    Ok(())
}

pub fn pin(_org: &str, _public_key: &str) -> Result<bool, String> {
    Ok(false)
}

pub fn remove(_public_key: &str) -> Result<bool, String> {
    Ok(false)
}

pub fn trusted_keys() -> Vec<TrustedKey> {
    Vec::new()
}

pub fn is_trusted(_public_key: &str) -> bool {
    false
}

pub fn any_pinned() -> bool {
    false
}
