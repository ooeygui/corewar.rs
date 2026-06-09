//! # CoreWar Server
//!
//! WebSocket server that hosts the orchestrator and provides real-time
//! battle updates to connected visualization clients.

use std::{env, sync::Arc};

use tokio::net::TcpListener;
use tracing::{info, Level};

mod connection;
mod handlers;
mod state;

use state::AppState;

struct Cli {
    port: u16,
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::from_env()?;
    let log_level = parse_log_level(&cli.log_level)?;

    tracing_subscriber::fmt().with_max_level(log_level).init();

    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = TcpListener::bind(&addr).await?;
    let state = Arc::new(AppState::new());

    info!(port = cli.port, log_level = %log_level, "CoreWar server listening");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = Arc::clone(&state);
        info!(%peer, "Accepted connection");
        tokio::spawn(async move {
            connection::handle(stream, state).await;
        });
    }
}

impl Cli {
    fn from_env() -> anyhow::Result<Self> {
        let mut args = env::args().skip(1);
        let mut port = env::var("COREWAR_SERVER_PORT")
            .ok()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(9000);
        let mut log_level =
            env::var("COREWAR_SERVER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--port" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --port"))?;
                    port = value.parse()?;
                }
                "--log-level" => {
                    log_level = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --log-level"))?;
                }
                other => return Err(anyhow::anyhow!("unknown argument: {other}")),
            }
        }

        Ok(Self { port, log_level })
    }
}

fn parse_log_level(value: &str) -> anyhow::Result<Level> {
    match value.to_ascii_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" | "warning" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        other => Err(anyhow::anyhow!("unsupported log level: {other}")),
    }
}
