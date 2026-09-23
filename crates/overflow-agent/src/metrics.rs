use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    pub connects: AtomicU64,
    pub disconnects: AtomicU64,
    pub active: AtomicU64,
    pub authorizes: AtomicU64,
    pub keep_local: AtomicU64,
    pub overflowed: AtomicU64,
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub connects: u64,
    pub disconnects: u64,
    pub active: u64,
    pub authorizes: u64,
    pub keep_local: u64,
    pub overflowed: u64,
}

impl Metrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connects: self.connects.load(Ordering::Relaxed),
            disconnects: self.disconnects.load(Ordering::Relaxed),
            active: self.active.load(Ordering::Relaxed),
            authorizes: self.authorizes.load(Ordering::Relaxed),
            keep_local: self.keep_local.load(Ordering::Relaxed),
            overflowed: self.overflowed.load(Ordering::Relaxed),
        }
    }
}
