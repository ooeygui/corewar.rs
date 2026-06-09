use std::panic::{catch_unwind, AssertUnwindSafe};

use corewar_core::{TimedEvent, Warrior};
use corewar_parser::parse_warrior;
use serde::Serialize;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::{
    battle::{Battle, BattleConfig},
    config::VmConfig,
    wasm_utils,
};

#[derive(Debug, Clone, Default)]
struct WarriorRuntimeStats {
    processes_created: u64,
    instructions_executed: u64,
}

#[derive(Debug, Serialize)]
struct CoreCellState {
    opcode: String,
    a_value: i32,
    b_value: i32,
    owner: Option<u32>,
}

#[derive(Debug, Serialize)]
struct WarriorInfo {
    id: u32,
    name: String,
    process_count: usize,
    alive: bool,
}

#[derive(Debug, Serialize)]
struct WarriorSummary {
    id: u32,
    name: String,
    alive: bool,
    process_count: usize,
    wins: u64,
    losses: u64,
    draws: u64,
    score: u64,
    processes_created: u64,
    instructions_executed: u64,
}

#[derive(Debug, Serialize)]
struct BattleSummary {
    complete: bool,
    cycle: u64,
    winner_id: Option<u32>,
    survivor_ids: Vec<u32>,
    max_cycles_reached: bool,
    warriors: Vec<WarriorSummary>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WasmBattle {
    config: BattleConfig,
    warriors: Vec<Warrior>,
    battle: Battle,
    events: Vec<TimedEvent>,
    runtime_stats: Vec<WarriorRuntimeStats>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WasmBattle {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(core_size: usize, max_cycles: u64, max_processes: usize) -> WasmBattle {
        wasm_utils::set_panic_hook();

        let mut vm = VmConfig::default();
        vm.core_size = core_size;
        vm.max_cycles = max_cycles;
        vm.max_processes = max_processes;

        let config = BattleConfig::from(vm);
        let battle = Battle::new(config.clone());

        Self {
            config,
            warriors: Vec::new(),
            battle,
            events: Vec::new(),
            runtime_stats: Vec::new(),
        }
    }

    pub fn load_warrior(&mut self, source: &str) -> Result<u32, JsValue> {
        let warrior = parse_warrior(source).map_err(|err| JsValue::from_str(&err.to_string()))?;
        let warrior_id = (self.warriors.len() + 1) as u32;

        let mut warriors = self.warriors.clone();
        warriors.push(warrior);
        let battle = Self::build_battle(self.config.clone(), &warriors)?;

        self.warriors = warriors;
        self.battle = battle;
        self.events.clear();
        self.runtime_stats = vec![WarriorRuntimeStats::default(); self.warriors.len()];

        Ok(warrior_id)
    }

    pub fn step(&mut self) -> bool {
        if self.is_complete() {
            return false;
        }

        let executing_ids = self.battle.executor.living_warrior_ids();
        let running = self.battle.executor.step();
        let events = self.battle.executor.event_bus.drain();
        self.record_cycle(&executing_ids, &events);

        running
    }

    pub fn run_cycles(&mut self, n: u32) -> bool {
        for _ in 0..n {
            if !self.step() {
                return false;
            }
        }

        !self.is_complete()
    }

    pub fn run_to_completion(&mut self) -> JsValue {
        while self.step() {}
        wasm_utils::to_js_value(&self.summary())
    }

    pub fn get_cycle(&self) -> u64 {
        self.battle.executor.cycle
    }

    pub fn get_core_state(&self) -> JsValue {
        let state: Vec<_> = self
            .battle
            .executor
            .core
            .snapshot()
            .into_iter()
            .map(|(instruction, owner)| CoreCellState {
                opcode: instruction.opcode.to_string(),
                a_value: instruction.a_value,
                b_value: instruction.b_value,
                owner,
            })
            .collect();
        wasm_utils::to_js_value(&state)
    }

    pub fn get_events_since(&mut self, last_cycle: u64) -> JsValue {
        let events: Vec<_> = self
            .events
            .iter()
            .copied()
            .filter(|event| event.cycle > last_cycle)
            .collect();
        wasm_utils::to_js_value(&events)
    }

    pub fn get_warrior_info(&self) -> JsValue {
        let info: Vec<_> = self
            .warriors
            .iter()
            .enumerate()
            .map(|(index, warrior)| {
                let id = (index + 1) as u32;
                let process_count = self.battle.executor.process_count(id).unwrap_or(0);
                WarriorInfo {
                    id,
                    name: warrior.name.clone(),
                    process_count,
                    alive: process_count > 0,
                }
            })
            .collect();
        wasm_utils::to_js_value(&info)
    }

    pub fn is_complete(&self) -> bool {
        self.battle.executor.cycle >= self.battle.executor.config().max_cycles
            || self.battle.executor.living_warrior_ids().len() <= 1
    }

    pub fn reset(&mut self) {
        self.battle = Self::build_battle(self.config.clone(), &self.warriors)
            .expect("reloading previously validated warriors should succeed");
        self.events.clear();
        self.runtime_stats = vec![WarriorRuntimeStats::default(); self.warriors.len()];
    }
}

impl WasmBattle {
    fn build_battle(config: BattleConfig, warriors: &[Warrior]) -> Result<Battle, JsValue> {
        let mut battle = Battle::new(config);
        for warrior in warriors.iter().cloned() {
            battle.add_warrior(warrior);
        }

        catch_unwind(AssertUnwindSafe(|| battle.load_warriors()))
            .map_err(|payload| JsValue::from_str(&panic_message(payload)))?;

        Ok(battle)
    }

    fn record_cycle(&mut self, executing_ids: &[u32], events: &[TimedEvent]) {
        for warrior_id in executing_ids {
            if let Some(stats) = self
                .runtime_stats
                .get_mut(warrior_id.saturating_sub(1) as usize)
            {
                stats.instructions_executed += 1;
            }
        }

        for event in events {
            if let corewar_core::CoreEvent::ProcessCreated { warrior_id, .. } = event.event {
                if let Some(stats) = self
                    .runtime_stats
                    .get_mut(warrior_id.saturating_sub(1) as usize)
                {
                    stats.processes_created += 1;
                }
            }
        }

        self.events.extend_from_slice(events);
    }

    fn summary(&self) -> BattleSummary {
        let survivor_ids = self.battle.executor.living_warrior_ids();
        let complete = self.is_complete();
        let winner_id = (survivor_ids.len() == 1).then_some(survivor_ids[0]);
        let max_cycles_reached =
            self.battle.executor.cycle >= self.battle.executor.config().max_cycles;

        let warriors = self
            .warriors
            .iter()
            .enumerate()
            .map(|(index, warrior)| {
                let id = (index + 1) as u32;
                let process_count = self.battle.executor.process_count(id).unwrap_or(0);
                let alive = process_count > 0;
                let (wins, losses, draws, score) = if complete {
                    if winner_id == Some(id) {
                        let score = match self.config.scoring_mode {
                            crate::battle::ScoringMode::WinLoss => 1,
                            crate::battle::ScoringMode::Points => 3,
                        };
                        (1, 0, 0, score)
                    } else if alive {
                        let score = match self.config.scoring_mode {
                            crate::battle::ScoringMode::WinLoss => 0,
                            crate::battle::ScoringMode::Points => 1,
                        };
                        (0, 0, 1, score)
                    } else {
                        (0, 1, 0, 0)
                    }
                } else {
                    (0, 0, 0, 0)
                };
                let runtime = &self.runtime_stats[index];

                WarriorSummary {
                    id,
                    name: warrior.name.clone(),
                    alive,
                    process_count,
                    wins,
                    losses,
                    draws,
                    score,
                    processes_created: runtime.processes_created,
                    instructions_executed: runtime.instructions_executed,
                }
            })
            .collect();

        BattleSummary {
            complete,
            cycle: self.battle.executor.cycle,
            winner_id,
            survivor_ids,
            max_cycles_reached,
            warriors,
        }
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "battle setup failed".to_owned()
}
