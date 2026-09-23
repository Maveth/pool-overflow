//! Agent TOML config.

use anyhow::{Context, Result};
use overflow_core::{Criteria, Peer};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    /// Overflow front (ASICs may dial here *or* you HAP/socat in front of house SV1).
    pub listen_sv1: SocketAddr,
    /// True local house SV1 when KeepLocal.
    pub local_sv1_backend: SocketAddr,
    #[serde(default)]
    pub local_peer_id: String,
    /// Operator-supplied estimate of current house SV1 share of network (%).
    #[serde(default)]
    pub house_sv1_network_share_pct: f64,
    #[serde(default = "default_authorize_wait_ms")]
    pub authorize_wait_ms: u64,
    #[serde(default = "default_max_conns")]
    pub max_forward_connections: usize,
    #[serde(default = "default_true")]
    pub tcp_nodelay: bool,
    #[serde(default = "default_http")]
    pub listen_http: SocketAddr,
    #[serde(default = "default_leases_path")]
    pub leases_path: PathBuf,
    #[serde(default = "default_addresses_path")]
    pub addresses_path: PathBuf,
    pub criteria: Criteria,
    #[serde(default)]
    pub peers: Vec<Peer>,
    #[serde(default)]
    pub federation_directory_url: Option<String>,
    /// Optional DATUM GW door registry (TOML with [doors.ID] host/port/api).
    #[serde(default)]
    pub gw_doors_path: Option<PathBuf>,
    /// Admin password for GW `/clients.json` + `/cmd` (or set via env below).
    #[serde(default)]
    pub gw_admin_password: Option<String>,
    /// If set, read GW admin password from this env var (preferred over plaintext).
    #[serde(default)]
    pub gw_admin_password_env: Option<String>,
}

fn default_authorize_wait_ms() -> u64 {
    800
}
fn default_max_conns() -> usize {
    64
}
fn default_true() -> bool {
    true
}
fn default_http() -> SocketAddr {
    "0.0.0.0:29791".parse().expect("static")
}
fn default_leases_path() -> PathBuf {
    PathBuf::from("leases.json")
}
fn default_addresses_path() -> PathBuf {
    PathBuf::from("addresses.json")
}

impl AgentConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&raw).context("parse agent.toml")
    }

    pub fn gw_password(&self) -> Option<String> {
        if let Some(env_name) = &self.gw_admin_password_env {
            if let Ok(v) = std::env::var(env_name) {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
        self.gw_admin_password.clone()
    }
}
