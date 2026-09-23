//! Optional federation directory: peer list + pool reports for agents to pull.
//!
//! Not miner-facing. Agents work fine without this service.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use clap::Parser;
use overflow_core::{Criteria, Peer, PoolReport};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "federation-directory")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:29880", env = "FED_DIRECTORY_LISTEN")]
    listen: SocketAddr,
    #[arg(long, default_value = "directory.toml")]
    config: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct DirectoryConfig {
    #[serde(default)]
    peers: Vec<Peer>,
    #[serde(default)]
    criteria: Criteria,
}

#[derive(Debug, Default)]
struct DirState {
    peers: Vec<Peer>,
    criteria: Criteria,
    reports: Vec<PoolReport>,
}

#[derive(Debug, Serialize)]
struct Snapshot {
    criteria: Criteria,
    peers: Vec<Peer>,
    reports: Vec<PoolReport>,
    note: &'static str,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("federation_directory=info".parse()?),
        )
        .init();

    let args = Args::parse();
    let file = std::fs::read_to_string(&args.config).unwrap_or_default();
    let cfg: DirectoryConfig = if file.trim().is_empty() {
        DirectoryConfig {
            peers: vec![],
            criteria: Criteria::default(),
        }
    } else {
        toml::from_str(&file)?
    };

    let state = Arc::new(RwLock::new(DirState {
        peers: cfg.peers,
        criteria: cfg.criteria,
        reports: vec![],
    }));

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/snapshot", get(api_snapshot))
        .route("/api/report", post(api_report))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    tracing::info!(%args.listen, "federation-directory listening (backend tracking only)");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn api_snapshot(State(state): State<Arc<RwLock<DirState>>>) -> Json<Snapshot> {
    let g = state.read().await;
    Json(Snapshot {
        criteria: g.criteria.clone(),
        peers: g.peers.clone(),
        reports: g.reports.clone(),
        note: "Optional coordination plane. overflow-agent works with local peers.toml alone.",
    })
}

async fn api_report(
    State(state): State<Arc<RwLock<DirState>>>,
    Json(mut report): Json<PoolReport>,
) -> Json<serde_json::Value> {
    if report.reported_at.is_none() {
        report.reported_at = Some(Utc::now());
    }
    let mut g = state.write().await;
    if let Some(existing) = g
        .reports
        .iter_mut()
        .find(|r| r.peer_id == report.peer_id)
    {
        *existing = report.clone();
    } else {
        g.reports.push(report.clone());
    }
    // Soft-update peer self_reported_hs if peer exists.
    if let Some(hs) = report.hashrate_hs {
        if let Some(peer) = g.peers.iter_mut().find(|p| p.id == report.peer_id) {
            peer.self_reported_hs = hs;
        }
    }
    if let Some(fee) = report.fee_bps {
        if let Some(peer) = g.peers.iter_mut().find(|p| p.id == report.peer_id) {
            peer.fee_bps = fee;
        }
    }
    Json(serde_json::json!({"ok": true}))
}
