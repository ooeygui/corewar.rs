use std::sync::Arc;

use corewar_protocol::{ClientMessage, LeaderboardEntry, ServerMessage};

use crate::state::{AppState, ClientId};

pub async fn dispatch_client_message(
    state: &Arc<AppState>,
    client_id: ClientId,
    message: ClientMessage,
) -> anyhow::Result<Vec<ServerMessage>> {
    match message {
        ClientMessage::ListInstances => handle_list_instances(state).await,
        ClientMessage::Subscribe { instance_id } => {
            handle_subscribe(state, client_id, instance_id).await
        }
        ClientMessage::Unsubscribe { instance_id } => {
            handle_unsubscribe(state, client_id, instance_id).await
        }
        ClientMessage::LeaderboardQuery { top_n } => handle_leaderboard_query(state, top_n).await,
        ClientMessage::LoadWarrior { source } => handle_load_warrior(state, source).await,
        ClientMessage::StartTournament { warrior_ids } => {
            handle_start_tournament(state, warrior_ids).await
        }
        ClientMessage::TogglePause { instance_id } => handle_toggle_pause(state, instance_id).await,
    }
}

pub async fn handle_list_instances(state: &Arc<AppState>) -> anyhow::Result<Vec<ServerMessage>> {
    let instances = state.orchestrator.read().await.list_instances();
    Ok(vec![ServerMessage::InstanceList { instances }])
}

pub async fn handle_subscribe(
    state: &Arc<AppState>,
    client_id: ClientId,
    instance_id: String,
) -> anyhow::Result<Vec<ServerMessage>> {
    let snapshot = {
        let orchestrator = state.orchestrator.read().await;
        orchestrator
            .snapshot_for(&instance_id)
            .ok_or_else(|| anyhow::anyhow!("unknown instance: {instance_id}"))?
    };

    state
        .subscribe_client(client_id, instance_id.clone())
        .await?;
    state.broadcast_to_instance(&instance_id, snapshot).await;

    Ok(Vec::new())
}

pub async fn handle_unsubscribe(
    state: &Arc<AppState>,
    client_id: ClientId,
    instance_id: String,
) -> anyhow::Result<Vec<ServerMessage>> {
    state.unsubscribe_client(client_id, &instance_id).await;
    Ok(Vec::new())
}

pub async fn handle_leaderboard_query(
    state: &Arc<AppState>,
    top_n: usize,
) -> anyhow::Result<Vec<ServerMessage>> {
    let leaderboard = state.leaderboard.read().await;
    let entries = leaderboard
        .top_n(top_n)
        .into_iter()
        .map(|rating| LeaderboardEntry {
            warrior_name: rating.name.clone(),
            rating: rating.rating,
            wins: rating.wins,
            losses: rating.losses,
            draws: rating.draws,
        })
        .collect();

    Ok(vec![ServerMessage::LeaderboardUpdate { entries }])
}

pub async fn handle_load_warrior(
    state: &Arc<AppState>,
    source: String,
) -> anyhow::Result<Vec<ServerMessage>> {
    let warrior_id = state.orchestrator.write().await.load_warrior(&source)?;
    let entries = {
        let mut leaderboard = state.leaderboard.write().await;
        leaderboard.get_or_create(&warrior_id);
        let total_entries = leaderboard.all_ratings().len();
        leaderboard
            .top_n(total_entries)
            .into_iter()
            .map(|rating| LeaderboardEntry {
                warrior_name: rating.name.clone(),
                rating: rating.rating,
                wins: rating.wins,
                losses: rating.losses,
                draws: rating.draws,
            })
            .collect()
    };

    Ok(vec![ServerMessage::LeaderboardUpdate { entries }])
}

pub async fn handle_start_tournament(
    state: &Arc<AppState>,
    warrior_ids: Vec<String>,
) -> anyhow::Result<Vec<ServerMessage>> {
    let instance = state
        .orchestrator
        .write()
        .await
        .start_tournament(warrior_ids)?;
    let snapshot = {
        let orchestrator = state.orchestrator.read().await;
        orchestrator.snapshot_for(&instance.id)
    };

    if let Some(snapshot) = snapshot {
        state.broadcast_to_instance(&instance.id, snapshot).await;
    }

    handle_list_instances(state).await
}

pub async fn handle_toggle_pause(
    state: &Arc<AppState>,
    instance_id: String,
) -> anyhow::Result<Vec<ServerMessage>> {
    let _instance = state
        .orchestrator
        .write()
        .await
        .toggle_pause(&instance_id)?;
    let snapshot = {
        let orchestrator = state.orchestrator.read().await;
        orchestrator.snapshot_for(&instance_id)
    };

    if let Some(snapshot) = snapshot {
        state.broadcast_to_instance(&instance_id, snapshot).await;
    }

    handle_list_instances(state).await
}
