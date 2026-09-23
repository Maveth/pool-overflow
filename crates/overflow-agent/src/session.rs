//! Authorize-peek + opaque pump (Lazarus-style).

use crate::state::AppState;
use crate::stratum::{authorize_username, payout_address_from_username, username_looks_payable};
use overflow_core::SessionDecision;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{info, warn};

pub async fn run_listener(state: Arc<AppState>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(state.config.listen_sv1).await?;
    info!(addr = %state.config.listen_sv1, "overflow-agent SV1 listener bound");
    let limit = Arc::new(Semaphore::new(state.config.max_forward_connections));

    loop {
        let (inbound, peer) = listener.accept().await?;
        let permit = match limit.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                warn!(%peer, "max connections; refuse");
                continue;
            }
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = run_session(state, inbound, peer).await {
                warn!(%peer, error = %err, "session error");
            }
        });
        tokio::task::yield_now().await;
    }
}

async fn run_session(
    state: Arc<AppState>,
    inbound: TcpStream,
    peer: SocketAddr,
) -> anyhow::Result<()> {
    if state.config.tcp_nodelay {
        let _ = inbound.set_nodelay(true);
    }
    state.metrics.connects.fetch_add(1, Ordering::Relaxed);
    state.metrics.active.fetch_add(1, Ordering::Relaxed);

    let wait = Duration::from_millis(state.config.authorize_wait_ms.max(100));
    let (buffered, username, mut inbound) = peek_for_authorize(inbound, wait).await?;

    let sticky = match username {
        Some(ref user) => {
            state.metrics.authorizes.fetch_add(1, Ordering::Relaxed);
            if username_looks_payable(user) {
                payout_address_from_username(user)
            } else {
                format!("ip:{}", peer.ip())
            }
        }
        None => format!("ip:{}", peer.ip()),
    };

    // Classify before note_seen so first-seen addresses stay New for this decision.
    let (peer_id, backend, decision) = state.resolve_route(&sticky).await?;
    if sticky.starts_with("bc1") || sticky.starts_with('1') || sticky.starts_with('3') {
        state.note_seen(&sticky).await;
    }
    match &decision {
        SessionDecision::KeepLocal { reason } => {
            state.metrics.keep_local.fetch_add(1, Ordering::Relaxed);
            info!(%peer, %sticky, %peer_id, %backend, %reason, "keep local");
        }
        SessionDecision::Overflow {
            peer_id,
            backend,
            reason,
        } => {
            state.metrics.overflowed.fetch_add(1, Ordering::Relaxed);
            info!(%peer, %sticky, %peer_id, %backend, %reason, "overflow");
        }
    }

    // Health already checked inside resolve_route (with one failover attempt).
    let mut outbound = match TcpStream::connect(backend).await {
        Ok(s) => s,
        Err(err) => {
            warn!(%peer, %backend, error = %err, "connect failed after health check");
            state.metrics.disconnects.fetch_add(1, Ordering::Relaxed);
            state.metrics.active.fetch_sub(1, Ordering::Relaxed);
            return Err(err.into());
        }
    };
    if state.config.tcp_nodelay {
        let _ = outbound.set_nodelay(true);
    }
    if !buffered.is_empty() {
        outbound.write_all(&buffered).await?;
        outbound.flush().await?;
    }

    let pump = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
    state.metrics.disconnects.fetch_add(1, Ordering::Relaxed);
    state.metrics.active.fetch_sub(1, Ordering::Relaxed);
    match pump {
        Ok((up, down)) => {
            info!(%peer, up, down, "session closed");
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

async fn peek_for_authorize(
    inbound: TcpStream,
    budget: Duration,
) -> anyhow::Result<(Vec<u8>, Option<String>, TcpStream)> {
    let deadline = Instant::now() + budget;
    let mut reader = BufReader::new(inbound);
    let mut buffered = Vec::new();
    let mut username = None;
    let mut line = String::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        line.clear();
        match tokio::time::timeout(remaining, reader.read_line(&mut line)).await {
            Err(_) => break,
            Ok(Ok(0)) => break,
            Ok(Ok(_)) => {
                buffered.extend_from_slice(line.as_bytes());
                if let Some(user) = authorize_username(&line) {
                    username = Some(user);
                    break;
                }
                if buffered.len() > 16 * 1024 {
                    break;
                }
            }
            Ok(Err(err)) => return Err(err.into()),
        }
    }
    Ok((buffered, username, reader.into_inner()))
}
