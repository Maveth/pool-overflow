//! Pool-local overflow agent.
//!
//! Stage 1: load peers + criteria, classify addresses, decide keep vs overflow.
//! SV1 authorize-peek + line-pump ports in next (from sv1-federation patterns).
//! DATUM 0xA4 migrate is a later hook — see docs/0xA4.md.

mod config;
mod decide;

use crate::config::AgentConfig;
use clap::Parser;
use overflow_core::{AddressClass, SessionDecision};
use std::path::PathBuf;
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

    tracing::info!(
        peers = cfg.peers.len(),
        max_fee_bps = cfg.criteria.max_fee_bps,
        enter_pct = cfg.criteria.enter_network_share_pct,
        listen = %cfg.listen_sv1,
        local_backend = %cfg.local_sv1_backend,
        directory = ?cfg.federation_directory_url,
        "overflow-agent starting (decision engine ready; pump listener next)"
    );

    // Smoke the decision path so the binary is useful before the pump lands.
    let example = decide::decide_session(
        &cfg,
        "bc1qexample00000000000000000000000000000",
        AddressClass::New,
        /* house_sv1_network_share_pct */ 18.0,
    );
    match &example {
        SessionDecision::KeepLocal { reason } => {
            tracing::info!(%reason, "example decision: keep local");
        }
        SessionDecision::Overflow {
            peer_id,
            backend,
            reason,
        } => {
            tracing::info!(%peer_id, %backend, %reason, "example decision: overflow");
        }
    }

    tracing::info!(
        "TODO: bind {} authorize-peek + pump (see sv1-federation session.rs); \
         DATUM path: follow 0xA4 when gateway supports it",
        cfg.listen_sv1
    );

    // Keep process alive lightly for lab supervision; replace with real listener soon.
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown");
    Ok(())
}
