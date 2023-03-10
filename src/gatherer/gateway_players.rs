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
            "game_ids": Vec::from_iter(managed_results.unmanaged_servers.keys()),
        }))
        .send()
        .await?;

    let mut players: HashMap<String, String> = HashMap::new();

    let servers = result
        .json::<Vec<HashMap<i64, crate::structs::game_players::GamePlayers>>>()
        .await?;

    for server in servers {
        for (game_id, result) in server {
            for player in result.players.keys() {
                // add guid for bf2042
                if game_name == "kingston" {
                    players.insert(
                        player.to_string(),
                        managed_results
                            .unmanaged_servers
                            .get(&game_id)
                            .unwrap_or(&"game_id".to_string())
                            .to_string(),
                    );
                } else {
                    players.insert(player.to_string(), game_id.to_string());
                }
            }
        }
    }

    Ok(players)
}
