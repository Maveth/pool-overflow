//! Light TCP health cache — avoid hammering live backends (e.g. rental GW J).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy)]
struct Entry {
    ok: bool,
    checked_at: Instant,
}

#[derive(Debug, Default)]
pub struct HealthCache {
    inner: Mutex<HashMap<SocketAddr, Entry>>,
    ttl: Duration,
    connect_timeout: Duration,
}

impl HealthCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs.max(5)),
            connect_timeout: Duration::from_millis(400),
        }
    }

    pub async fn is_healthy(&self, addr: SocketAddr) -> bool {
        if let Ok(guard) = self.inner.lock() {
            if let Some(entry) = guard.get(&addr) {
                if entry.checked_at.elapsed() < self.ttl {
                    return entry.ok;
                }
            }
        }

        let ok = matches!(
            timeout(self.connect_timeout, TcpStream::connect(addr)).await,
            Ok(Ok(_))
        );

        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(
                addr,
                Entry {
                    ok,
                    checked_at: Instant::now(),
                },
            );
        }
        ok
    }
}
