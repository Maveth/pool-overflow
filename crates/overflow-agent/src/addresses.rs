//! Soft address classification: pinned / known / new.

use chrono::{DateTime, Duration, Utc};
use overflow_core::AddressClass;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AddressBook {
    /// Explicit friend / contract pins (stay local; count against pie later).
    #[serde(default)]
    pub pinned: HashSet<String>,
    /// address -> last_seen
    #[serde(default)]
    pub known: HashMap<String, DateTime<Utc>>,
    /// How long "known" lasts without refresh.
    #[serde(default = "default_known_hours")]
    pub known_ttl_hours: i64,
}

fn default_known_hours() -> i64 {
    72
}

impl AddressBook {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self {
                known_ttl_hours: default_known_hours(),
                ..Self::default()
            });
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
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

    pub fn classify(&self, address: &str) -> AddressClass {
        if self.pinned.contains(address) {
            return AddressClass::Pinned;
        }
        let now = Utc::now();
        if let Some(seen) = self.known.get(address) {
            if *seen + Duration::hours(self.known_ttl_hours) > now {
                return AddressClass::Known;
            }
        }
        AddressClass::New
    }

    pub fn note_seen(&mut self, address: &str) {
        self.known.insert(address.to_string(), Utc::now());
    }
}
