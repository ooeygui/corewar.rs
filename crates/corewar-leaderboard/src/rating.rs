//! Glicko-2 rating system implementation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A player's Glicko-2 rating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRating {
    pub name: String,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

impl PlayerRating {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rating: 1500.0,
            deviation: 350.0,
            volatility: 0.06,
            wins: 0,
            losses: 0,
            draws: 0,
        }
    }
}

/// The leaderboard holding all warrior ratings.
pub struct Leaderboard {
    players: HashMap<String, PlayerRating>,
}

impl Leaderboard {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    /// Get or create a player rating.
    pub fn get_or_create(&mut self, name: &str) -> &mut PlayerRating {
        self.players
            .entry(name.to_string())
            .or_insert_with(|| PlayerRating::new(name))
    }

    /// Get top N players by rating.
    pub fn top_n(&self, n: usize) -> Vec<&PlayerRating> {
        let mut sorted: Vec<_> = self.players.values().collect();
        sorted.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());
        sorted.truncate(n);
        sorted
    }

    /// Get all ratings for serialization.
    pub fn all_ratings(&self) -> &HashMap<String, PlayerRating> {
        &self.players
    }
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self::new()
    }
}
