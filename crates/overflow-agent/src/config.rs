//! Agent TOML config: local backends + peer list + criteria.

use anyhow::{Context, Result};
use overflow_core::{Criteria, Peer};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    /// Where ASICs may dial *this* overflow front (optional stage).
    pub listen_sv1: SocketAddr,
    /// True local house SV1 / GW when we KeepLocal.
    pub local_sv1_backend: SocketAddr,
    #[serde(default)]
    pub local_peer_id: String,
    pub criteria: Criteria,
    #[serde(default)]
    pub peers: Vec<Peer>,
    /// If set, agent may refresh peers/rates from the directory (optional).
    #[serde(default)]
    pub federation_directory_url: Option<String>,
}

impl AgentConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&raw).context("parse agent.toml")
    }
}
