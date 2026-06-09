//! # CoreWar Orchestrator
//!
//! Manages multiple concurrent VM instances, schedules tournaments,
//! and coordinates with the leaderboard system.

pub mod bridge;
pub mod instance;
pub mod scheduler;
pub mod tournament;

pub use bridge::LeaderboardBridge;
pub use instance::{BattleInstance, BattleInstanceStatus, InstanceEventObserver, InstanceManager};
pub use scheduler::{generate_matches, Match, ScheduleStrategy};
pub use tournament::{
    BattleRunReport, BattleRunner, Tournament, TournamentConfig, TournamentEvent, TournamentResult,
};
