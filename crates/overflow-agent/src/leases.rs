//! Local sticky leases for overflow destinations.

use chrono::{DateTime, Duration, Utc};
use overflow_core::Lease;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseEvent {
    pub at: DateTime<Utc>,
    pub payout_address: String,
    pub peer_id: String,
    pub backend: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LeaseStore {
    pub leases: HashMap<String, Lease>,
    #[serde(default)]
    pub history: Vec<LeaseEvent>,
}

impl LeaseStore {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn active(&self, address: &str, now: DateTime<Utc>) -> Option<&Lease> {
        self.leases
            .get(address)
            .filter(|lease| lease.expires_at > now)
    }

    pub fn bind_or_renew(
        &mut self,
        payout_address: &str,
        peer_id: &str,
        backend: &str,
        lease_hours: u64,
        reason: &str,
    ) -> Lease {
        let now = Utc::now();
        let kind = if self.leases.contains_key(payout_address) {
            "renew"
        } else {
            "bind"
        };
        let lease = Lease {
            payout_address: payout_address.to_string(),
            peer_id: peer_id.to_string(),
            backend: backend.to_string(),
            bound_at: now,
            expires_at: now + Duration::hours(lease_hours as i64),
            reason: reason.to_string(),
        };
        self.history.push(LeaseEvent {
            at: now,
            payout_address: payout_address.to_string(),
            peer_id: peer_id.to_string(),
            backend: backend.to_string(),
            kind: kind.to_string(),
            detail: reason.to_string(),
        });
        if self.history.len() > 500 {
            let n = self.history.len() - 500;
            self.history.drain(0..n);
        }
        self.leases
            .insert(payout_address.to_string(), lease.clone());
        lease
    }

    pub fn history_for(&self, address: &str, limit: usize) -> Vec<LeaseEvent> {
        self.history
            .iter()
            .rev()
            .filter(|e| e.payout_address == address)
            .take(limit)
            .cloned()
            .collect()
    }
}
