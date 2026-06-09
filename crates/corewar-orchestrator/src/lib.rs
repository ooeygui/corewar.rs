//! # CoreWar Orchestrator
//!
//! Manages multiple concurrent VM instances, schedules tournaments,
//! and coordinates with the leaderboard system.

pub mod scheduler;
pub mod tournament;

pub use tournament::{Tournament, TournamentConfig};
