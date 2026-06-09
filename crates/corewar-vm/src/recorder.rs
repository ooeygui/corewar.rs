use corewar_core::TimedEvent;
use serde::{Deserialize, Serialize};

use crate::battle::BattleObserver;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

/// Serializable snapshot of a bounded event recorder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSnapshot {
    pub max_events: usize,
    pub dropped_events: u64,
    pub events: Vec<TimedEvent>,
}

/// In-memory bounded recorder for timed battle events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecorder {
    max_events: usize,
    dropped_events: u64,
    events: Vec<TimedEvent>,
}

impl EventRecorder {
    pub fn new(max_events: usize) -> Self {
        Self {
            max_events,
            dropped_events: 0,
            events: Vec::new(),
        }
    }

    pub fn max_events(&self) -> usize {
        self.max_events
    }

    pub fn dropped_events(&self) -> u64 {
        self.dropped_events
    }

    pub fn events(&self) -> &[TimedEvent] {
        &self.events
    }

    pub fn events_since(&self, cycle: u64) -> &[TimedEvent] {
        let index = self.events.partition_point(|event| event.cycle <= cycle);
        &self.events[index..]
    }

    pub fn events_for_warrior(&self, id: u32) -> Vec<&TimedEvent> {
        self.events
            .iter()
            .filter(|event| event.event.warrior_id() == Some(id))
            .collect()
    }

    pub fn snapshot(&self) -> EventSnapshot {
        EventSnapshot {
            max_events: self.max_events,
            dropped_events: self.dropped_events,
            events: self.events.clone(),
        }
    }

    fn record(&mut self, event: TimedEvent) {
        if self.max_events == 0 {
            self.dropped_events += 1;
            return;
        }

        if self.events.len() == self.max_events {
            self.events.rotate_left(1);
            if let Some(last) = self.events.last_mut() {
                *last = event;
            }
            self.dropped_events += 1;
        } else {
            self.events.push(event);
        }
    }
}

impl BattleObserver for EventRecorder {
    fn on_cycle(&mut self, _cycle: u64, events: &[TimedEvent]) {
        for event in events.iter().copied() {
            self.record(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;
    use std::rc::Rc;

    use corewar_core::{
        AddressingMode, CoreEvent, CoreEventKind, EventFilter, Instruction, Modifier, Opcode,
        TimedEvent, Warrior,
    };

    use super::EventRecorder;
    use crate::{Battle, BattleObserver, VmConfig};

    fn config() -> VmConfig {
        VmConfig {
            core_size: 32,
            max_cycles: 4,
            max_processes: 8,
            max_length: 8,
            min_distance: 4,
            seed: 5,
        }
    }

    #[test]
    fn recorder_drops_oldest_events_when_full() {
        let mut recorder = EventRecorder::new(2);
        recorder.on_cycle(
            0,
            &[
                TimedEvent {
                    cycle: 0,
                    event: CoreEvent::CycleComplete,
                },
                TimedEvent {
                    cycle: 1,
                    event: CoreEvent::CycleComplete,
                },
                TimedEvent {
                    cycle: 2,
                    event: CoreEvent::CycleComplete,
                },
            ],
        );

        assert_eq!(recorder.dropped_events(), 1);
        assert_eq!(
            recorder.events(),
            &[
                TimedEvent {
                    cycle: 1,
                    event: CoreEvent::CycleComplete,
                },
                TimedEvent {
                    cycle: 2,
                    event: CoreEvent::CycleComplete,
                },
            ]
        );
        assert_eq!(
            recorder.events_since(1),
            &[TimedEvent {
                cycle: 2,
                event: CoreEvent::CycleComplete,
            }]
        );
    }

    #[test]
    fn battle_filters_events_for_subscribers() {
        let recorder = Rc::new(RefCell::new(EventRecorder::new(16)));
        let mut battle = Battle::new(config());
        battle
            .add_observer_with_filter(recorder.clone(), EventFilter::only(CoreEventKind::Execute));
        battle.add_warrior(Warrior::new(
            "imp-1",
            vec![Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                1,
            )],
        ));
        battle.add_warrior(Warrior::new(
            "imp-2",
            vec![Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                1,
            )],
        ));

        battle.run();

        let recorder = recorder.borrow();
        assert!(!recorder.events().is_empty());
        assert!(recorder
            .events()
            .iter()
            .all(|event| matches!(event.event, CoreEvent::Execute { .. })));
    }
}
