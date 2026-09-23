//! Session keep-vs-overflow policy (pure).

use crate::config::AgentConfig;
use overflow_core::{AddressClass, Peer, SessionDecision};

pub fn decide_session(
    cfg: &AgentConfig,
    _payout_address: &str,
    class: AddressClass,
    house_sv1_network_share_pct: f64,
) -> SessionDecision {
    if matches!(class, AddressClass::Pinned) {
        return SessionDecision::KeepLocal {
            reason: "pinned address".into(),
        };
    }

    if house_sv1_network_share_pct < cfg.criteria.enter_network_share_pct {
        return SessionDecision::KeepLocal {
            reason: format!(
                "house SV1 share {:.2}% < enter {:.2}%",
                house_sv1_network_share_pct, cfg.criteria.enter_network_share_pct
            ),
        };
    }

    if matches!(class, AddressClass::Known) && cfg.criteria.overflow_new_addresses {
        return SessionDecision::KeepLocal {
            reason: "known address; overflow prefers new".into(),
        };
    }

    match pick_overflow_peer(cfg) {
        Some(peer) => SessionDecision::Overflow {
            peer_id: peer.id.clone(),
            backend: peer.sv1_backend.clone(),
            reason: format!(
                "house SV1 share {:.2}% >= enter {:.2}%; class={class:?}",
                house_sv1_network_share_pct, cfg.criteria.enter_network_share_pct
            ),
        },
        None => SessionDecision::KeepLocal {
            reason: "over threshold but no eligible peer".into(),
        },
    }
}

fn pick_overflow_peer(cfg: &AgentConfig) -> Option<&Peer> {
    let mut eligible: Vec<&Peer> = cfg
        .peers
        .iter()
        .filter(|p| p.opt_in && p.fee_bps <= cfg.criteria.max_fee_bps)
        .filter(|p| !p.sv1_backend.is_empty())
        .collect();
    if eligible.is_empty() {
        return None;
    }
    eligible.sort_by(|a, b| {
        a.network_share_pct
            .partial_cmp(&b.network_share_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.self_reported_hs
                    .partial_cmp(&b.self_reported_hs)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then_with(|| a.id.cmp(&b.id))
    });
    eligible.first().copied()
}
