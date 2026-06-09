use serde::{Deserialize, Serialize};

/// Protocol version for negotiation.
pub const PROTOCOL_VERSION: u32 = 1;

/// Client-to-server messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Request the list of active and historical battle instances.
    ListInstances,
    /// Subscribe to updates from a specific battle instance.
    Subscribe { instance_id: String },
    /// Unsubscribe from an instance.
    Unsubscribe { instance_id: String },
    /// Request current leaderboard state.
    LeaderboardQuery { top_n: usize },
    /// Upload a warrior source file for compilation and registration.
    LoadWarrior { source: String },
    /// Start a new tournament.
    StartTournament { warrior_ids: Vec<String> },
    /// Pause/resume a running instance.
    TogglePause { instance_id: String },
}

/// Server-to-client messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Protocol version negotiation response.
    Hello { version: u32 },
    /// List of instances available to subscribe to.
    InstanceList { instances: Vec<InstanceInfo> },
    /// Full core state for a specific instance.
    CoreSnapshot {
        instance_id: String,
        cells: Vec<CellInfo>,
    },
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

/// Summary information for an instance in the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub warrior_names: Vec<String>,
    pub core_size: usize,
    pub cycle: u64,
    pub status: InstanceStatus,
}

/// Runtime status for an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceStatus {
    Running,
    Paused,
    Complete,
}

/// Summary information for a warrior participating in an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarriorInfo {
    pub id: u32,
    pub name: String,
    pub author: String,
    pub process_count: usize,
    pub color_index: u32,
}

/// Snapshot information for a single core cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellInfo {
    pub address: usize,
    pub owner: Option<u32>,
    pub instruction_summary: String,
}

/// A single event within a cycle (for visualization).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleEvent {
    Read { address: usize, warrior_id: u32 },
    Write { address: usize, warrior_id: u32 },
    Execute { address: usize, warrior_id: u32 },
    ProcessCreated { warrior_id: u32, address: usize },
    ProcessKilled { warrior_id: u32, address: usize },
}

/// Battle result as transmitted over the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleResultMsg {
    Win { winner: String },
    Draw { survivors: Vec<String> },
}

/// A single leaderboard entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub warrior_name: String,
    pub rating: f64,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}
