use crate::{connectors::mongo::MongoClient, structs::results};
use chrono::Utc;
use std::collections::HashMap;

pub async fn push_old_games(
    mongo_client: &mut MongoClient,
    mongo_game_name: &str,
    frontend_game_name: &str,
) -> anyhow::Result<results::RegionResult> {
    let servers = match mongo_client.gather_old_title(mongo_game_name).await? {
        Some(servers) => servers,
        None => anyhow::bail!("No serverinfo gotten {}", frontend_game_name),
    };

    let mut soldier_amount: i64 = 0;
    for server in &servers.server_list {
        soldier_amount += server.numplayers.parse::<i64>().unwrap_or_default();
    }

    let game_result = results::OldGameResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
        server_amount: servers.server_list.len() as i64,
        soldier_amount,
        timestamp: Utc::now(),
    };
    mongo_client
        .push_old_games(frontend_game_name, game_result)
        .await?;
    Ok(results::RegionResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
        amounts: results::RegionAmounts {
            server_amount: servers.server_list.len() as i64,
            soldier_amount: soldier_amount as i64,
            queue_amount: 0,
            spectator_amount: 0,
            dice_server_amount: 0,
            dice_soldier_amount: 0,
            dice_queue_amount: 0,
            dice_spectator_amount: 0,
            community_server_amount: 0,
            community_soldier_amount: 0,
            community_queue_amount: 0,
            community_spectator_amount: 0,
        },
        maps: HashMap::new(),
        modes: HashMap::new(),
        settings: HashMap::new(),
        owner_platform: HashMap::new(),
        timestamp: Utc::now(),
    })
}
