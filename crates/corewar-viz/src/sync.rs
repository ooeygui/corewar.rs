use corewar_protocol::{
    BattleResultMsg, CellInfo, ClientMessage, CycleEvent, ServerMessage, PROTOCOL_VERSION,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::network::NetworkClient;
#[cfg(target_arch = "wasm32")]
use crate::network_wasm::NetworkClient;
use crate::{app::App, renderer::CellState, NetworkError, VizState};

/// Applies network updates to the visualization state and exposes connection UI state.
pub struct StateSynchronizer {
    client: NetworkClient,
    instance_id: String,
    pending_events: Vec<CycleEvent>,
    overlay_message: Option<String>,
    initialized: bool,
    was_connected: bool,
}

impl StateSynchronizer {
    pub async fn connect(url: &str, instance_id: impl Into<String>) -> Result<Self, NetworkError> {
        let mut client = NetworkClient::connect(url).await?;
        let instance_id = instance_id.into();
        client
            .send(ClientMessage::Subscribe {
                instance_id: instance_id.clone(),
            })
            .await?;
        let was_connected = client.is_connected();

        Ok(Self {
            client,
            instance_id,
            pending_events: Vec::new(),
            overlay_message: None,
            initialized: false,
            was_connected,
        })
    }

    pub async fn synchronize(&mut self, state: &mut VizState) -> Result<(), NetworkError> {
        self.refresh_connection_state().await?;

        for message in self.client.poll_messages() {
            self.handle_message(state, message);
        }

        if !self.pending_events.is_empty() {
            state.apply_events(&self.pending_events);
            self.pending_events.clear();
        }

        Ok(())
    }

    pub async fn synchronize_app(&mut self, app: &mut App<'_>) -> Result<(), NetworkError> {
        self.refresh_connection_state().await?;

        for message in self.client.poll_messages() {
            self.handle_app_message(app, message);
        }

        Ok(())
    }

    pub async fn send(&mut self, msg: ClientMessage) -> Result<(), NetworkError> {
        self.client.send(msg).await
    }

    pub fn overlay_message(&self) -> Option<&str> {
        self.overlay_message.as_deref()
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn refresh_connection_state(&mut self) -> Result<(), NetworkError> {
        let connected = self.client.is_connected();
        if connected && !self.was_connected {
            self.pending_events.clear();
            self.initialized = false;
            self.overlay_message = Some("reconnected - syncing...".to_string());
            self.client
                .send(ClientMessage::Subscribe {
                    instance_id: self.instance_id.clone(),
                })
                .await?;
        } else if !connected {
            self.overlay_message = Some("reconnecting...".to_string());
        }
        self.was_connected = connected;
        Ok(())
    }

    fn handle_message(&mut self, state: &mut VizState, message: ServerMessage) {
        match message {
            ServerMessage::Hello { version } if version != PROTOCOL_VERSION => {
                self.overlay_message = Some(format!(
                    "protocol mismatch: expected {PROTOCOL_VERSION}, got {version}"
                ));
            }
            ServerMessage::Hello { .. } => {}
            ServerMessage::CoreSnapshot { instance_id, cells }
                if instance_id == self.instance_id =>
            {
                apply_snapshot(state, &cells);
                self.pending_events.clear();
                self.overlay_message = None;
                self.initialized = true;
            }
            ServerMessage::CycleEvents {
                instance_id,
                events,
                ..
            } if instance_id == self.instance_id => {
                self.pending_events.extend(events);
                if self.initialized {
                    self.overlay_message = None;
                }
            }
            ServerMessage::BattleComplete {
                instance_id,
                result,
            } if instance_id == self.instance_id => {
                self.overlay_message = Some(format_battle_result(&result));
            }
            ServerMessage::Error { message } => {
                self.overlay_message = Some(message);
            }
            _ => {}
        }
    }

    fn handle_app_message(&mut self, app: &mut App<'_>, message: ServerMessage) {
        match message {
            ServerMessage::Hello { version } if version != PROTOCOL_VERSION => {
                self.overlay_message = Some(format!(
                    "protocol mismatch: expected {PROTOCOL_VERSION}, got {version}"
                ));
            }
            ServerMessage::Hello { .. } => {}
            ServerMessage::CoreSnapshot { instance_id, cells }
                if instance_id == self.instance_id =>
            {
                app.apply_snapshot(&cells);
                self.pending_events.clear();
                self.overlay_message = None;
                self.initialized = true;
            }
            ServerMessage::CycleEvents {
                instance_id,
                events,
                ..
            } if instance_id == self.instance_id => {
                app.queue_cycle_events(std::iter::once(events));
                if self.initialized {
                    self.overlay_message = None;
                }
            }
            ServerMessage::BattleComplete {
                instance_id,
                result,
            } if instance_id == self.instance_id => {
                self.overlay_message = Some(format_battle_result(&result));
            }
            ServerMessage::Error { message } => {
                self.overlay_message = Some(message);
            }
            _ => {}
        }
    }
}

fn apply_snapshot(state: &mut VizState, cells: &[CellInfo]) {
    for cell in &mut state.core.cells {
        *cell = CellState::default();
    }
    state.core.current_cycle = 0;

    for snapshot_cell in cells {
        if let Some(cell) = state.core.cells.get_mut(snapshot_cell.address) {
            cell.owner = snapshot_cell.owner;
            cell.heat = if snapshot_cell.owner.is_some() {
                1.0
            } else {
                0.0
            };
            cell.last_access_cycle = state.core.current_cycle;
        }
    }
}

fn format_battle_result(result: &BattleResultMsg) -> String {
    match result {
        BattleResultMsg::Win { winner } => format!("battle complete - winner: {winner}"),
        BattleResultMsg::Draw { survivors } if survivors.is_empty() => {
            "battle complete - draw".to_string()
        }
        BattleResultMsg::Draw { survivors } => {
            format!("battle complete - draw between {}", survivors.join(", "))
        }
    }
}
