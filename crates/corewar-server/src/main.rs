//! # CoreWar Server
//!
//! WebSocket server that hosts the orchestrator and provides real-time
//! battle updates to connected visualization clients.

use tokio::net::TcpListener;
use tracing::info;

mod connection;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "0.0.0.0:9000";
    let listener = TcpListener::bind(addr).await?;
    info!("CoreWar server listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New connection from {}", peer);
        tokio::spawn(connection::handle(stream));
    }
}
