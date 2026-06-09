#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::event::{CoreEvent, EventBus};
use crate::instruction::Instruction;

/// The core memory array — the battlefield where warriors fight.
pub struct Core {
    memory: Vec<Instruction>,
    ownership: Vec<Option<u32>>,
    size: usize,
}

impl Core {
    /// Create a new core of the given size, initialized to DAT 0, 0.
    pub fn new(size: usize) -> Self {
        Self {
            memory: vec![Instruction::default(); size],
            ownership: vec![None; size],
            size,
        }
    }

    /// Returns the core size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Normalize an address into the core, wrapping negative offsets.
    pub fn normalize(&self, addr: i32) -> usize {
        let size = self.size as i64;
        i64::from(addr).rem_euclid(size) as usize
    }

    /// Read an instruction at the given address (wraps around).
    pub fn read(&self, addr: usize) -> &Instruction {
        &self.memory[addr % self.size]
    }

    /// Read an instruction mutably at the given address (wraps around).
    pub fn read_mut(&mut self, addr: usize) -> &mut Instruction {
        let normalized = addr % self.size;
        &mut self.memory[normalized]
    }

    /// Write an instruction at the given address and update ownership.
    pub fn write_with_owner(&mut self, addr: usize, instr: Instruction, warrior_id: u32) {
        let normalized = addr % self.size;
        self.memory[normalized] = instr;
        self.ownership[normalized] = Some(warrior_id);
    }

    /// Write an instruction at the given address, emitting an event.
    pub fn write(
        &mut self,
        addr: usize,
        instr: Instruction,
        warrior_id: u32,
        event_bus: &EventBus,
    ) {
        let normalized = addr % self.size;
        self.write_with_owner(normalized, instr, warrior_id);
        event_bus.emit(CoreEvent::MemoryWrite {
            address: normalized,
            warrior_id,
            instruction: instr,
        });
    }

    /// Direct mutable access without events (for loading warriors).
    pub fn load(&mut self, addr: usize, instr: Instruction) {
        let normalized = addr % self.size;
        self.memory[normalized] = instr;
        self.ownership[normalized] = None;
    }

    /// Get a slice of the entire memory (for visualization snapshots).
    pub fn as_slice(&self) -> &[Instruction] {
        &self.memory
    }

    /// Get a copy of memory with ownership metadata for visualization.
    pub fn snapshot(&self) -> Vec<(Instruction, Option<u32>)> {
        self.memory
            .iter()
            .copied()
            .zip(self.ownership.iter().copied())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Core;
    use crate::{
        event::TimedEvent, AddressingMode, CoreEvent, EventBus, Instruction, Modifier, Opcode,
    };

    fn mov_instruction(value: i32) -> Instruction {
        Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            value,
            AddressingMode::Direct,
            value,
        )
    }

    #[test]
    fn normalize_wraps_negative_addresses() {
        let core = Core::new(8);

        assert_eq!(core.normalize(-1), 7);
        assert_eq!(core.normalize(-9), 7);
        assert_eq!(core.normalize(10), 2);
    }

    #[test]
    fn read_mut_updates_wrapped_cell() {
        let mut core = Core::new(4);
        core.read_mut(5).a_value = 11;

        assert_eq!(core.read(1).a_value, 11);
    }

    #[test]
    fn write_tracks_owner_and_snapshot() {
        let mut core = Core::new(4);
        let bus = EventBus::new();
        let instr = mov_instruction(3);

        core.write(6, instr, 42, &bus);

        assert_eq!(core.read(2), &instr);
        assert_eq!(core.snapshot()[2], (instr, Some(42)));
        assert_eq!(
            bus.drain(),
            vec![TimedEvent {
                cycle: 0,
                event: CoreEvent::MemoryWrite {
                    address: 2,
                    warrior_id: 42,
                    instruction: instr,
                },
            }]
        );
    }

    #[test]
    fn load_clears_owner_metadata() {
        let mut core = Core::new(2);
        let instr = mov_instruction(9);

        core.write_with_owner(0, instr, 7);
        core.load(0, Instruction::default());

        assert_eq!(core.snapshot()[0], (Instruction::default(), None));
    }
}
