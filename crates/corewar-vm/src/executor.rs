//! Instruction execution engine.

use corewar_core::{Core, EventBus};

use crate::config::VmConfig;

/// The execution engine that steps through processes.
pub struct Executor {
    pub core: Core,
    pub event_bus: EventBus,
    pub cycle: u64,
    config: VmConfig,
}

impl Executor {
    pub fn new(config: VmConfig) -> Self {
        let core = Core::new(config.core_size);
        Self {
            core,
            event_bus: EventBus::new(),
            cycle: 0,
            config,
        }
    }

    /// Execute one cycle (one instruction per living warrior).
    pub fn step(&mut self) -> bool {
        if self.cycle >= self.config.max_cycles {
            return false;
        }
        self.cycle += 1;
        // TODO: Implement round-robin execution
        true
    }

    /// Run until battle completion.
    pub fn run(&mut self) {
        while self.step() {}
    }
}
