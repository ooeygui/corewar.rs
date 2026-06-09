use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, bail};
use corewar_leaderboard::Leaderboard;
use corewar_protocol::{CellInfo, InstanceInfo, InstanceStatus, ServerMessage};
use tokio::sync::{mpsc, RwLock};

pub type ClientId = u64;
pub type ClientSender = mpsc::UnboundedSender<ServerMessage>;

pub struct AppState {
    next_client_id: AtomicU64,
    pub clients: Arc<RwLock<ClientRegistry>>,
    pub orchestrator: Arc<RwLock<PlaceholderOrchestrator>>,
    pub leaderboard: Arc<RwLock<Leaderboard>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            next_client_id: AtomicU64::new(1),
            clients: Arc::new(RwLock::new(ClientRegistry::default())),
            orchestrator: Arc::new(RwLock::new(PlaceholderOrchestrator::default())),
            leaderboard: Arc::new(RwLock::new(Leaderboard::default())),
        }
    }

    pub async fn add_client(&self, sender: ClientSender) -> ClientId {
        let client_id = self.next_client_id.fetch_add(1, Ordering::Relaxed);
        self.clients.write().await.add_client(client_id, sender);
        client_id
    }

    pub async fn remove_client(&self, client_id: ClientId) {
        self.clients.write().await.remove_client(client_id);
    }

    pub async fn subscribe_client(
        &self,
        client_id: ClientId,
        instance_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        self.clients
            .write()
            .await
            .subscribe_client(client_id, instance_id.into())
    }

    pub async fn unsubscribe_client(&self, client_id: ClientId, instance_id: &str) {
        self.clients
            .write()
            .await
            .unsubscribe_client(client_id, instance_id);
    }

    pub async fn broadcast_to_instance(&self, instance_id: &str, message: ServerMessage) {
        let subscribers = self.clients.read().await.subscribers_for(instance_id);
        let mut stale_clients = Vec::new();

        for (client_id, sender) in subscribers {
            if sender.send(message.clone()).is_err() {
                stale_clients.push(client_id);
            }
        }

        for client_id in stale_clients {
            self.remove_client(client_id).await;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct ClientRegistry {
    clients: HashMap<ClientId, ClientEntry>,
    instance_subscribers: HashMap<String, HashSet<ClientId>>,
}

impl ClientRegistry {
    pub fn add_client(&mut self, client_id: ClientId, sender: ClientSender) {
        self.clients.insert(
            client_id,
            ClientEntry {
                sender,
                subscriptions: HashSet::new(),
            },
        );
    }

    pub fn remove_client(&mut self, client_id: ClientId) {
        if let Some(client) = self.clients.remove(&client_id) {
            let mut empty_instances = Vec::new();

            for instance_id in client.subscriptions {
                if let Some(subscribers) = self.instance_subscribers.get_mut(&instance_id) {
                    subscribers.remove(&client_id);
                    if subscribers.is_empty() {
                        empty_instances.push(instance_id);
                    }
                }
            }

            for instance_id in empty_instances {
                self.instance_subscribers.remove(&instance_id);
            }
        }
    }

    pub fn subscribe_client(
        &mut self,
        client_id: ClientId,
        instance_id: String,
    ) -> anyhow::Result<()> {
        let client = self
            .clients
            .get_mut(&client_id)
            .ok_or_else(|| anyhow!("unknown client: {client_id}"))?;

        client.subscriptions.insert(instance_id.clone());
        self.instance_subscribers
            .entry(instance_id)
            .or_default()
            .insert(client_id);
        Ok(())
    }

    pub fn unsubscribe_client(&mut self, client_id: ClientId, instance_id: &str) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.subscriptions.remove(instance_id);
        }

        let remove_instance_entry =
            if let Some(subscribers) = self.instance_subscribers.get_mut(instance_id) {
                subscribers.remove(&client_id);
                subscribers.is_empty()
            } else {
                false
            };

        if remove_instance_entry {
            self.instance_subscribers.remove(instance_id);
        }
    }

    pub fn subscribers_for(&self, instance_id: &str) -> Vec<(ClientId, ClientSender)> {
        self.instance_subscribers
            .get(instance_id)
            .into_iter()
            .flat_map(|subscribers| subscribers.iter().copied())
            .filter_map(|client_id| {
                self.clients
                    .get(&client_id)
                    .map(|client| (client_id, client.sender.clone()))
            })
            .collect()
    }
}

struct ClientEntry {
    sender: ClientSender,
    subscriptions: HashSet<String>,
}

pub struct PlaceholderOrchestrator {
    instances: HashMap<String, InstanceInfo>,
    next_instance_id: usize,
    next_warrior_id: usize,
}

impl PlaceholderOrchestrator {
    pub fn list_instances(&self) -> Vec<InstanceInfo> {
        let mut instances: Vec<_> = self.instances.values().cloned().collect();
        instances.sort_by(|left, right| left.id.cmp(&right.id));
        instances
    }

    pub fn snapshot_for(&self, instance_id: &str) -> Option<ServerMessage> {
        self.instances.get(instance_id).map(|instance| {
            let cells = (0..16)
                .map(|address| CellInfo {
                    address,
                    owner: Some((address % instance.warrior_names.len().max(1)) as u32),
                    instruction_summary: format!("DAT #{}", address),
                })
                .collect();

            ServerMessage::CoreSnapshot {
                instance_id: instance.id.clone(),
                cells,
            }
        })
    }

    pub fn load_warrior(&mut self, source: &str) -> anyhow::Result<String> {
        if source.trim().is_empty() {
            bail!("warrior source cannot be empty");
        }

        let warrior_id = format!("warrior-{}", self.next_warrior_id);
        self.next_warrior_id += 1;
        Ok(warrior_id)
    }

    pub fn start_tournament(&mut self, warrior_ids: Vec<String>) -> anyhow::Result<InstanceInfo> {
        if warrior_ids.is_empty() {
            bail!("at least one warrior id is required to start a tournament");
        }

        let instance = InstanceInfo {
            id: format!("arena-{}", self.next_instance_id),
            warrior_names: warrior_ids,
            core_size: 8_000,
            cycle: 0,
            status: InstanceStatus::Running,
        };
        self.next_instance_id += 1;
        self.instances.insert(instance.id.clone(), instance.clone());
        Ok(instance)
    }

    pub fn toggle_pause(&mut self, instance_id: &str) -> anyhow::Result<InstanceInfo> {
        let instance = self
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| anyhow!("unknown instance: {instance_id}"))?;

        instance.status = match instance.status {
            InstanceStatus::Running => InstanceStatus::Paused,
            InstanceStatus::Paused => InstanceStatus::Running,
            InstanceStatus::Complete => bail!("instance {instance_id} is already complete"),
        };
        instance.cycle += 1;

        Ok(instance.clone())
    }
}

impl Default for PlaceholderOrchestrator {
    fn default() -> Self {
        let mut instances = HashMap::new();
        let seed_instance = InstanceInfo {
            id: "arena-1".to_string(),
            warrior_names: vec!["Imp".to_string(), "Dwarf".to_string()],
            core_size: 8_000,
            cycle: 0,
            status: InstanceStatus::Running,
        };
        instances.insert(seed_instance.id.clone(), seed_instance);

        Self {
            instances,
            next_instance_id: 2,
            next_warrior_id: 1,
        }
    }
}
