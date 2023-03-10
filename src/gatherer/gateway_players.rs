use crate::structs::results;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::collections::HashMap;

pub async fn gather_players(
    game_name: &str,
    managed_results: results::ManagedInfo,
) -> anyhow::Result<HashMap<String, String>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert(
        "authentication",
        ((chrono::Utc::now().timestamp() / 60) * 306596067).to_string()[..]
            .parse()
            .unwrap(),
    );
    let result = client
        .post(format!(
            "https://gateway.gametools.network/{}/players",
            game_name
        ))
        .headers(headers)
        .json(&json!({
            "game_ids": managed_results.unmanaged_servers,
        }))
        .send()
        .await?;

    let mut players: HashMap<String, String> = HashMap::new();

    let servers = result
        .json::<Vec<HashMap<i64, crate::structs::game_players::GamePlayers>>>()
        .await?;

    for server in servers {
        for result in server.values() {
            for player in result.players.keys() {
                players.insert(player.to_string(), result.server_info.server_name.clone());
            }
        }
    }

    Ok(players)
}
