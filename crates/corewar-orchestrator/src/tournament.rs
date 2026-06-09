//! Tournament management.

use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};

use corewar_core::Warrior;
use corewar_protocol::{BattleResultMsg, ServerMessage};
use corewar_vm::{
    battle::{BattleConfig, BattleStats},
    Battle, BattleResult,
};
use tokio::{sync::broadcast, task::JoinSet};

use crate::{
    bridge::{aggregate_battle_result, battle_result_message, LeaderboardBridge},
    instance::{BattleInstance, BattleInstanceStatus, InstanceEventObserver, InstanceManager},
    scheduler::{generate_matches, Match, ScheduleStrategy},
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentResult {
    pub standings: Vec<(String, BattleStats)>,
    pub matches_played: usize,
}

#[derive(Debug, Clone)]
pub enum TournamentEvent {
    MatchComplete {
        match_id: String,
        round: usize,
        instance_id: String,
        result: BattleResultMsg,
    },
    TournamentComplete {
        standings: Vec<(String, BattleStats)>,
        matches_played: usize,
        leaderboard: ServerMessage,
    },
}

pub trait BattleRunner: Send + Sync + 'static {
    fn run(
        &self,
        instance: Arc<BattleInstance>,
        warriors: Vec<Warrior>,
        config: BattleConfig,
    ) -> BattleRunReport;
}

impl<F> BattleRunner for F
where
    F: Fn(Arc<BattleInstance>, Vec<Warrior>, BattleConfig) -> BattleRunReport
        + Send
        + Sync
        + 'static,
{
    fn run(
        &self,
        instance: Arc<BattleInstance>,
        warriors: Vec<Warrior>,
        config: BattleConfig,
    ) -> BattleRunReport {
        self(instance, warriors, config)
    }
}

#[derive(Debug, Clone)]
pub struct BattleRunReport {
    pub result: BattleResult,
    pub result_message: BattleResultMsg,
    pub stats: Vec<BattleStats>,
}

/// A running tournament instance.
pub struct Tournament {
    pub config: TournamentConfig,
    battle_config: BattleConfig,
    bridge: LeaderboardBridge,
    instances: InstanceManager,
    event_tx: broadcast::Sender<TournamentEvent>,
    runner: Arc<dyn BattleRunner>,
}

impl Tournament {
    pub fn new(config: TournamentConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let mut battle_config = BattleConfig::default();
        battle_config.rounds = config.rounds_per_match;

        Self {
            config,
            battle_config,
            bridge: LeaderboardBridge::new(),
            instances: InstanceManager::new(),
            event_tx,
            runner: Arc::new(default_battle_runner),
        }
    }

    pub fn with_battle_config(mut self, mut battle_config: BattleConfig) -> Self {
        battle_config.rounds = self.config.rounds_per_match;
        self.battle_config = battle_config;
        self
    }

    pub fn with_runner<R>(mut self, runner: R) -> Self
    where
        R: BattleRunner,
    {
        self.runner = Arc::new(runner);
        self
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TournamentEvent> {
        self.event_tx.subscribe()
    }

    pub fn instances(&self) -> &InstanceManager {
        &self.instances
    }

    pub fn leaderboard(&self) -> &LeaderboardBridge {
        &self.bridge
    }

    pub async fn run(&mut self, warriors: Vec<Warrior>) -> TournamentResult {
        let seed_order: HashMap<_, _> = warriors
            .iter()
            .enumerate()
            .map(|(index, warrior)| (warrior.name.clone(), index))
            .collect();
        let roster: HashMap<_, _> = warriors
            .into_iter()
            .map(|warrior| (warrior.name.clone(), warrior))
            .collect();

        let mut standings = initialise_standings(roster.keys().cloned().collect());
        let mut matches_played = 0;

        match &self.config.strategy {
            ScheduleStrategy::RoundRobin => {
                let mut grouped = BTreeMap::<usize, Vec<Match>>::new();
                let initial_order = ordered_participants(
                    &standings.keys().cloned().collect::<Vec<_>>(),
                    &standings,
                    &seed_order,
                );
                for scheduled_match in generate_matches(&initial_order, &self.config.strategy) {
                    grouped
                        .entry(scheduled_match.round)
                        .or_default()
                        .push(scheduled_match);
                }

                for matches in grouped.into_values() {
                    matches_played += self.execute_round(matches, &roster, &mut standings).await;
                }
            }
            ScheduleStrategy::Swiss { rounds } => {
                let participants = standings.keys().cloned().collect::<Vec<_>>();
                for round in 1..=*rounds {
                    let ordered = ordered_participants(&participants, &standings, &seed_order);
                    let matches = generate_matches(&ordered, &self.config.strategy)
                        .into_iter()
                        .enumerate()
                        .map(|(index, scheduled_match)| Match {
                            id: format!("swiss-r{round}-m{}", index + 1),
                            round,
                            ..scheduled_match
                        })
                        .collect();
                    matches_played += self.execute_round(matches, &roster, &mut standings).await;
                }
            }
            ScheduleStrategy::Elimination => {
                let mut participants = ordered_participants(
                    &standings.keys().cloned().collect::<Vec<_>>(),
                    &standings,
                    &seed_order,
                );
                let mut round = 1;

                while participants.len() > 1 {
                    let byes = if participants.len() % 2 == 1 {
                        vec![participants
                            .last()
                            .cloned()
                            .expect("participant must exist")]
                    } else {
                        Vec::new()
                    };

                    let matches: Vec<_> = generate_matches(&participants, &self.config.strategy)
                        .into_iter()
                        .enumerate()
                        .map(|(index, scheduled_match)| Match {
                            id: format!("elimination-r{round}-m{}", index + 1),
                            round,
                            ..scheduled_match
                        })
                        .collect();

                    let completed = self
                        .execute_round_with_results(matches, &roster, &mut standings)
                        .await;
                    matches_played += completed.len();

                    let mut next_round = byes;
                    next_round.extend(completed.into_iter().filter_map(|entry| {
                        pick_advancing_warrior(&entry.scheduled_match, &entry.report.result)
                    }));

                    participants = next_round;
                    round += 1;
                }
            }
        }

        let result = TournamentResult {
            standings: standings_vec(&standings),
            matches_played,
        };
        let _ = self.event_tx.send(TournamentEvent::TournamentComplete {
            standings: result.standings.clone(),
            matches_played: result.matches_played,
            leaderboard: self.bridge.final_standings_message(),
        });
        result
    }

    async fn execute_round(
        &mut self,
        matches: Vec<Match>,
        roster: &HashMap<String, Warrior>,
        standings: &mut HashMap<String, BattleStats>,
    ) -> usize {
        self.execute_round_with_results(matches, roster, standings)
            .await
            .len()
    }

    async fn execute_round_with_results(
        &mut self,
        matches: Vec<Match>,
        roster: &HashMap<String, Warrior>,
        standings: &mut HashMap<String, BattleStats>,
    ) -> Vec<CompletedMatch> {
        let concurrency = self.config.concurrency.max(1);
        let mut join_set = JoinSet::new();
        let mut pending = matches.into_iter();
        let mut completed = Vec::new();

        loop {
            while join_set.len() < concurrency {
                let Some(scheduled_match) = pending.next() else {
                    break;
                };
                let warriors = scheduled_match
                    .warrior_ids
                    .iter()
                    .filter_map(|warrior_id| roster.get(warrior_id).cloned())
                    .collect::<Vec<_>>();
                let instance = self
                    .instances
                    .create_instance(scheduled_match.warrior_ids.clone());
                let runner = self.runner.clone();
                let battle_config = self.battle_config.clone();
                join_set.spawn(run_match(
                    runner,
                    battle_config,
                    scheduled_match,
                    warriors,
                    instance,
                ));
            }

            let Some(join_result) = join_set.join_next().await else {
                break;
            };
            let outcome = join_result.expect("tournament battle task panicked");
            self.apply_match_report(&outcome.scheduled_match, &outcome.report, standings);
            let _ = self.event_tx.send(TournamentEvent::MatchComplete {
                match_id: outcome.scheduled_match.id.clone(),
                round: outcome.scheduled_match.round,
                instance_id: outcome.instance_id.clone(),
                result: outcome.report.result_message.clone(),
            });
            completed.push(outcome);
        }

        completed
    }

    fn apply_match_report(
        &mut self,
        scheduled_match: &Match,
        report: &BattleRunReport,
        standings: &mut HashMap<String, BattleStats>,
    ) {
        self.bridge
            .record_match(&scheduled_match.warrior_ids, &report.result);

        for (index, warrior_name) in scheduled_match.warrior_ids.iter().enumerate() {
            let stat = report
                .stats
                .iter()
                .find(|stats| stats.warrior_id == (index + 1) as u32)
                .cloned()
                .unwrap_or_else(|| BattleStats {
                    warrior_id: (index + 1) as u32,
                    ..BattleStats::default()
                });
            merge_stats(standings.entry(warrior_name.clone()).or_default(), &stat);
        }
    }
}

struct CompletedMatch {
    scheduled_match: Match,
    instance_id: String,
    report: BattleRunReport,
}

async fn run_match(
    runner: Arc<dyn BattleRunner>,
    battle_config: BattleConfig,
    scheduled_match: Match,
    warriors: Vec<Warrior>,
    instance: Arc<BattleInstance>,
) -> CompletedMatch {
    instance.set_status(BattleInstanceStatus::Running);
    let instance_id = instance.id().to_string();
    let report = tokio::task::spawn_blocking(move || runner.run(instance, warriors, battle_config))
        .await
        .expect("battle execution failed");

    CompletedMatch {
        scheduled_match,
        instance_id,
        report,
    }
}

fn default_battle_runner(
    instance: Arc<BattleInstance>,
    warriors: Vec<Warrior>,
    config: BattleConfig,
) -> BattleRunReport {
    let observer = Rc::new(RefCell::new(InstanceEventObserver::new(instance.clone())));
    let mut battle = Battle::new(config);
    battle.add_observer(observer);
    for warrior in warriors {
        battle.add_warrior(warrior);
    }

    battle.run_configured_rounds();
    let stats = battle.battle_stats().to_vec();
    let result = aggregate_battle_result(&stats);
    let result_message = battle_result_message(instance.warrior_ids(), &result);
    instance.set_status(BattleInstanceStatus::Complete);
    instance.emit_complete(result_message.clone());

    BattleRunReport {
        result,
        result_message,
        stats,
    }
}

fn initialise_standings(warrior_names: Vec<String>) -> HashMap<String, BattleStats> {
    warrior_names
        .into_iter()
        .map(|warrior_name| (warrior_name, BattleStats::default()))
        .collect()
}

fn ordered_participants(
    participants: &[String],
    standings: &HashMap<String, BattleStats>,
    seed_order: &HashMap<String, usize>,
) -> Vec<String> {
    let mut ordered = participants.to_vec();
    ordered.sort_by(|left, right| compare_warriors(left, right, standings, seed_order));
    ordered
}

fn compare_warriors(
    left: &str,
    right: &str,
    standings: &HashMap<String, BattleStats>,
    seed_order: &HashMap<String, usize>,
) -> Ordering {
    let left_stats = standings.get(left).cloned().unwrap_or_default();
    let right_stats = standings.get(right).cloned().unwrap_or_default();

    right_stats
        .score
        .cmp(&left_stats.score)
        .then_with(|| right_stats.wins.cmp(&left_stats.wins))
        .then_with(|| right_stats.draws.cmp(&left_stats.draws))
        .then_with(|| left_stats.losses.cmp(&right_stats.losses))
        .then_with(|| {
            seed_order
                .get(left)
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(&seed_order.get(right).copied().unwrap_or(usize::MAX))
        })
        .then_with(|| left.cmp(right))
}

fn merge_stats(total: &mut BattleStats, delta: &BattleStats) {
    total.wins += delta.wins;
    total.losses += delta.losses;
    total.draws += delta.draws;
    total.score += delta.score;
    total.processes_created += delta.processes_created;
    total.instructions_executed += delta.instructions_executed;
}

fn standings_vec(standings: &HashMap<String, BattleStats>) -> Vec<(String, BattleStats)> {
    let mut values: Vec<_> = standings
        .iter()
        .map(|(warrior_name, stats)| (warrior_name.clone(), stats.clone()))
        .collect();
    values.sort_by(|(left_name, left_stats), (right_name, right_stats)| {
        right_stats
            .score
            .cmp(&left_stats.score)
            .then_with(|| right_stats.wins.cmp(&left_stats.wins))
            .then_with(|| right_stats.draws.cmp(&left_stats.draws))
            .then_with(|| left_stats.losses.cmp(&right_stats.losses))
            .then_with(|| left_name.cmp(right_name))
    });
    values
}

fn pick_advancing_warrior(scheduled_match: &Match, result: &BattleResult) -> Option<String> {
    match result {
        BattleResult::Win { winner_id } => scheduled_match
            .warrior_ids
            .get(winner_id.checked_sub(1)? as usize)
            .cloned(),
        BattleResult::Draw { survivor_ids } => survivor_ids
            .iter()
            .filter_map(|warrior_id| {
                scheduled_match
                    .warrior_ids
                    .get(warrior_id.checked_sub(1)? as usize)
                    .cloned()
            })
            .next()
            .or_else(|| scheduled_match.warrior_ids.first().cloned()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
        Arc,
    };

    use corewar_core::{AddressingMode, Instruction, Modifier, Opcode};

    use super::{BattleRunReport, Tournament, TournamentConfig};
    use crate::scheduler::ScheduleStrategy;
    use corewar_core::Warrior;
    use corewar_protocol::BattleResultMsg;
    use corewar_vm::{
        battle::{BattleConfig, BattleStats},
        BattleResult, VmConfig,
    };

    fn dat(name: &str) -> Warrior {
        Warrior::new(
            name,
            vec![Instruction::new(
                Opcode::DAT,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        )
    }

    #[tokio::test]
    async fn tournament_completes_with_mock_battles() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let runner_calls = call_count.clone();
        let config = TournamentConfig {
            strategy: ScheduleStrategy::RoundRobin,
            rounds_per_match: 1,
            concurrency: 2,
        };
        let battle_config = BattleConfig {
            vm: VmConfig {
                core_size: 32,
                max_cycles: 4,
                max_processes: 8,
                max_length: 8,
                min_distance: 4,
                seed: 1,
            },
            rounds: 1,
            scoring_mode: corewar_vm::battle::ScoringMode::WinLoss,
        };

        let mut tournament = Tournament::new(config)
            .with_battle_config(battle_config)
            .with_runner(
                move |instance, warriors: Vec<Warrior>, _config: BattleConfig| {
                    runner_calls.fetch_add(1, AtomicOrdering::Relaxed);
                    let winner = if warriors[0].name <= warriors[1].name {
                        1
                    } else {
                        2
                    };
                    let result = BattleResult::Win { winner_id: winner };
                    let result_message = BattleResultMsg::Win {
                        winner: warriors[(winner - 1) as usize].name.clone(),
                    };
                    instance.set_status(crate::instance::BattleInstanceStatus::Complete);
                    instance.emit_complete(result_message.clone());

                    BattleRunReport {
                        result,
                        result_message,
                        stats: vec![
                            BattleStats {
                                warrior_id: 1,
                                wins: u64::from(winner == 1),
                                losses: u64::from(winner != 1),
                                score: u64::from(winner == 1),
                                ..BattleStats::default()
                            },
                            BattleStats {
                                warrior_id: 2,
                                wins: u64::from(winner == 2),
                                losses: u64::from(winner != 2),
                                score: u64::from(winner == 2),
                                ..BattleStats::default()
                            },
                        ],
                    }
                },
            );

        let result = tournament
            .run(vec![dat("alpha"), dat("beta"), dat("gamma")])
            .await;

        assert_eq!(call_count.load(AtomicOrdering::Relaxed), 3);
        assert_eq!(result.matches_played, 3);
        assert_eq!(result.standings[0].0, "alpha");
        assert_eq!(result.standings[0].1.wins, 2);
        assert_eq!(result.standings[2].0, "gamma");
    }
}
