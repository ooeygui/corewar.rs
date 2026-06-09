use corewar_core::{Core, CoreEvent, TimedEvent};
use serde::{Deserialize, Serialize};

use crate::battle::{Battle, BattleObserver, BattleSetup};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::{cell::RefCell, fs::File, io, path::Path, rc::Rc};

/// Serializable replay payload for a single battle execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub initial_setup: BattleSetup,
    pub events: Vec<TimedEvent>,
}

impl Replay {
    pub fn new(initial_setup: BattleSetup, events: Vec<TimedEvent>) -> Self {
        Self {
            initial_setup,
            events,
        }
    }

    pub fn core_at(&self, cycle: u64) -> Core {
        let mut core = Core::new(self.initial_setup.core_size);
        for warrior in &self.initial_setup.warriors {
            for (offset, instruction) in warrior.instructions.iter().copied().enumerate() {
                core.write_with_owner(warrior.start + offset, instruction, warrior.warrior_id);
            }
        }

        for event in &self.events {
            if event.cycle > cycle {
                break;
            }

            if let CoreEvent::MemoryWrite {
                address,
                warrior_id,
                instruction,
            } = event.event
            {
                core.write_with_owner(address, instruction, warrior_id);
            }
        }

        core
    }

    #[cfg(feature = "std")]
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self).map_err(io::Error::other)
    }

    #[cfg(feature = "std")]
    pub fn load_from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::open(path)?;
        serde_json::from_reader(file).map_err(io::Error::other)
    }
}

/// Observer that builds a replay from battle setup and emitted events.
#[derive(Debug, Default, Clone)]
pub struct ReplayBuilder {
    initial_setup: Option<BattleSetup>,
    events: Vec<TimedEvent>,
}

impl ReplayBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "std")]
    pub fn attach(battle: &mut Battle) -> Rc<RefCell<Self>> {
        let builder = Rc::new(RefCell::new(Self::new()));
        battle.add_observer(builder.clone());
        builder
    }

    pub fn build(&self) -> Option<Replay> {
        self.initial_setup
            .clone()
            .map(|initial_setup| Replay::new(initial_setup, self.events.clone()))
    }
}

impl BattleObserver for ReplayBuilder {
    fn on_battle_loaded(&mut self, setup: &BattleSetup) {
        self.initial_setup = Some(setup.clone());
        self.events.clear();
    }

    fn on_cycle(&mut self, _cycle: u64, events: &[TimedEvent]) {
        self.events.extend_from_slice(events);
    }
}

#[cfg(test)]
mod tests {
    use corewar_core::{AddressingMode, CoreEvent, Instruction, Modifier, Opcode};

    use super::Replay;
    use crate::battle::{BattleSetup, WarriorPlacement};

    #[test]
    fn replay_reconstructs_core_memory_from_writes() {
        let imp = Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        );
        let dat = Instruction::new(
            Opcode::DAT,
            Modifier::F,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            0,
        );
        let replay = Replay::new(
            BattleSetup {
                core_size: 8,
                warriors: vec![WarriorPlacement {
                    warrior_id: 1,
                    name: "imp".into(),
                    start: 0,
                    entry_pc: 0,
                    instructions: vec![imp],
                }],
            },
            vec![corewar_core::TimedEvent {
                cycle: 0,
                event: CoreEvent::MemoryWrite {
                    address: 1,
                    warrior_id: 1,
                    instruction: dat,
                },
            }],
        );

        let core = replay.core_at(0);
        assert_eq!(*core.read(0), imp);
        assert_eq!(*core.read(1), dat);
    }
}
