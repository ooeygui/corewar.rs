//! Tournament management.

use crate::scheduler::ScheduleStrategy;

/// Configuration for a tournament.
#[derive(Debug, Clone)]
pub struct TournamentConfig {
    /// How matches are scheduled.
    pub strategy: ScheduleStrategy,
    /// Number of rounds per matchup.
    pub rounds_per_match: usize,
    /// Maximum concurrent battles.
    pub concurrency: usize,
}

impl Default for TournamentConfig {
    fn default() -> Self {
        Self {
            strategy: ScheduleStrategy::RoundRobin,
            rounds_per_match: 100,
            concurrency: 4,
        }
    }
}

/// A running tournament instance.
pub struct Tournament {
    pub config: TournamentConfig,
    // TODO: warrior list, match queue, results
}

impl Tournament {
    pub fn new(config: TournamentConfig) -> Self {
        Self { config }
    }
}
