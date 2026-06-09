#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::event::{CoreEvent, EventBus};
use crate::instruction::Instruction;

/// The core memory array — the battlefield where warriors fight.
pub struct Core {
    memory: Vec<Instruction>,
    size: usize,
}

impl Core {
    /// Create a new core of the given size, initialized to DAT 0, 0.
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![Instruction::default(); size],
            size,
        }
    }

    /// Returns the core size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Read an instruction at the given address (wraps around).
    pub fn read(&self, addr: usize) -> &Instruction {
        &self.memory[addr % self.size]
    }

    /// Write an instruction at the given address, emitting an event.
    pub fn write(&mut self, addr: usize, instr: Instruction, warrior_id: u32, event_bus: &EventBus) {
        let normalized = addr % self.size;
        self.memory[normalized] = instr;
        event_bus.emit(CoreEvent::MemoryWrite {
            address: normalized,
            warrior_id,
        });
    }

    /// Direct mutable access without events (for loading warriors).
    pub fn load(&mut self, addr: usize, instr: Instruction) {
        let normalized = addr % self.size;
        self.memory[normalized] = instr;
    }

    /// Get a slice of the entire memory (for visualization snapshots).
    pub fn as_slice(&self) -> &[Instruction] {
        &self.memory
    }
}
