#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// Events emitted by the core during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    /// A memory cell was written to.
    MemoryWrite { address: usize, warrior_id: u32 },
    /// A new process was spawned.
    ProcessCreated { warrior_id: u32, address: usize },
    /// A process was killed (executed DAT or similar).
    ProcessKilled { warrior_id: u32, address: usize },
    /// A warrior has been eliminated (no remaining processes).
    WarriorEliminated { warrior_id: u32 },
    /// A complete cycle has finished.
    CycleComplete { cycle: u64 },
}

/// Simple event bus for collecting events during execution.
pub struct EventBus {
    events: Vec<CoreEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn emit(&self, _event: CoreEvent) {
        // TODO: Use interior mutability (Cell/RefCell) or channel
        // For now this is a placeholder for the architecture
    }

    pub fn drain(&mut self) -> Vec<CoreEvent> {
        core::mem::take(&mut self.events)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
