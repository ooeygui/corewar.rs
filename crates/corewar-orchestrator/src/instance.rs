use std::{
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use corewar_core::{CoreEvent, Instruction, TimedEvent};
use corewar_protocol::{BattleResultMsg, CellInfo, CycleEvent, ServerMessage};
use corewar_vm::{battle::RoundResult, BattleObserver, BattleSetup};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleInstanceStatus {
    Pending,
    Running,
    Complete,
}

#[derive(Debug)]
pub struct BattleInstance {
    id: String,
    warrior_ids: Vec<String>,
    status: RwLock<BattleInstanceStatus>,
    event_tx: broadcast::Sender<ServerMessage>,
}

impl BattleInstance {
    pub fn new(id: String, warrior_ids: Vec<String>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            id,
            warrior_ids,
            status: RwLock::new(BattleInstanceStatus::Pending),
            event_tx,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn warrior_ids(&self) -> &[String] {
        &self.warrior_ids
    }

    pub fn status(&self) -> BattleInstanceStatus {
        *self.status.read().expect("instance status lock poisoned")
    }

    pub fn set_status(&self, status: BattleInstanceStatus) {
        *self.status.write().expect("instance status lock poisoned") = status;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.event_tx.subscribe()
    }

    pub fn emit(&self, message: ServerMessage) {
        let _ = self.event_tx.send(message);
    }

    pub fn emit_complete(&self, result: BattleResultMsg) {
        self.emit(ServerMessage::BattleComplete {
            instance_id: self.id.clone(),
            result,
        });
    }
}

#[derive(Debug, Default)]
pub struct InstanceManager {
    next_id: AtomicUsize,
    instances: Mutex<HashMap<String, Arc<BattleInstance>>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_instance(&self, warrior_ids: Vec<String>) -> Arc<BattleInstance> {
        let id = format!(
            "battle-{}",
            self.next_id.fetch_add(1, Ordering::Relaxed) + 1
        );
        let instance = Arc::new(BattleInstance::new(id.clone(), warrior_ids));
        self.instances
            .lock()
            .expect("instance manager lock poisoned")
            .insert(id, instance.clone());
        instance
    }

    pub fn get(&self, instance_id: &str) -> Option<Arc<BattleInstance>> {
        self.instances
            .lock()
            .expect("instance manager lock poisoned")
            .get(instance_id)
            .cloned()
    }

    pub fn list(&self) -> Vec<Arc<BattleInstance>> {
        let mut instances: Vec<_> = self
            .instances
            .lock()
            .expect("instance manager lock poisoned")
            .values()
            .cloned()
            .collect();
        instances.sort_by(|left, right| left.id().cmp(right.id()));
        instances
    }
}

pub struct InstanceEventObserver {
    instance: Arc<BattleInstance>,
}

impl InstanceEventObserver {
    pub fn new(instance: Arc<BattleInstance>) -> Self {
        Self { instance }
    }
}

impl fmt::Debug for InstanceEventObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstanceEventObserver")
            .field("instance_id", &self.instance.id())
            .finish()
    }
}

impl BattleObserver for InstanceEventObserver {
    fn on_battle_loaded(&mut self, setup: &BattleSetup) {
        self.instance.emit(ServerMessage::CoreSnapshot {
            instance_id: self.instance.id().to_string(),
            cells: snapshot_from_setup(setup),
        });
    }

    fn on_cycle(&mut self, cycle: u64, events: &[TimedEvent]) {
        let cycle_events: Vec<_> = events.iter().filter_map(map_cycle_event).collect();
        if cycle_events.is_empty() {
            return;
        }

        self.instance.emit(ServerMessage::CycleEvents {
            instance_id: self.instance.id().to_string(),
            cycle,
            events: cycle_events,
        });
    }

    fn on_round_complete(&mut self, _result: &RoundResult) {}
}

fn snapshot_from_setup(setup: &BattleSetup) -> Vec<CellInfo> {
    let mut cells = vec![
        CellInfo {
            address: 0,
            owner: None,
            instruction_summary: Instruction::default().to_string(),
        };
        setup.core_size
    ];

    for (address, cell) in cells.iter_mut().enumerate() {
        cell.address = address;
    }

    for warrior in &setup.warriors {
        for (offset, instruction) in warrior.instructions.iter().enumerate() {
            let address = (warrior.start + offset) % setup.core_size;
            cells[address] = CellInfo {
                address,
                owner: Some(warrior.warrior_id),
                instruction_summary: instruction.to_string(),
            };
        }
    }

    cells
}

fn map_cycle_event(event: &TimedEvent) -> Option<CycleEvent> {
    match event.event {
        CoreEvent::Read {
            address,
            warrior_id,
        } => Some(CycleEvent::Read {
            address,
            warrior_id,
        }),
        CoreEvent::MemoryWrite {
            address,
            warrior_id,
            ..
        } => Some(CycleEvent::Write {
            address,
            warrior_id,
        }),
        CoreEvent::Execute {
            address,
            warrior_id,
        } => Some(CycleEvent::Execute {
            address,
            warrior_id,
        }),
        CoreEvent::ProcessCreated {
            warrior_id,
            address,
        } => Some(CycleEvent::ProcessCreated {
            warrior_id,
            address,
        }),
        CoreEvent::ProcessKilled {
            warrior_id,
            address,
        } => Some(CycleEvent::ProcessKilled {
            warrior_id,
            address,
        }),
        CoreEvent::WarriorEliminated { .. } | CoreEvent::CycleComplete => None,
    }
}
