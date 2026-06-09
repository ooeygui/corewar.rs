#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use serde::{Deserialize, Serialize};

use crate::Instruction;

/// Events emitted by the core during execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreEvent {
    /// An instruction is about to be executed.
    Execute { address: usize, warrior_id: u32 },
    /// A memory cell was read while resolving an address.
    Read { address: usize, warrior_id: u32 },
    /// A memory cell was written to.
    MemoryWrite {
        address: usize,
        warrior_id: u32,
        instruction: Instruction,
    },
    /// A new process was spawned.
    ProcessCreated { warrior_id: u32, address: usize },
    /// A process was killed (executed DAT or similar).
    ProcessKilled { warrior_id: u32, address: usize },
    /// A warrior has been eliminated (no remaining processes).
    WarriorEliminated { warrior_id: u32 },
    /// A complete cycle has finished.
    CycleComplete,
}

/// Event type identifiers used by [`EventFilter`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreEventKind {
    Execute,
    Read,
    MemoryWrite,
    ProcessCreated,
    ProcessKilled,
    WarriorEliminated,
    CycleComplete,
}

impl CoreEventKind {
    const fn bit(self) -> u32 {
        match self {
            Self::Execute => 1 << 0,
            Self::Read => 1 << 1,
            Self::MemoryWrite => 1 << 2,
            Self::ProcessCreated => 1 << 3,
            Self::ProcessKilled => 1 << 4,
            Self::WarriorEliminated => 1 << 5,
            Self::CycleComplete => 1 << 6,
        }
    }
}

impl CoreEvent {
    pub const fn kind(&self) -> CoreEventKind {
        match self {
            Self::Execute { .. } => CoreEventKind::Execute,
            Self::Read { .. } => CoreEventKind::Read,
            Self::MemoryWrite { .. } => CoreEventKind::MemoryWrite,
            Self::ProcessCreated { .. } => CoreEventKind::ProcessCreated,
            Self::ProcessKilled { .. } => CoreEventKind::ProcessKilled,
            Self::WarriorEliminated { .. } => CoreEventKind::WarriorEliminated,
            Self::CycleComplete => CoreEventKind::CycleComplete,
        }
    }

    pub const fn warrior_id(&self) -> Option<u32> {
        match *self {
            Self::Execute { warrior_id, .. }
            | Self::Read { warrior_id, .. }
            | Self::MemoryWrite { warrior_id, .. }
            | Self::ProcessCreated { warrior_id, .. }
            | Self::ProcessKilled { warrior_id, .. }
            | Self::WarriorEliminated { warrior_id } => Some(warrior_id),
            Self::CycleComplete => None,
        }
    }
}

/// An execution event tagged with the cycle on which it occurred.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimedEvent {
    pub cycle: u64,
    pub event: CoreEvent,
}

/// Filter for subscribing to a subset of event kinds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventFilter {
    mask: u32,
}

impl EventFilter {
    pub const fn all() -> Self {
        Self { mask: (1 << 7) - 1 }
    }

    pub const fn none() -> Self {
        Self { mask: 0 }
    }

    pub const fn only(kind: CoreEventKind) -> Self {
        Self { mask: kind.bit() }
    }

    pub const fn with_kind(self, kind: CoreEventKind) -> Self {
        Self {
            mask: self.mask | kind.bit(),
        }
    }

    pub const fn matches_event(&self, event: &CoreEvent) -> bool {
        self.matches_kind(event.kind())
    }

    pub const fn matches(&self, event: &TimedEvent) -> bool {
        self.matches_event(&event.event)
    }

    pub const fn matches_kind(&self, kind: CoreEventKind) -> bool {
        self.mask & kind.bit() != 0
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::all()
    }
}

/// Simple event bus for collecting timed events during execution.
pub struct EventBus {
    cycle: Cell<u64>,
    events: RefCell<Vec<TimedEvent>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            cycle: Cell::new(0),
            events: RefCell::new(Vec::new()),
        }
    }

    pub fn cycle(&self) -> u64 {
        self.cycle.get()
    }

    pub fn set_cycle(&self, cycle: u64) {
        self.cycle.set(cycle);
    }

    pub fn emit(&self, event: CoreEvent) {
        self.emit_timed(TimedEvent {
            cycle: self.cycle(),
            event,
        });
    }

    pub fn emit_timed(&self, event: TimedEvent) {
        self.events.borrow_mut().push(event);
    }

    pub fn drain(&self) -> Vec<TimedEvent> {
        core::mem::take(&mut *self.events.borrow_mut())
    }

    pub fn is_empty(&self) -> bool {
        self.events.borrow().is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.borrow().len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreEvent, CoreEventKind, EventBus, EventFilter, TimedEvent};
    use crate::{AddressingMode, Instruction, Modifier, Opcode};

    #[test]
    fn event_bus_records_and_drains_events() {
        let bus = EventBus::new();
        bus.set_cycle(7);
        bus.emit(CoreEvent::CycleComplete);
        bus.emit(CoreEvent::WarriorEliminated { warrior_id: 3 });

        assert_eq!(bus.len(), 2);
        assert!(!bus.is_empty());
        assert_eq!(
            bus.drain(),
            vec![
                TimedEvent {
                    cycle: 7,
                    event: CoreEvent::CycleComplete,
                },
                TimedEvent {
                    cycle: 7,
                    event: CoreEvent::WarriorEliminated { warrior_id: 3 },
                },
            ]
        );
        assert!(bus.is_empty());
    }

    #[test]
    fn event_filter_matches_expected_kinds() {
        let filter = EventFilter::only(CoreEventKind::Execute).with_kind(CoreEventKind::Read);
        let instruction = Instruction::new(
            Opcode::MOV,
            Modifier::I,
            AddressingMode::Direct,
            0,
            AddressingMode::Direct,
            1,
        );

        assert!(filter.matches(&TimedEvent {
            cycle: 1,
            event: CoreEvent::Execute {
                address: 4,
                warrior_id: 2,
            },
        }));
        assert!(filter.matches(&TimedEvent {
            cycle: 1,
            event: CoreEvent::Read {
                address: 4,
                warrior_id: 2,
            },
        }));
        assert!(!filter.matches(&TimedEvent {
            cycle: 1,
            event: CoreEvent::MemoryWrite {
                address: 4,
                warrior_id: 2,
                instruction,
            },
        }));
    }
}
