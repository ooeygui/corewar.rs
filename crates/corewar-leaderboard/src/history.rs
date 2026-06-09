//! Match history tracking for leaderboard updates.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::rating::MatchOutcome;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoricalMatch {
    pub outcome: MatchOutcome,
    pub timestamp_unix_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchHistory {
    matches: Vec<HistoricalMatch>,
}

impl MatchHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, outcome: MatchOutcome) {
        let timestamp_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.matches.push(HistoricalMatch {
            outcome,
            timestamp_unix_secs,
        });
    }

    pub fn matches(&self) -> &[HistoricalMatch] {
        &self.matches
    }

    pub fn recent_matches_for_player(&self, player: &str, limit: usize) -> Vec<&HistoricalMatch> {
        self.matches
            .iter()
            .rev()
            .filter(|entry| entry.outcome.involves_player(player))
            .take(limit)
            .collect()
    }
}
