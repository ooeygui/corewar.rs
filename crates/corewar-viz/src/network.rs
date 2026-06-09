use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use corewar_protocol::{ClientMessage, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, warn};

use crate::NetworkError;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const OUTBOUND_BUFFER: usize = 128;
const INBOUND_BUFFER: usize = 256;
const INITIAL_BACKOFF_MS: u64 = 250;
const MAX_BACKOFF_MS: u64 = 5_000;

/// Native WebSocket client for the visualization frontend.
pub struct NetworkClient {
    outbound_tx: mpsc::Sender<ClientMessage>,
    inbound_rx: mpsc::Receiver<ServerMessage>,
    connected: Arc<AtomicBool>,
}

impl NetworkClient {
    pub async fn connect(url: &str) -> Result<Self, NetworkError> {
        let (stream, _) = connect_async(url)
            .await
            .map_err(|err| NetworkError::Connect(err.to_string()))?;

        let (outbound_tx, outbound_rx) = mpsc::channel(OUTBOUND_BUFFER);
        let (inbound_tx, inbound_rx) = mpsc::channel(INBOUND_BUFFER);
        let connected = Arc::new(AtomicBool::new(true));

        tokio::spawn(connection_task(
            url.to_owned(),
            stream,
            outbound_rx,
            inbound_tx,
            Arc::clone(&connected),
        ));

        Ok(Self {
            outbound_tx,
            inbound_rx,
            connected,
        })
    }

    pub async fn send(&mut self, msg: ClientMessage) -> Result<(), NetworkError> {
        self.outbound_tx
            .send(msg)
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    pub fn poll_messages(&mut self) -> Vec<ServerMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.inbound_rx.try_recv() {
            messages.push(message);
        }
        messages
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

async fn connection_task(
    url: String,
    mut current_stream: WsStream,
    mut outbound_rx: mpsc::Receiver<ClientMessage>,
    inbound_tx: mpsc::Sender<ServerMessage>,
    connected: Arc<AtomicBool>,
) {
    let mut pending_outbound = VecDeque::new();

    loop {
        connected.store(true, Ordering::Relaxed);
        let (mut write, mut read) = current_stream.split();

        while let Some(message) = pending_outbound.pop_front() {
            if let Err(err) = send_ws_message(&mut write, &message).await {
                warn!(error = %err, "failed to flush outbound message; reconnecting");
                pending_outbound.push_front(message);
                connected.store(false, Ordering::Relaxed);
                break;
            }
        }

        while connected.load(Ordering::Relaxed) {
            tokio::select! {
                maybe_outbound = outbound_rx.recv() => {
                    match maybe_outbound {
                        Some(message) => {
                            if let Err(err) = send_ws_message(&mut write, &message).await {
                                warn!(error = %err, "failed to send websocket message; reconnecting");
                                pending_outbound.push_back(message);
                                connected.store(false, Ordering::Relaxed);
                            }
                        }
                        None => return,
                    }
                }
                maybe_inbound = read.next() => {
                    match maybe_inbound {
                        Some(Ok(message)) => {
                            match decode_ws_message(message) {
                                Ok(Some(server_message)) => {
                                    if inbound_tx.send(server_message).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => {}
                                Err(err) => warn!(error = %err, "failed to decode websocket message"),
                            }
                        }
                        Some(Err(err)) => {
                            warn!(error = %err, "websocket receive failed; reconnecting");
                            connected.store(false, Ordering::Relaxed);
                        }
                        None => {
                            debug!("websocket stream ended; reconnecting");
                            connected.store(false, Ordering::Relaxed);
                        }
                    }
                }
            }
        }

        connected.store(false, Ordering::Relaxed);
        current_stream = match reconnect(&url, &mut outbound_rx, &mut pending_outbound).await {
            Some(stream) => stream,
            None => return,
        };
    }
}

async fn reconnect(
    url: &str,
    outbound_rx: &mut mpsc::Receiver<ClientMessage>,
    pending_outbound: &mut VecDeque<ClientMessage>,
) -> Option<WsStream> {
    let mut backoff_ms = INITIAL_BACKOFF_MS;

    loop {
        match connect_async(url).await {
            Ok((stream, _)) => return Some(stream),
            Err(err) => {
                warn!(error = %err, delay_ms = backoff_ms, "websocket reconnect attempt failed");
            }
        }

        let sleep = tokio::time::sleep(Duration::from_millis(backoff_ms));
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                _ = &mut sleep => break,
                maybe_outbound = outbound_rx.recv() => {
                    match maybe_outbound {
                        Some(message) => pending_outbound.push_back(message),
                        None => return None,
                    }
                }
            }
        }

        backoff_ms = (backoff_ms.saturating_mul(2)).min(MAX_BACKOFF_MS);
    }
}

async fn send_ws_message<S>(write: &mut S, msg: &ClientMessage) -> Result<(), NetworkError>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let payload = serde_json::to_string(msg)?;
    write
        .send(Message::Text(payload.into()))
        .await
        .map_err(|err| NetworkError::Send(err.to_string()))
}

fn decode_ws_message(message: Message) -> Result<Option<ServerMessage>, NetworkError> {
    match message {
        Message::Text(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(NetworkError::DecodeJson),
        Message::Binary(bytes) => rmp_serde::from_slice(bytes.as_ref())
            .map(Some)
            .map_err(NetworkError::DecodeMessagePack),
        _ => Ok(None),
    }
}
