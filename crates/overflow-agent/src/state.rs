//! Shared agent state.

use crate::addresses::AddressBook;
use crate::config::AgentConfig;
use crate::health::HealthCache;
use crate::leases::LeaseStore;
use crate::metrics::Metrics;
use chrono::Utc;
use overflow_core::{AddressClass, SessionDecision};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: AgentConfig,
    pub leases: RwLock<LeaseStore>,
    pub addresses: RwLock<AddressBook>,
    pub metrics: Metrics,
    pub health: HealthCache,
}

impl AppState {
    pub fn new(config: AgentConfig, leases: LeaseStore, addresses: AddressBook) -> Arc<Self> {
        Arc::new(Self {
            config,
            leases: RwLock::new(leases),
            addresses: RwLock::new(addresses),
            metrics: Metrics::default(),
            // 30s TTL — gentle on live backends like rental GW J
            health: HealthCache::new(30),
        })
    }

    pub async fn persist(&self) -> anyhow::Result<()> {
        {
            let book = self.leases.read().await;
            book.save(&self.config.leases_path)?;
        }
        {
            let addrs = self.addresses.read().await;
            addrs.save(&self.config.addresses_path)?;
        }
        Ok(())
    }

    pub async fn classify(&self, address: &str) -> AddressClass {
        self.addresses.read().await.classify(address)
    }

    pub async fn note_seen(&self, address: &str) {
        let mut book = self.addresses.write().await;
        book.note_seen(address);
    }

    /// Resolve TCP backend for this payout address using lease + policy.
    /// If the leased/chosen backend fails a light health check, failover once
    /// (destination down is an allowed mid-lease exception).
    pub async fn resolve_route(
        &self,
        payout_address: &str,
    ) -> anyhow::Result<(String, SocketAddr, SessionDecision)> {
        let now = Utc::now();
        let mut from_lease = false;
        let (mut peer_id, mut backend_str, mut decision) = {
            let leases = self.leases.read().await;
            if let Some(lease) = leases.active(payout_address, now) {
                from_lease = true;
                let decision = if lease.peer_id == self.config.local_peer_id
                    || lease.backend == self.config.local_sv1_backend.to_string()
                {
                    SessionDecision::KeepLocal {
                        reason: "active lease local".into(),
                    }
                } else {
                    SessionDecision::Overflow {
                        peer_id: lease.peer_id.clone(),
                        backend: lease.backend.clone(),
                        reason: "active lease overflow".into(),
                    }
                };
                (lease.peer_id.clone(), lease.backend.clone(), decision)
            } else {
                drop(leases);
                let class = self.classify(payout_address).await;
                let decision = crate::decide::decide_session(
                    &self.config,
                    payout_address,
                    class,
                    self.config.house_sv1_network_share_pct,
                );
                let (peer_id, backend_str) = match &decision {
                    SessionDecision::KeepLocal { .. } => (
                        self.config.local_peer_id.clone(),
                        self.config.local_sv1_backend.to_string(),
                    ),
                    SessionDecision::Overflow {
                        peer_id, backend, ..
                    } => (peer_id.clone(), backend.clone()),
                };
                (peer_id, backend_str, decision)
            }
        };

        let mut backend: SocketAddr = backend_str.parse()?;
        if !self.health.is_healthy(backend).await {
            // Failover: try the other side once (local <-> first eligible peer).
            let (alt_id, alt_backend) = self.failover_alternative(&peer_id)?;
            if self.health.is_healthy(alt_backend).await {
                peer_id = alt_id;
                backend = alt_backend;
                backend_str = backend.to_string();
                decision = if peer_id == self.config.local_peer_id {
                    SessionDecision::KeepLocal {
                        reason: format!(
                            "failover to local (prior unhealthy; from_lease={from_lease})"
                        ),
                    }
                } else {
                    SessionDecision::Overflow {
                        peer_id: peer_id.clone(),
                        backend: backend_str.clone(),
                        reason: format!(
                            "failover to peer (prior unhealthy; from_lease={from_lease})"
                        ),
                    }
                };
            } else {
                anyhow::bail!("no healthy backend for {payout_address}");
            }
        }

        {
            let mut leases = self.leases.write().await;
            leases.bind_or_renew(
                payout_address,
                &peer_id,
                &backend_str,
                self.config.criteria.lease_hours,
                match &decision {
                    SessionDecision::KeepLocal { reason } => reason,
                    SessionDecision::Overflow { reason, .. } => reason,
                },
            );
        }
        let _ = self.persist().await;
        Ok((peer_id, backend, decision))
    }

    fn failover_alternative(&self, current_peer_id: &str) -> anyhow::Result<(String, SocketAddr)> {
        if current_peer_id != self.config.local_peer_id {
            return Ok((
                self.config.local_peer_id.clone(),
                self.config.local_sv1_backend,
            ));
        }
        let peer = self
            .config
            .peers
            .iter()
            .filter(|p| p.opt_in && p.fee_bps <= self.config.criteria.max_fee_bps)
            .filter(|p| !p.sv1_backend.is_empty())
            .min_by(|a, b| {
                a.network_share_pct
                    .partial_cmp(&b.network_share_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| anyhow::anyhow!("no overflow peer for failover"))?;
        Ok((peer.id.clone(), peer.sv1_backend.parse()?))
    }
}
