//! WebSocket connection handler.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tracing::{error, info};

pub async fn handle(stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    // Send hello message
    let hello = corewar_protocol::ServerMessage::Hello {
        version: corewar_protocol::PROTOCOL_VERSION,
    };
    let msg = serde_json::to_string(&hello).unwrap();
    let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(msg.into())).await;

    // Message loop
    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                info!("Received: {}", text);
                // TODO: Parse ClientMessage and dispatch
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
