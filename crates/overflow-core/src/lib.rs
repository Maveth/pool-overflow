//! Shared types for pool-local overflow and optional federation directory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Curated destination (or self) in the operator peer list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    /// SV1 host:port the agent may pump to.
    pub sv1_backend: String,
    /// Optional DATUM endpoint / note for later 0xA4 targeting.
    #[serde(default)]
    pub datum_endpoint: String,
    #[serde(default)]
    pub pubkey_hex: String,
    #[serde(default)]
    pub fee_bps: u32,
    #[serde(default)]
    pub self_reported_hs: f64,
    /// Soft % of network hashrate (operator or directory supplied).
    #[serde(default)]
    pub network_share_pct: f64,
    #[serde(default = "default_true")]
    pub opt_in: bool,
    /// How much to trust empty-tag2 ⇒ SV1 heuristics for this peer's blocks.
    #[serde(default = "default_tag_confidence")]
    pub tag_confidence: f32,
    #[serde(default)]
    pub notes: String,
}

fn default_true() -> bool {
    true
}
fn default_tag_confidence() -> f32 {
    0.5
}

/// Sticky assignment so an address does not hop every reconnect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub payout_address: String,
    pub peer_id: String,
    pub backend: String,
    pub bound_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub reason: String,
}

/// Riptide-shaped (and friends) pool self-report for the directory / agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PoolReport {
    pub peer_id: String,
    #[serde(default)]
    pub network_hashrate_hs: Option<f64>,
    #[serde(default)]
    pub hashrate_hs: Option<f64>,
    #[serde(default)]
    pub hashrate_hs_1h: Option<f64>,
    #[serde(default)]
    pub hashrate_hs_24h: Option<f64>,
    #[serde(default)]
    pub active_sv1_addrs: Option<u64>,
    #[serde(default)]
    pub active_datum_addrs: Option<u64>,
    #[serde(default)]
    pub fee_bps: Option<u32>,
    #[serde(default)]
    pub blocks_last_24h: Option<u64>,
    #[serde(default)]
    pub reported_at: Option<DateTime<Utc>>,
}

/// Overflow policy knobs (local agent and/or directory defaults).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criteria {
    /// Do not overflow to peers advertising above this fee.
    pub max_fee_bps: u32,
    /// Soft enter threshold: house SV1 / network share (percent).
    pub enter_network_share_pct: f64,
    /// Soft exit threshold (hysteresis).
    pub exit_network_share_pct: f64,
    pub lease_hours: u64,
    /// Prefer overflowing addresses classified as New.
    #[serde(default = "default_true")]
    pub overflow_new_addresses: bool,
}

impl Default for Criteria {
    fn default() -> Self {
        Self {
            max_fee_bps: 200,
            enter_network_share_pct: 15.0,
            exit_network_share_pct: 12.0,
            lease_hours: 24,
            overflow_new_addresses: true,
        }
    }
}

/// Soft classification for overflow eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressClass {
    /// Friend-pin / prefer — stay local; counts against local pie.
    Pinned,
    /// Seen recently in shares / coinbases — prefer keep.
    Known,
    /// Not seen — easier to overflow.
    New,
}

/// Where a new session should go.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionDecision {
    KeepLocal { reason: String },
    Overflow {
        peer_id: String,
        backend: String,
        reason: String,
    },
}

/// Hook for later DATUM-plane migrate (Luke 0xA4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatumMigrationHint {
    pub home_peer_id: String,
    pub temporary_peer_id: String,
    pub return_home_after_secs: u64,
    pub note: String,
}
