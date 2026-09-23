//! Shared agent state.

use crate::addresses::AddressBook;
use crate::config::AgentConfig;
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
}

impl AppState {
    pub fn new(config: AgentConfig, leases: LeaseStore, addresses: AddressBook) -> Arc<Self> {
        Arc::new(Self {
            config,
            leases: RwLock::new(leases),
            addresses: RwLock::new(addresses),
            metrics: Metrics::default(),
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
    pub async fn resolve_route(
        &self,
        payout_address: &str,
    ) -> anyhow::Result<(String, SocketAddr, SessionDecision)> {
        let now = Utc::now();
        {
            let leases = self.leases.read().await;
            if let Some(lease) = leases.active(payout_address, now) {
                let backend: SocketAddr = lease.backend.parse()?;
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
                return Ok((lease.peer_id.clone(), backend, decision));
            }
        }

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
        let backend: SocketAddr = backend_str.parse()?;

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
}
