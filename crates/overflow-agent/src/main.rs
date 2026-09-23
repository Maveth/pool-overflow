//! Pool-local overflow agent — Stage 1 SV1 valve.
//!
//! Works with local peers only. Optional federation-directory URL for later.
//! DATUM 0xA4 migrate is documented, not implemented here yet.

mod addresses;
mod config;
mod decide;
mod leases;
mod metrics;
mod session;
mod state;
mod stratum;
mod web;

use crate::addresses::AddressBook;
use crate::config::AgentConfig;
use crate::leases::LeaseStore;
use crate::state::AppState;
use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "overflow-agent",
    about = "Pool-local SV1 overflow valve (federation directory optional)"
)]
struct Args {
    #[arg(short, long, default_value = "agent.toml", env = "OVERFLOW_AGENT_CONFIG")]
    config: PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("overflow_agent=info".parse()?),
        )
        .init();

    let args = Args::parse();
    let cfg = AgentConfig::load(&args.config)?;
    let leases = LeaseStore::load(&cfg.leases_path).unwrap_or_default();
    let addresses = AddressBook::load(&cfg.addresses_path).unwrap_or_else(|_| AddressBook {
        known_ttl_hours: 72,
        ..AddressBook::default()
    });
    let state = AppState::new(cfg, leases, addresses);

    tracing::info!(
        listen_sv1 = %state.config.listen_sv1,
        listen_http = %state.config.listen_http,
        local = %state.config.local_sv1_backend,
        house_pct = state.config.house_sv1_network_share_pct,
        peers = state.config.peers.len(),
        "overflow-agent starting"
    );

    let session_state = Arc::clone(&state);
    let session_task = tokio::spawn(async move {
        if let Err(err) = session::run_listener(session_state).await {
            tracing::error!(error = %err, "SV1 listener exited");
        }
    });

    let app = web::router(Arc::clone(&state));
    let listener = tokio::net::TcpListener::bind(state.config.listen_http)
        .await
        .with_context(|| format!("bind http {}", state.config.listen_http))?;
    tracing::info!(addr = %state.config.listen_http, "agent HTTP bound");

    let http_task = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            tracing::error!(error = %err, "http exited");
        }
    });

    tokio::select! {
        _ = session_task => {}
        _ = http_task => {}
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("ctrl-c");
        }
    }

    let _ = state.persist().await;
    Ok(())
}
