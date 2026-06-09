use corewar_leaderboard::{Leaderboard, MatchOutcome};
use corewar_protocol::{BattleResultMsg, LeaderboardEntry, ServerMessage};
use corewar_vm::{battle::BattleStats, BattleResult};

#[derive(Debug, Clone, Default)]
pub struct LeaderboardBridge {
    leaderboard: Leaderboard,
}

impl LeaderboardBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn leaderboard(&self) -> &Leaderboard {
        &self.leaderboard
    }

    pub fn record_match(
        &mut self,
        warriors: &[String],
        result: &BattleResult,
    ) -> Vec<MatchOutcome> {
        let outcomes = battle_result_to_outcomes(warriors, result);
        self.leaderboard.record_match_result(&outcomes);
        outcomes
    }

    pub fn final_standings_message(&self) -> ServerMessage {
        let entries = self
            .leaderboard
            .top_n(self.leaderboard.all_ratings().len())
            .into_iter()
            .map(|rating| LeaderboardEntry {
                warrior_name: rating.name.clone(),
                rating: rating.rating,
                wins: rating.wins,
                losses: rating.losses,
                draws: rating.draws,
            })
            .collect();

        ServerMessage::LeaderboardUpdate { entries }
    }
}

pub fn aggregate_battle_result(stats: &[BattleStats]) -> BattleResult {
    let mut leaders = Vec::new();
    let mut best_key = None;

    for stats in stats {
        let key = (stats.score, stats.wins, stats.draws);
        match best_key {
            None => {
                best_key = Some(key);
                leaders.push(stats.warrior_id);
            }
            Some(current) if key > current => {
                best_key = Some(key);
                leaders.clear();
                leaders.push(stats.warrior_id);
            }
            Some(current) if key == current => leaders.push(stats.warrior_id),
            Some(_) => {}
        }
    }

    match leaders.as_slice() {
        [winner_id] => BattleResult::Win {
            winner_id: *winner_id,
        },
        _ => BattleResult::Draw {
            survivor_ids: leaders,
        },
    }
}

pub fn battle_result_message(warriors: &[String], result: &BattleResult) -> BattleResultMsg {
    match result {
        BattleResult::Win { winner_id } => BattleResultMsg::Win {
            winner: warrior_name(warriors, *winner_id)
                .unwrap_or_else(|| format!("warrior-{winner_id}")),
        },
        BattleResult::Draw { survivor_ids } => BattleResultMsg::Draw {
            survivors: survivor_ids
                .iter()
                .filter_map(|warrior_id| warrior_name(warriors, *warrior_id))
                .collect(),
        },
    }
}

pub fn battle_result_to_outcomes(warriors: &[String], result: &BattleResult) -> Vec<MatchOutcome> {
    match result {
        BattleResult::Win { winner_id } => {
            let Some(winner) = warrior_name(warriors, *winner_id) else {
                return Vec::new();
            };

            warriors
                .iter()
                .filter(|warrior| *warrior != &winner)
                .map(|loser| MatchOutcome::Win {
                    winner: winner.clone(),
                    loser: loser.clone(),
                })
                .collect()
        }
        BattleResult::Draw { survivor_ids } => {
            let survivors: Vec<_> = survivor_ids
                .iter()
                .filter_map(|warrior_id| warrior_name(warriors, *warrior_id))
                .collect();
            if survivors.len() < 2 {
                Vec::new()
            } else {
                vec![MatchOutcome::Draw { players: survivors }]
            }
        }
    }
}

fn warrior_name(warriors: &[String], warrior_id: u32) -> Option<String> {
    warriors.get(warrior_id.checked_sub(1)? as usize).cloned()
}
