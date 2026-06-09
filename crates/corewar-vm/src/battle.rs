//! Battle setup and management.

use corewar_core::Warrior;

use crate::config::VmConfig;
use crate::executor::Executor;

/// Result of a completed battle.
#[derive(Debug, Clone)]
pub enum BattleResult {
    /// A single warrior survived.
    Win { winner_id: u32 },
    /// Multiple warriors survived (or max cycles reached).
    Draw { survivor_ids: Vec<u32> },
}

/// A battle between warriors in a shared core.
pub struct Battle {
    pub executor: Executor,
    warriors: Vec<Warrior>,
}

impl Battle {
    /// Create a new battle with the given configuration.
    pub fn new(config: VmConfig) -> Self {
        Self {
            executor: Executor::new(config),
            warriors: Vec::new(),
        }
    }

    /// Add a warrior to the battle. Must be called before `run`.
    pub fn add_warrior(&mut self, warrior: Warrior) {
        self.warriors.push(warrior);
    }

    /// Run the battle to completion and return the result.
    pub fn run(&mut self) -> BattleResult {
        // TODO: Load warriors into core, execute, determine winner
        self.executor.run();
        BattleResult::Draw {
            survivor_ids: self.warriors.iter().map(|w| w.id).collect(),
        }
    }
}

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
