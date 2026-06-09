//! Tournament scheduling strategies.

/// Scheduling strategy for tournament matchmaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleStrategy {
    /// Every warrior fights every other warrior.
    RoundRobin,
    /// Swiss-system pairing based on current standings.
    Swiss { rounds: usize },
    /// Single-elimination bracket.
    Elimination,
}

/// A scheduled tournament match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub id: String,
    pub warrior_ids: Vec<String>,
    pub round: usize,
}

/// Generate matches for the provided warriors using the requested strategy.
///
/// For Swiss and elimination tournaments, the incoming `warriors` slice is expected
/// to already be ordered by the current standings or bracket order.
pub fn generate_matches(warriors: &[String], strategy: &ScheduleStrategy) -> Vec<Match> {
    match strategy {
        ScheduleStrategy::RoundRobin => generate_round_robin_matches(warriors),
        ScheduleStrategy::Swiss { .. } | ScheduleStrategy::Elimination => {
            pair_adjacent(warriors, 1, strategy_name(strategy))
        }
    }
}

fn generate_round_robin_matches(warriors: &[String]) -> Vec<Match> {
    if warriors.len() < 2 {
        return Vec::new();
    }

    let mut rotation: Vec<Option<&String>> = warriors.iter().map(Some).collect();
    if rotation.len() % 2 == 1 {
        rotation.push(None);
    }

    let rounds = rotation.len() - 1;
    let half = rotation.len() / 2;
    let mut matches = Vec::new();

    for round in 0..rounds {
        for slot in 0..half {
            let left = rotation[slot].cloned();
            let right = rotation[rotation.len() - 1 - slot].cloned();
            if let (Some(left), Some(right)) = (left, right) {
                let match_number = matches.len() + 1;
                matches.push(Match {
                    id: format!("round-robin-r{}-m{}", round + 1, match_number),
                    warrior_ids: vec![left.clone(), right.clone()],
                    round: round + 1,
                });
            }
        }

        if let Some(last) = rotation.pop() {
            rotation.insert(1, last);
        }
    }

    matches
}

fn pair_adjacent(warriors: &[String], round: usize, prefix: &str) -> Vec<Match> {
    warriors
        .chunks(2)
        .enumerate()
        .filter(|(_, pair)| pair.len() == 2)
        .map(|(index, pair)| Match {
            id: format!("{prefix}-r{round}-m{}", index + 1),
            warrior_ids: pair.to_vec(),
            round,
        })
        .collect()
}

fn strategy_name(strategy: &ScheduleStrategy) -> &'static str {
    match strategy {
        ScheduleStrategy::RoundRobin => "round-robin",
        ScheduleStrategy::Swiss { .. } => "swiss",
        ScheduleStrategy::Elimination => "elimination",
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_matches, ScheduleStrategy};

    #[test]
    fn round_robin_generates_all_pairs() {
        let warriors = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
        ];

        let matches = generate_matches(&warriors, &ScheduleStrategy::RoundRobin);

        assert_eq!(matches.len(), warriors.len() * (warriors.len() - 1) / 2);
        assert!(matches.iter().all(|entry| entry.warrior_ids.len() == 2));
    }

    #[test]
    fn swiss_pairing_respects_input_standings_order() {
        let warriors = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
            "fourth".to_string(),
        ];

        let matches = generate_matches(&warriors, &ScheduleStrategy::Swiss { rounds: 3 });

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].warrior_ids, vec!["first", "second"]);
        assert_eq!(matches[1].warrior_ids, vec!["third", "fourth"]);
    }
}
