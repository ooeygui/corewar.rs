//! Glicko-2 rating system implementation.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::history::MatchHistory;

const DEFAULT_RATING: f64 = 1500.0;
const DEFAULT_DEVIATION: f64 = 350.0;
const DEFAULT_VOLATILITY: f64 = 0.06;
const GLICKO_SCALE: f64 = 173.7178;
const TAU: f64 = 0.5;
const EPSILON: f64 = 0.000_001;

/// A player's Glicko-2 rating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            rating: DEFAULT_RATING,
            deviation: DEFAULT_DEVIATION,
            volatility: DEFAULT_VOLATILITY,
            wins: 0,
            losses: 0,
            draws: 0,
        }
    }

    pub fn games_played(&self) -> u32 {
        self.wins + self.losses + self.draws
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatchOutcome {
    Win { winner: String, loser: String },
    Draw { players: Vec<String> },
}

impl MatchOutcome {
    pub fn involves_player(&self, player: &str) -> bool {
        match self {
            Self::Win { winner, loser } => winner == player || loser == player,
            Self::Draw { players } => players.iter().any(|entry| entry == player),
        }
    }

    fn participant_names(&self) -> Vec<String> {
        match self {
            Self::Win { winner, loser } => vec![winner.clone(), loser.clone()],
            Self::Draw { players } => unique_players(players),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeadToHead {
    pub wins_a: u32,
    pub wins_b: u32,
    pub draws: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Leaderboard {
    players: HashMap<String, PlayerRating>,
    head_to_head: HashMap<String, HeadToHead>,
    history: MatchHistory,
}

impl Leaderboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a player rating.
    pub fn get_or_create(&mut self, name: &str) -> &mut PlayerRating {
        self.players
            .entry(name.to_string())
            .or_insert_with(|| PlayerRating::new(name))
    }

    pub fn player(&self, name: &str) -> Option<&PlayerRating> {
        self.players.get(name)
    }

    pub fn history(&self) -> &MatchHistory {
        &self.history
    }

    /// Get top N players by rating.
    pub fn top_n(&self, n: usize) -> Vec<&PlayerRating> {
        let mut sorted: Vec<_> = self.players.values().collect();
        sorted.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        sorted.truncate(n);
        sorted
    }

    /// Get all ratings for serialization.
    pub fn all_ratings(&self) -> &HashMap<String, PlayerRating> {
        &self.players
    }

    pub fn record_result(&mut self, winner: &str, loser: &str) {
        self.record_match_result(&[MatchOutcome::Win {
            winner: winner.to_string(),
            loser: loser.to_string(),
        }]);
    }

    pub fn record_draw(&mut self, player_a: &str, player_b: &str) {
        self.record_match_result(&[MatchOutcome::Draw {
            players: vec![player_a.to_string(), player_b.to_string()],
        }]);
    }

    pub fn record_match_result(&mut self, results: &[MatchOutcome]) {
        if results.is_empty() {
            return;
        }

        let involved_players: HashSet<String> = results
            .iter()
            .flat_map(MatchOutcome::participant_names)
            .collect();

        for name in &involved_players {
            self.get_or_create(name);
        }

        let snapshot: HashMap<String, PlayerRating> = involved_players
            .iter()
            .filter_map(|name| {
                self.players
                    .get(name)
                    .cloned()
                    .map(|player| (name.clone(), player))
            })
            .collect();

        let mut period_results: HashMap<String, Vec<OpponentResult>> = HashMap::new();
        let mut record_updates: HashMap<String, (u32, u32, u32)> = HashMap::new();

        for outcome in results {
            self.history.record(outcome.clone());

            match outcome {
                MatchOutcome::Win { winner, loser } => {
                    let winner_rating =
                        snapshot.get(winner).expect("winner must exist in snapshot");
                    let loser_rating = snapshot.get(loser).expect("loser must exist in snapshot");

                    period_results
                        .entry(winner.clone())
                        .or_default()
                        .push(OpponentResult::new(loser_rating, 1.0));
                    period_results
                        .entry(loser.clone())
                        .or_default()
                        .push(OpponentResult::new(winner_rating, 0.0));

                    record_updates.entry(winner.clone()).or_default().0 += 1;
                    record_updates.entry(loser.clone()).or_default().1 += 1;
                    self.update_head_to_head_win(winner, loser);
                }
                MatchOutcome::Draw { players } => {
                    let unique = unique_players(players);

                    for (index, player) in unique.iter().enumerate() {
                        for opponent in unique.iter().skip(index + 1) {
                            let player_rating = snapshot
                                .get(player)
                                .expect("draw participant must exist in snapshot");
                            let opponent_rating = snapshot
                                .get(opponent)
                                .expect("draw participant must exist in snapshot");

                            period_results
                                .entry(player.clone())
                                .or_default()
                                .push(OpponentResult::new(opponent_rating, 0.5));
                            period_results
                                .entry(opponent.clone())
                                .or_default()
                                .push(OpponentResult::new(player_rating, 0.5));

                            self.update_head_to_head_draw(player, opponent);
                        }
                    }

                    let draw_count = unique.len().saturating_sub(1) as u32;
                    for player in unique {
                        record_updates.entry(player).or_default().2 += draw_count;
                    }
                }
            }
        }

        for name in involved_players {
            let current = snapshot
                .get(&name)
                .expect("all involved players must have a snapshot");
            let updated = update_player_rating(current, period_results.get(&name));
            let (wins, losses, draws) = record_updates.get(&name).copied().unwrap_or_default();
            let entry = self
                .players
                .get_mut(&name)
                .expect("all involved players must exist");
            entry.rating = updated.rating;
            entry.deviation = updated.deviation;
            entry.volatility = updated.volatility;
            entry.wins += wins;
            entry.losses += losses;
            entry.draws += draws;
        }
    }

    pub fn head_to_head(&self, player_a: &str, player_b: &str) -> HeadToHead {
        let (key, is_forward) = pair_key(player_a, player_b);
        let stats = self.head_to_head.get(&key).cloned().unwrap_or_default();
        if is_forward {
            stats
        } else {
            HeadToHead {
                wins_a: stats.wins_b,
                wins_b: stats.wins_a,
                draws: stats.draws,
            }
        }
    }

    fn update_head_to_head_win(&mut self, winner: &str, loser: &str) {
        let (key, is_forward) = pair_key(winner, loser);
        let entry = self.head_to_head.entry(key).or_default();
        if is_forward {
            entry.wins_a += 1;
        } else {
            entry.wins_b += 1;
        }
    }

    fn update_head_to_head_draw(&mut self, player_a: &str, player_b: &str) {
        let (key, _) = pair_key(player_a, player_b);
        self.head_to_head.entry(key).or_default().draws += 1;
    }
}

#[derive(Debug, Clone, Copy)]
struct OpponentResult {
    rating: f64,
    deviation: f64,
    score: f64,
}

impl OpponentResult {
    fn new(opponent: &PlayerRating, score: f64) -> Self {
        Self {
            rating: opponent.rating,
            deviation: opponent.deviation,
            score,
        }
    }
}

fn unique_players(players: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    players
        .iter()
        .filter(|player| seen.insert(player.as_str()))
        .cloned()
        .collect()
}

fn pair_key(player_a: &str, player_b: &str) -> (String, bool) {
    if player_a <= player_b {
        (format!("{player_a}\0{player_b}"), true)
    } else {
        (format!("{player_b}\0{player_a}"), false)
    }
}

fn update_player_rating(
    player: &PlayerRating,
    results: Option<&Vec<OpponentResult>>,
) -> PlayerRating {
    let mut updated = player.clone();
    let mu = (player.rating - DEFAULT_RATING) / GLICKO_SCALE;
    let phi = player.deviation / GLICKO_SCALE;

    let Some(results) = results.filter(|results| !results.is_empty()) else {
        updated.deviation = ((phi.powi(2) + player.volatility.powi(2)).sqrt() * GLICKO_SCALE)
            .min(DEFAULT_DEVIATION);
        return updated;
    };

    let variance_inverse: f64 = results
        .iter()
        .map(|result| {
            let g = g(result.deviation / GLICKO_SCALE);
            let expectation = expected_score(mu, result.rating, result.deviation);
            g.powi(2) * expectation * (1.0 - expectation)
        })
        .sum();
    let v = 1.0 / variance_inverse;

    let delta_sum: f64 = results
        .iter()
        .map(|result| {
            let g = g(result.deviation / GLICKO_SCALE);
            let expectation = expected_score(mu, result.rating, result.deviation);
            g * (result.score - expectation)
        })
        .sum();
    let delta = v * delta_sum;

    let sigma_prime = update_volatility(phi, player.volatility, delta, v);
    let phi_star = (phi.powi(2) + sigma_prime.powi(2)).sqrt();
    let phi_prime = 1.0 / ((1.0 / phi_star.powi(2)) + (1.0 / v)).sqrt();
    let mu_prime = mu + phi_prime.powi(2) * delta_sum;

    updated.rating = mu_prime * GLICKO_SCALE + DEFAULT_RATING;
    updated.deviation = (phi_prime * GLICKO_SCALE).min(DEFAULT_DEVIATION);
    updated.volatility = sigma_prime;
    updated
}

fn g(phi: f64) -> f64 {
    1.0 / (1.0 + (3.0 * phi.powi(2) / std::f64::consts::PI.powi(2))).sqrt()
}

fn expected_score(mu: f64, opponent_rating: f64, opponent_deviation: f64) -> f64 {
    let opponent_mu = (opponent_rating - DEFAULT_RATING) / GLICKO_SCALE;
    let opponent_phi = opponent_deviation / GLICKO_SCALE;
    1.0 / (1.0 + (-g(opponent_phi) * (mu - opponent_mu)).exp())
}

fn update_volatility(phi: f64, sigma: f64, delta: f64, v: f64) -> f64 {
    let a = (sigma * sigma).ln();
    let tau_squared = TAU.powi(2);
    let volatility_function = |x: f64| {
        let exp_x = x.exp();
        let numerator = exp_x * (delta.powi(2) - phi.powi(2) - v - exp_x);
        let denominator = 2.0 * (phi.powi(2) + v + exp_x).powi(2);
        numerator / denominator - (x - a) / tau_squared
    };

    let mut a_bound = a;
    let mut b_bound = if delta.powi(2) > phi.powi(2) + v {
        (delta.powi(2) - phi.powi(2) - v).ln()
    } else {
        let mut k = 1.0;
        loop {
            let candidate = a - k * TAU;
            if volatility_function(candidate) < 0.0 {
                break candidate;
            }
            k += 1.0;
        }
    };

    let mut f_a = volatility_function(a_bound);
    let mut f_b = volatility_function(b_bound);

    while (b_bound - a_bound).abs() > EPSILON {
        let c_bound = a_bound + ((a_bound - b_bound) * f_a / (f_b - f_a));
        let f_c = volatility_function(c_bound);

        if f_c * f_b < 0.0 {
            a_bound = b_bound;
            f_a = f_b;
        } else {
            f_a /= 2.0;
        }

        b_bound = c_bound;
        f_b = f_c;
    }

    (a_bound / 2.0).exp()
}

impl Default for PlayerRating {
    fn default() -> Self {
        Self::new("")
    }
}
