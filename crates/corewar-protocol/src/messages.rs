use serde::{Deserialize, Serialize};

/// Protocol version for negotiation.
pub const PROTOCOL_VERSION: u32 = 1;

/// Client-to-server messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to updates from a specific battle instance.
    Subscribe { instance_id: String },
    /// Unsubscribe from an instance.
    Unsubscribe { instance_id: String },
    /// Request current leaderboard state.
    LeaderboardQuery { top_n: usize },
    /// Start a new tournament.
    StartTournament { warrior_ids: Vec<String> },
    /// Pause/resume a running instance.
    TogglePause { instance_id: String },
}

/// Server-to-client messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Protocol version negotiation response.
    Hello { version: u32 },
    /// Batch of cycle events for visualization.
    CycleEvents {
        instance_id: String,
        cycle: u64,
        events: Vec<CycleEvent>,
    },
    /// A battle has completed.
    BattleComplete {
        instance_id: String,
        result: BattleResultMsg,
    },
    /// Leaderboard data.
    LeaderboardUpdate { entries: Vec<LeaderboardEntry> },
    /// Error response.
    Error { message: String },
}

/// A single event within a cycle (for visualization).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CycleEvent {
    Write { address: usize, warrior_id: u32 },
    Execute { address: usize, warrior_id: u32 },
    ProcessCreated { warrior_id: u32, address: usize },
    ProcessKilled { warrior_id: u32, address: usize },
}

/// Battle result as transmitted over the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BattleResultMsg {
    Win { winner: String },
    Draw { survivors: Vec<String> },
}

/// A single leaderboard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub warrior_name: String,
    pub rating: f64,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}
