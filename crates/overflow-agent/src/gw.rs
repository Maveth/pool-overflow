//! Call DATUM Gateway admin APIs: list clients, migrate by payout address.
//!
//! Destination `host:port` comes from the door registry — never from `rem_host`
//! (useless behind HAP/socat). Policy that *chooses* when/where lives elsewhere;
//! this module only executes list + move.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Door {
    pub host: String,
    pub port: u16,
    pub api: String,
    #[serde(default)]
    pub stratum_internal: Option<u16>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DoorsFile {
    doors: BTreeMap<String, Door>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Exact,
    Address,
}

impl MatchMode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "exact" => Ok(Self::Exact),
            "address" | "prefix" => Ok(Self::Address),
            other => bail!("unknown match mode {other}"),
        }
    }
}

pub fn normalize_address(username: &str) -> &str {
    username.split('.').next().unwrap_or(username)
}

fn matches(username: &str, identity: &str, mode: MatchMode) -> bool {
    let u = username.trim();
    let ident = identity.trim();
    match mode {
        MatchMode::Exact => u == ident,
        MatchMode::Address => {
            normalize_address(u).eq_ignore_ascii_case(normalize_address(ident))
        }
    }
}

pub fn load_doors(path: &Path) -> Result<BTreeMap<String, Door>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read doors {}", path.display()))?;
    let parsed: DoorsFile = toml::from_str(&raw).context("parse doors.toml")?;
    if parsed.doors.is_empty() {
        bail!("no [doors.*] entries in {}", path.display());
    }
    Ok(parsed.doors)
}

async fn curl_json(args: &[&str]) -> Result<(u16, String)> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-m", "15"]).args(args).arg("-w").arg("\n__HTTP__%{http_code}");
    let out = cmd.output().await.context("run curl")?;
    if !out.status.success() && out.stdout.is_empty() {
        bail!(
            "curl failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let Some((body, code)) = text.rsplit_once("__HTTP__") else {
        bail!("curl missing http code trailer: {text}");
    };
    let code: u16 = code.trim().parse().unwrap_or(0);
    Ok((code, body.to_string()))
}

pub async fn fetch_clients(api: &str, password: &str) -> Result<Value> {
    let url = format!("{}/clients.json", api.trim_end_matches('/'));
    let (code, body) = curl_json(&[
        "--digest",
        "-u",
        &format!("admin:{password}"),
        &url,
    ])
    .await?;
    if code != 200 {
        bail!("clients.json HTTP {code}: {}", body.chars().take(300).collect::<String>());
    }
    serde_json::from_str(&body).context("parse clients.json")
}

pub async fn post_migrate_client(
    api: &str,
    password: &str,
    tid: i64,
    cid: i64,
    host: &str,
    port: u16,
) -> Result<()> {
    let url = format!("{}/cmd", api.trim_end_matches('/'));
    let payload = serde_json::json!({
        "cmd": "migrate_client",
        "tid": tid,
        "cid": cid,
        "host": host,
        "port": port,
        "password": password,
    });
    let (code, body) = curl_json(&[
        "-H",
        "Content-Type: application/json",
        "-d",
        &payload.to_string(),
        &url,
    ])
    .await?;
    if code != 200 {
        bail!("migrate_client HTTP {code}: {}", body.chars().take(300).collect::<String>());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchedClient {
    pub tid: i64,
    pub cid: i64,
    pub username: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrateReport {
    pub src: String,
    pub dst: Option<String>,
    pub identity: String,
    pub match_mode: String,
    pub matched: Vec<MatchedClient>,
    pub migrated: Vec<MatchedClient>,
    pub dry_run: bool,
}

pub async fn migrate_by_address(
    doors: &BTreeMap<String, Door>,
    src_id: &str,
    dst_id: Option<&str>,
    identity: &str,
    mode: MatchMode,
    password: &str,
    dry_run: bool,
) -> Result<MigrateReport> {
    let src = doors
        .get(src_id)
        .with_context(|| format!("unknown source door {src_id}"))?;
    let clients = fetch_clients(&src.api, password).await?;
    let list = clients
        .get("clients")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let mut matched = Vec::new();
    for c in list {
        let username = c
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !matches(&username, identity, mode) {
            continue;
        }
        let tid = c.get("tid").and_then(|v| v.as_i64()).unwrap_or(-1);
        let cid = c.get("cid").and_then(|v| v.as_i64()).unwrap_or(-1);
        if tid < 0 || cid < 0 {
            continue;
        }
        matched.push(MatchedClient {
            tid,
            cid,
            username,
        });
    }

    let mut report = MigrateReport {
        src: src_id.to_string(),
        dst: dst_id.map(|s| s.to_string()),
        identity: identity.to_string(),
        match_mode: match mode {
            MatchMode::Exact => "exact".into(),
            MatchMode::Address => "address".into(),
        },
        matched: matched.clone(),
        migrated: Vec::new(),
        dry_run,
    };

    if dry_run || matched.is_empty() {
        return Ok(report);
    }

    let dst_id = dst_id.context("--to / dst required unless dry_run")?;
    let dst = doors
        .get(dst_id)
        .with_context(|| format!("unknown dest door {dst_id}"))?;

    for m in &matched {
        post_migrate_client(&src.api, password, m.tid, m.cid, &dst.host, dst.port).await?;
        tracing::info!(
            tid = m.tid,
            cid = m.cid,
            username = %m.username,
            dest = %format!("{}:{}", dst.host, dst.port),
            "gw migrate_client sent"
        );
        report.migrated.push(m.clone());
    }
    Ok(report)
}
