//! WebSocket connection handler.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::{handlers, state::AppState};

pub async fn handle(stream: TcpStream, state: Arc<AppState>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(error) => {
            error!(%error, "WebSocket handshake failed");
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let client_id = state.add_client(tx.clone()).await;

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let payload = corewar_protocol::encode_json(&message);
            if let Err(error) = write.send(Message::Text(payload.into())).await {
                warn!(%error, "Failed to write websocket message");
                break;
            }
        }

        if let Err(error) = write.close().await {
            warn!(%error, "Failed to close websocket writer cleanly");
        }
    });

    let _ = tx.send(corewar_protocol::ServerMessage::Hello {
        version: corewar_protocol::PROTOCOL_VERSION,
    });

    while let Some(frame) = read.next().await {
        match frame {
            Ok(Message::Text(text)) => {
                info!(client_id, payload = %text, "Received client message");
                match corewar_protocol::decode_json(&text) {
                    Ok(message) => {
                        match handlers::dispatch_client_message(&state, client_id, message).await {
                            Ok(responses) => {
                                for response in responses {
                                    if tx.send(response).is_err() {
                                        warn!(client_id, "Client response channel closed");
                                        break;
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = tx.send(corewar_protocol::ServerMessage::Error {
                                    message: error.to_string(),
                                });
                            }
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(corewar_protocol::ServerMessage::Error {
                            message: error.to_string(),
                        });
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                let _ = tx.send(corewar_protocol::ServerMessage::Error {
                    message: "binary websocket frames are not supported".to_string(),
                });
            }
            Ok(Message::Close(_)) => {
                info!(client_id, "Client initiated websocket close");
                break;
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {}
            Err(error) => {
                error!(client_id, %error, "WebSocket read failed");
                break;
            }
        }
    }

    state.remove_client(client_id).await;
    drop(tx);

    if let Err(error) = writer.await {
        warn!(client_id, %error, "WebSocket writer task ended unexpectedly");
    }

    info!(client_id, "Client disconnected");
}
