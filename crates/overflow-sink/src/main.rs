//! Minimal Stratum sink for lab overflow-path tests.
//! Speaks just enough SV1 that a pump session looks alive. Not a real pool.

use clap::Parser;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "overflow-sink")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:29792")]
    listen: SocketAddr,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("overflow_sink=info".parse()?),
        )
        .init();
    let args = Args::parse();
    let listener = TcpListener::bind(args.listen).await?;
    info!(%args.listen, "overflow-sink listening (lab only)");

    loop {
        let (sock, peer) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = handle(sock, peer).await {
                warn!(%peer, error = %err, "sink session end");
            }
        });
        tokio::task::yield_now().await;
    }
}

async fn handle(sock: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    let (reader, mut writer) = sock.into_split();
    let mut lines = BufReader::new(reader).lines();
    info!(%peer, "sink client connected");

    while let Some(line) = lines.next_line().await? {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let id = v.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
        if method.eq_ignore_ascii_case("mining.subscribe") {
            let reply = serde_json::json!({
                "id": id,
                "error": null,
                "result": [[["mining.notify","sink1"],["mining.set_difficulty","sink2"]], "sinkxn", 8]
            });
            writer
                .write_all(format!("{reply}\n").as_bytes())
                .await?;
            let diff = serde_json::json!({
                "id": null,
                "method": "mining.set_difficulty",
                "params": [1024.0]
            });
            writer.write_all(format!("{diff}\n").as_bytes()).await?;
            writer.flush().await?;
        } else if method.eq_ignore_ascii_case("mining.authorize") {
            let user = v
                .get("params")
                .and_then(|p| p.as_array())
                .and_then(|a| a.first())
                .and_then(|u| u.as_str())
                .unwrap_or("");
            info!(%peer, %user, "sink authorize");
            let reply = serde_json::json!({"id": id, "error": null, "result": true});
            writer
                .write_all(format!("{reply}\n").as_bytes())
                .await?;
            // One dummy notify so clients that wait for work don't spin hard.
            let notify = serde_json::json!({
                "id": null,
                "method": "mining.notify",
                "params": ["1", "00", "00", "00", [], "00000000", "00000000", "00000000", false]
            });
            writer
                .write_all(format!("{notify}\n").as_bytes())
                .await?;
            writer.flush().await?;
        } else if method.eq_ignore_ascii_case("mining.submit") {
            let reply = serde_json::json!({"id": id, "error": null, "result": true});
            writer
                .write_all(format!("{reply}\n").as_bytes())
                .await?;
            writer.flush().await?;
        }
    }
    info!(%peer, "sink client closed");
    Ok(())
}
