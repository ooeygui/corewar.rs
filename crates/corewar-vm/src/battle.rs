//! Battle setup and management.

use core::cell::RefCell;

use corewar_core::{CoreEvent, EventFilter, TimedEvent, Warrior};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::config::VmConfig;
use crate::executor::Executor;

#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::{rc::Rc, string::String};

/// Result of a completed battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleResult {
    /// A single warrior survived.
    Win { winner_id: u32 },
    /// Multiple warriors survived (or max cycles reached).
    Draw { survivor_ids: Vec<u32> },
}

/// Battle scoring model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoringMode {
    /// Count only outright wins.
    WinLoss,
    /// pMARS-style scoring: wins are worth 3, draws are worth 1.
    Points,
}

impl Default for ScoringMode {
    fn default() -> Self {
        Self::WinLoss
    }
}

/// Battle-specific configuration layered on top of VM execution settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleConfig {
    #[serde(flatten)]
    pub vm: VmConfig,
    pub rounds: usize,
    pub scoring_mode: ScoringMode,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            vm: VmConfig::default(),
            rounds: 1,
            scoring_mode: ScoringMode::WinLoss,
        }
    }
}

impl From<VmConfig> for BattleConfig {
    fn from(vm: VmConfig) -> Self {
        Self {
            vm,
            ..Self::default()
        }
    }
}

/// Aggregated per-warrior battle statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleStats {
    pub warrior_id: u32,
    pub wins: u64,
    pub losses: u64,
    pub draws: u64,
    pub score: u64,
    pub processes_created: u64,
    pub instructions_executed: u64,
}

/// Result of an individual round in a multi-round battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundResult {
    pub winner: Option<u32>,
    pub survivors: Vec<u32>,
    pub cycles_used: u64,
}

/// Initial warrior placement information for replay and visualization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarriorPlacement {
    pub warrior_id: u32,
    pub name: String,
    pub start: usize,
    pub entry_pc: usize,
    pub instructions: Vec<corewar_core::Instruction>,
}

/// Serializable battle setup snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetup {
    pub core_size: usize,
    pub warriors: Vec<WarriorPlacement>,
}

/// Observer for battle execution.
pub trait BattleObserver {
    fn on_battle_loaded(&mut self, _setup: &BattleSetup) {}

    fn on_cycle(&mut self, cycle: u64, events: &[TimedEvent]);

    fn on_round_complete(&mut self, _result: &RoundResult) {}
}

type SharedBattleObserver = Rc<RefCell<dyn BattleObserver>>;

struct ObserverRegistration {
    filter: EventFilter,
    observer: SharedBattleObserver,
}

/// A battle between warriors in a shared core.
pub struct Battle {
    pub executor: Executor,
    config: BattleConfig,
    warriors: Vec<Warrior>,
    stats: Vec<BattleStats>,
    observers: Vec<ObserverRegistration>,
    loaded: bool,
    setup: Option<BattleSetup>,
}

impl Battle {
    /// Create a new battle with the given configuration.
    pub fn new(config: impl Into<BattleConfig>) -> Self {
        let config = config.into();
        Self {
            executor: Executor::new(config.vm.clone()),
            config,
            warriors: Vec::new(),
            stats: Vec::new(),
            observers: Vec::new(),
            loaded: false,
            setup: None,
        }
    }

    /// Add a warrior to the battle. Must be called before `run`.
    pub fn add_warrior(&mut self, warrior: Warrior) {
        self.warriors.push(warrior);
        self.loaded = false;
        self.setup = None;
    }

    pub fn add_observer<O>(&mut self, observer: Rc<RefCell<O>>)
    where
        O: BattleObserver + 'static,
    {
        self.add_observer_with_filter(observer, EventFilter::all());
    }

    pub fn add_observer_with_filter<O>(&mut self, observer: Rc<RefCell<O>>, filter: EventFilter)
    where
        O: BattleObserver + 'static,
    {
        let observer: SharedBattleObserver = observer;
        if let Some(setup) = &self.setup {
            observer.borrow_mut().on_battle_loaded(setup);
        }
        self.observers
            .push(ObserverRegistration { filter, observer });
    }

    pub fn config(&self) -> &BattleConfig {
        &self.config
    }

    pub fn battle_stats(&self) -> &[BattleStats] {
        &self.stats
    }

    pub fn setup(&self) -> Option<&BattleSetup> {
        self.setup.as_ref()
    }

    pub fn load_warriors(&mut self) {
        if self.loaded {
            return;
        }

        let config = self.executor.config().clone();
        if self.warriors.is_empty() {
            self.setup = Some(BattleSetup {
                core_size: config.core_size,
                warriors: Vec::new(),
            });
            self.loaded = true;
            self.notify_battle_loaded();
            return;
        }

        let starts = place_warriors(&self.warriors, &config);
        let mut placements = Vec::with_capacity(self.warriors.len());

        for (index, warrior) in self.warriors.iter_mut().enumerate() {
            warrior.id = (index + 1) as u32;
            let start = starts[index];

            for (offset, instruction) in warrior.instructions.iter().copied().enumerate() {
                self.executor
                    .core
                    .write_with_owner(start + offset, instruction, warrior.id);
            }

            let entry_pc = (start + warrior.start_offset) % config.core_size;
            self.executor.add_warrior(warrior.id, entry_pc);
            placements.push(WarriorPlacement {
                warrior_id: warrior.id,
                name: warrior.name.clone(),
                start,
                entry_pc,
                instructions: warrior.instructions.clone(),
            });
        }

        self.setup = Some(BattleSetup {
            core_size: config.core_size,
            warriors: placements,
        });
        self.loaded = true;
        self.notify_battle_loaded();
    }

    /// Run a single round to completion and return the result.
    pub fn run(&mut self) -> BattleResult {
        self.reset_stats();
        let round = self.run_round(self.config.vm.seed);
        self.apply_round_result(&round);
        BattleResult::from(round)
    }

    /// Run the configured number of rounds.
    pub fn run_configured_rounds(&mut self) -> Vec<RoundResult> {
        self.run_rounds(self.config.rounds)
    }

    /// Run `n` rounds with deterministic seed variation.
    pub fn run_rounds(&mut self, n: usize) -> Vec<RoundResult> {
        self.reset_stats();
        let mut rounds = Vec::with_capacity(n);
        for round_index in 0..n {
            let round = self.run_round(self.round_seed(round_index));
            self.apply_round_result(&round);
            rounds.push(round);
        }
        rounds
    }

    fn run_round(&mut self, seed: u64) -> RoundResult {
        self.prepare_round(seed);
        self.load_warriors();
        self.execute_loaded_round()
    }

    fn prepare_round(&mut self, seed: u64) {
        let mut config = self.config.vm.clone();
        config.seed = seed;
        self.executor = Executor::new(config);
        self.loaded = false;
        self.setup = None;
    }

    fn execute_loaded_round(&mut self) -> RoundResult {
        while !self.round_finished() {
            let executing_ids = self.executor.living_warrior_ids();
            for warrior_id in executing_ids {
                if let Some(stats) = self.stats_mut(warrior_id) {
                    stats.instructions_executed += 1;
                }
            }

            self.executor.step();
            let events = self.executor.event_bus.drain();
            self.record_events(&events);
            self.dispatch_cycle_events(self.executor.cycle, &events);
        }

        let survivors = self.executor.living_warrior_ids();
        let winner = (survivors.len() == 1).then_some(survivors[0]);
        let result = RoundResult {
            winner,
            survivors,
            cycles_used: self.executor.cycle,
        };

        for registration in &self.observers {
            registration
                .observer
                .borrow_mut()
                .on_round_complete(&result);
        }

        result
    }

    fn round_finished(&self) -> bool {
        self.executor.cycle >= self.executor.config().max_cycles
            || self.executor.living_warrior_ids().len() <= 1
    }

    fn reset_stats(&mut self) {
        self.stats = (0..self.warriors.len())
            .map(|index| BattleStats {
                warrior_id: (index + 1) as u32,
                ..BattleStats::default()
            })
            .collect();
    }

    fn round_seed(&self, round_index: usize) -> u64 {
        self.config.vm.seed.wrapping_add(round_index as u64)
    }

    fn record_events(&mut self, events: &[TimedEvent]) {
        for event in events {
            if let CoreEvent::ProcessCreated { warrior_id, .. } = event.event {
                if let Some(stats) = self.stats_mut(warrior_id) {
                    stats.processes_created += 1;
                }
            }
        }
    }

    fn notify_battle_loaded(&mut self) {
        let Some(setup) = &self.setup else {
            return;
        };

        for registration in &self.observers {
            registration.observer.borrow_mut().on_battle_loaded(setup);
        }
    }

    fn dispatch_cycle_events(&self, cycle: u64, events: &[TimedEvent]) {
        for registration in &self.observers {
            if registration.filter == EventFilter::all() {
                registration.observer.borrow_mut().on_cycle(cycle, events);
                continue;
            }

            let filtered: Vec<_> = events
                .iter()
                .copied()
                .filter(|event| registration.filter.matches(event))
                .collect();
            if !filtered.is_empty() {
                registration
                    .observer
                    .borrow_mut()
                    .on_cycle(cycle, &filtered);
            }
        }
    }

    fn apply_round_result(&mut self, result: &RoundResult) {
        let mut survivors = vec![false; self.stats.len()];
        for &warrior_id in &result.survivors {
            if let Some(survived) = survivors.get_mut(warrior_id.saturating_sub(1) as usize) {
                *survived = true;
            }
        }

        for stats in &mut self.stats {
            let is_winner = Some(stats.warrior_id) == result.winner;
            let survived = survivors
                .get(stats.warrior_id.saturating_sub(1) as usize)
                .copied()
                .unwrap_or(false);

            if is_winner {
                stats.wins += 1;
                stats.score += match self.config.scoring_mode {
                    ScoringMode::WinLoss => 1,
                    ScoringMode::Points => 3,
                };
            } else if survived {
                stats.draws += 1;
                if self.config.scoring_mode == ScoringMode::Points {
                    stats.score += 1;
                }
            } else {
                stats.losses += 1;
            }
        }
    }

    fn stats_mut(&mut self, warrior_id: u32) -> Option<&mut BattleStats> {
        self.stats.get_mut(warrior_id.checked_sub(1)? as usize)
    }
}

impl From<RoundResult> for BattleResult {
    fn from(result: RoundResult) -> Self {
        match result.winner {
            Some(winner_id) => Self::Win { winner_id },
            None => Self::Draw {
                survivor_ids: result.survivors,
            },
        }
    }
}

fn place_warriors(warriors: &[Warrior], config: &VmConfig) -> Vec<usize> {
    for warrior in warriors {
        assert!(
            warrior.len() <= config.max_length,
            "warrior exceeds VM max length"
        );
    }

    let occupied_cells: usize = warriors.iter().map(Warrior::len).sum();
    assert!(
        occupied_cells <= config.core_size,
        "warriors exceed core size"
    );

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut candidates: Vec<usize> = (0..config.core_size).collect();
    candidates.shuffle(&mut rng);

    let mut min_distance = effective_min_distance(config, warriors.len());
    loop {
        if let Some(starts) = try_place_warriors(warriors, &candidates, config, min_distance) {
            return starts;
        }
        if min_distance == 0 {
            panic!("unable to place warriors while respecting minimum distance");
        }
        min_distance -= 1;
    }
}

fn try_place_warriors(
    warriors: &[Warrior],
    candidates: &[usize],
    config: &VmConfig,
    min_distance: usize,
) -> Option<Vec<usize>> {
    let mut starts = Vec::with_capacity(warriors.len());
    let mut occupied = vec![false; config.core_size];

    for warrior in warriors {
        let start = candidates.iter().copied().find(|candidate| {
            placement_is_valid(
                *candidate,
                warrior.len(),
                &starts,
                &occupied,
                config.core_size,
                min_distance,
            )
        })?;

        starts.push(start);
        for offset in 0..warrior.len() {
            occupied[(start + offset) % config.core_size] = true;
        }
    }

    Some(starts)
}

fn effective_min_distance(config: &VmConfig, warrior_count: usize) -> usize {
    if warrior_count == 0 {
        return config.min_distance;
    }

    let scaled = config.core_size / warrior_count;
    config.min_distance.min(scaled)
}

fn placement_is_valid(
    candidate: usize,
    warrior_len: usize,
    starts: &[usize],
    occupied: &[bool],
    core_size: usize,
    min_distance: usize,
) -> bool {
    if (0..warrior_len).any(|offset| occupied[(candidate + offset) % core_size]) {
        return false;
    }

    starts
        .iter()
        .all(|start| ring_distance(candidate, *start, core_size) >= min_distance)
}

fn ring_distance(a: usize, b: usize, core_size: usize) -> usize {
    let distance = a.abs_diff(b);
    distance.min(core_size - distance)
}

#[cfg(test)]
mod tests {
    use super::{Battle, BattleConfig, BattleResult, ScoringMode};
    use crate::config::VmConfig;
    use corewar_core::{AddressingMode, Instruction, Modifier, Opcode, Warrior};

    fn vm_config() -> VmConfig {
        VmConfig {
            core_size: 64,
            max_cycles: 8,
            max_processes: 16,
            max_length: 8,
            min_distance: 8,
            seed: 5,
        }
    }

    fn battle_config(scoring_mode: ScoringMode, rounds: usize) -> BattleConfig {
        BattleConfig {
            vm: vm_config(),
            rounds,
            scoring_mode,
        }
    }

    fn imp(name: &str) -> Warrior {
        Warrior::new(
            name,
            vec![Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                1,
            )],
        )
    }

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

    fn nop(name: &str) -> Warrior {
        Warrior::new(
            name,
            vec![Instruction::new(
                Opcode::NOP,
                Modifier::F,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                0,
            )],
        )
    }

    #[test]
    fn two_warrior_battle_completes() {
        let mut battle = Battle::new(vm_config());
        battle.add_warrior(dat("dat"));
        battle.add_warrior(nop("nop"));

        assert_eq!(battle.run(), BattleResult::Win { winner_id: 2 });
        assert_eq!(battle.battle_stats()[0].losses, 1);
        assert_eq!(battle.battle_stats()[1].wins, 1);
    }

    #[test]
    fn ten_warrior_battle_tracks_points_scoring() {
        let mut config = vm_config();
        config.core_size = 100;
        config.min_distance = 20;
        let mut battle = Battle::new(BattleConfig {
            vm: config,
            rounds: 3,
            scoring_mode: ScoringMode::Points,
        });
        battle.add_warrior(imp("imp"));
        for index in 1..10 {
            battle.add_warrior(dat(&format!("dat-{index}")));
        }

        let rounds = battle.run_rounds(3);

        assert_eq!(rounds.len(), 3);
        assert!(rounds.iter().all(|round| round.winner == Some(1)));
        assert_eq!(battle.battle_stats()[0].wins, 3);
        assert_eq!(battle.battle_stats()[0].score, 9);
        assert!(battle.battle_stats()[1..]
            .iter()
            .all(|stats| stats.losses == 3));
    }

    #[test]
    fn multi_round_aggregation_uses_configured_rounds() {
        let mut battle = Battle::new(battle_config(ScoringMode::WinLoss, 4));
        battle.add_warrior(dat("dat"));
        battle.add_warrior(imp("imp"));

        let rounds = battle.run_configured_rounds();

        assert_eq!(rounds.len(), 4);
        assert!(rounds.iter().all(|round| round.winner == Some(2)));
        assert_eq!(battle.battle_stats()[1].wins, 4);
        assert_eq!(battle.battle_stats()[1].score, 4);
        assert_eq!(battle.battle_stats()[0].losses, 4);
    }

    #[test]
    fn imp_vs_imp_always_draws() {
        let mut battle = Battle::new(BattleConfig {
            vm: vm_config(),
            rounds: 5,
            scoring_mode: ScoringMode::Points,
        });
        battle.add_warrior(imp("imp-1"));
        battle.add_warrior(imp("imp-2"));

        let rounds = battle.run_rounds(5);

        assert!(rounds.iter().all(|round| round.winner.is_none()));
        assert!(rounds.iter().all(|round| round.survivors == vec![1, 2]));
        assert_eq!(battle.battle_stats()[0].draws, 5);
        assert_eq!(battle.battle_stats()[1].draws, 5);
        assert_eq!(battle.battle_stats()[0].score, 5);
        assert_eq!(battle.battle_stats()[1].score, 5);
    }
}
