//! Light status HTTP for the local agent.

use crate::gw::{self, MatchMode};
use crate::state::AppState;
use crate::stratum::payout_address_from_username;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use overflow_core::AddressClass;
use serde::Deserialize;
use std::sync::Arc;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(page_home))
        .route("/api/status", get(api_status))
        .route("/api/lease", get(api_lease))
        .route("/api/address/class", get(api_class))
        .route("/api/address/pin", post(api_pin))
        .route("/api/gw/migrate", post(api_gw_migrate))
        .with_state(state)
}

async fn page_home(State(state): State<Arc<AppState>>) -> Html<String> {
    let m = state.metrics.snapshot();
    Html(format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"/><title>overflow-agent</title>
<style>body{{font-family:system-ui,sans-serif;margin:2rem;max-width:900px}}
code{{background:#f4f4f5;padding:.1rem .3rem}} .card{{border:1px solid #ddd;border-radius:8px;padding:1rem;margin:1rem 0}}</style>
</head><body>
<h1>overflow-agent</h1>
<p>Pool-local SV1 valve. Federation directory optional.</p>
<div class="card">
<p>Listen SV1 <code>{}</code> · HTTP <code>{}</code></p>
<p>Local backend <code>{}</code> · house SV1 share <b>{:.2}%</b> · enter <b>{:.2}%</b></p>
<p>Metrics: connects={} authorizes={} keep_local={} overflowed={} active={}</p>
<p><a href="/api/status">JSON status</a></p>
</div>
</body></html>"#,
        state.config.listen_sv1,
        state.config.listen_http,
        state.config.local_sv1_backend,
        state.config.house_sv1_network_share_pct,
        state.config.criteria.enter_network_share_pct,
        m.connects,
        m.authorizes,
        m.keep_local,
        m.overflowed,
        m.active
    ))
}

async fn api_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let leases = state.leases.read().await;
    let now = Utc::now();
    let active = leases
        .leases
        .values()
        .filter(|l| l.expires_at > now)
        .count();
    Json(serde_json::json!({
        "listen_sv1": state.config.listen_sv1.to_string(),
        "listen_http": state.config.listen_http.to_string(),
        "local_sv1_backend": state.config.local_sv1_backend.to_string(),
        "house_sv1_network_share_pct": state.config.house_sv1_network_share_pct,
        "criteria": state.config.criteria,
        "peers": state.config.peers,
        "active_leases": active,
        "metrics": state.metrics.snapshot(),
        "federation_directory_url": state.config.federation_directory_url,
        "gw_doors_path": state.config.gw_doors_path.as_ref().map(|p| p.display().to_string()),
        "gw_migrate": "POST /api/gw/migrate {from,to,identity,match,dry_run}",
    }))
}

#[derive(Debug, Deserialize)]
struct AddrQuery {
    address: String,
}

async fn api_lease(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AddrQuery>,
) -> impl IntoResponse {
    let addr = payout_address_from_username(&q.address);
    let now = Utc::now();
    let leases = state.leases.read().await;
    Json(serde_json::json!({
        "address": addr,
        "class": format!("{:?}", state.addresses.read().await.classify(&addr)),
        "current": leases.active(&addr, now),
        "history": leases.history_for(&addr, 25),
    }))
}

async fn api_class(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AddrQuery>,
) -> impl IntoResponse {
    let addr = payout_address_from_username(&q.address);
    let class = state.classify(&addr).await;
    Json(serde_json::json!({ "address": addr, "class": class }))
}

#[derive(Debug, Deserialize)]
struct PinBody {
    address: String,
    #[serde(default = "default_true")]
    pinned: bool,
}

fn default_true() -> bool {
    true
}

async fn api_pin(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PinBody>,
) -> impl IntoResponse {
    let addr = payout_address_from_username(&body.address);
    {
        let mut book = state.addresses.write().await;
        if body.pinned {
            book.pinned.insert(addr.clone());
        } else {
            book.pinned.remove(&addr);
        }
    }
    let _ = state.persist().await;
    Json(serde_json::json!({
        "ok": true,
        "address": addr,
        "class": AddressClass::Pinned,
        "pinned": body.pinned
    }))
}

#[derive(Debug, Deserialize)]
struct GwMigrateBody {
    /// Source door id from doors.toml (e.g. "M").
    from: String,
    /// Dest door id (required unless dry_run).
    #[serde(default)]
    to: Option<String>,
    /// Payout address or address.worker.
    identity: String,
    /// "exact" or "address" (address.* / all workers on payout).
    #[serde(default = "default_match_address")]
    #[serde(rename = "match")]
    match_mode: String,
    #[serde(default)]
    dry_run: bool,
}

fn default_match_address() -> String {
    "address".into()
}

async fn api_gw_migrate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GwMigrateBody>,
) -> impl IntoResponse {
    let Some(doors_path) = state.config.gw_doors_path.as_ref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "gw_doors_path not configured in agent.toml"
            })),
        )
            .into_response();
    };
    let Some(password) = state.config.gw_password() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "gw admin password not configured (gw_admin_password_env / gw_admin_password)"
            })),
        )
            .into_response();
    };
    let mode = match MatchMode::parse(&body.match_mode) {
        Ok(m) => m,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "ok": false, "error": err.to_string() })),
            )
                .into_response();
        }
    };
    let doors = match gw::load_doors(doors_path) {
        Ok(d) => d,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "ok": false, "error": err.to_string() })),
            )
                .into_response();
        }
    };
    match gw::migrate_by_address(
        &doors,
        &body.from,
        body.to.as_deref(),
        &body.identity,
        mode,
        &password,
        body.dry_run,
    )
    .await
    {
        Ok(report) => Json(serde_json::json!({ "ok": true, "report": report })).into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "ok": false, "error": err.to_string() })),
        )
            .into_response(),
    }
}
