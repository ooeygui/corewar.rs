//! Tournament scheduling strategies.

/// Scheduling strategy for tournament matchmaking.
#[derive(Debug, Clone)]
pub enum ScheduleStrategy {
    /// Every warrior fights every other warrior.
    RoundRobin,
    /// Swiss-system pairing based on current standings.
    Swiss { rounds: usize },
    /// Single-elimination bracket.
    Elimination,
}
