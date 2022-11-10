use std::collections::HashMap;
use futures::future::join_all;
use reqwest::header::HeaderMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{mongo::MongoClient, structs::results};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BattlelogServer {
    guid: String,
    ip: String,
    region: String,
    #[serde(rename = "queueAmount")]
    queue_amount: i64,
    #[serde(rename = "soldierAmount")]
    soldier_amount: i64,
}

async fn get_battlelog_keeper_data(guid: &String) -> anyhow::Result<(&String, usize)> {
    let client = reqwest::Client::new();
    let url = format!("https://keeper.battlelog.com/snapshot/{guid}");
    let mut players = 0;
    match client.get(url).send().await {
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Ok(json_res) => {
                    for item in json_res["snapshot"]["teamInfo"].as_array().unwrap_or(&vec![]) {
                        players += item["players"].as_array().unwrap_or(&vec![]).len();
                    }
                },
                Err(e) => {
                    log::error!("bf4 failed to read json of snapshot of guid {}: {:#?}", guid, e);
                },
            }
        },
        Err(e) => {
            log::error!("bf4 failed to get snapshot of guid {}: {:#?}", guid, e);
        },
    }

    Ok((guid, players))
}

async fn get_all_regions(game_name: &str, base_uri: &str) -> anyhow::Result<HashMap<String, BattlelogServer>> {
    let battlelog_regions = HashMap::from([
        (1, "NAm"),
        (2, "SAm"),
        (4, "AU"),
        (8, "Africa"),
        (16, "EU"),
        (32, "Asia"),
        (64, "OC"),
    ]);
    
    let mut offset = 0;
    let per_page = 60;
    let mut pages_since_last_unique_server = 0;
    let mut attempt = 0;
    let max_attempts = 3;
    let page_limit = 10;
    let client = reqwest::Client::new();
    let mut server_total_before: usize = 0;

    let mut found_servers: HashMap<String, BattlelogServer> = HashMap::new();
    
    while pages_since_last_unique_server < page_limit && attempt < max_attempts {
        let mut headers = HeaderMap::new();
        headers.insert("X-Requested-With", "XMLHttpRequest".parse()?);
        let url = format!("{}?count={}&offset=0", base_uri, per_page);
        match client.get(url).headers(headers).send().await {
            Ok(resp) => {
                match resp.json::<serde_json::Value>().await {
                    Ok(json_res) => {
                        attempt = 0;
                        server_total_before = found_servers.len();

                        for server in json_res["data"].as_array().unwrap_or(&vec![]) {
                            let found_server = BattlelogServer {
                                guid: server["guid"].as_str().unwrap_or_default().to_string(),
                                ip: server["ip"].as_str().unwrap_or_default().to_string(),
                                region: battlelog_regions.get(&server["region"].as_i64().unwrap_or_default()).unwrap_or(&"").to_string(),
                                queue_amount: server["slots"]["1"]["current"].as_i64().unwrap_or_default(),
                                soldier_amount: server["slots"]["2"]["current"].as_i64().unwrap_or_default(),
                            };
                            // against duplicates
                            if found_server.ip.len() > 0 && !found_servers.contains_key(&found_server.guid) {
                                found_servers.insert(found_server.clone().guid, found_server);
                            }
                        }

                        if found_servers.len() == server_total_before {
                            pages_since_last_unique_server += 1;
                        } else {
                            // Found new unique server, reset
                            pages_since_last_unique_server = 0;
                        }
                        offset += per_page;
                    },
                    Err(_) => {
                        attempt += 1;
                    }
                };
            },
            Err(_) => {
                attempt += 1;
            },
        };
    }

    if game_name == "bf4" {
        let found_server_copy = found_servers.clone();
        let populated_servers = found_server_copy.values().filter(|&x| x.soldier_amount > 0);
        let mut tasks = vec![];
        for server in populated_servers {
            tasks.push(get_battlelog_keeper_data(&server.guid));
        }
        let result = join_all(tasks).await;
        for server in result {
            match server {
                Ok((server, result)) => {
                    match found_servers.get_mut(server) {
                        Some(found_server) => {
                            found_server.soldier_amount = result as i64;
                        },
                        None => {},
                    }
                },
                Err(_) => {},
            }
        }
    }

    Ok(found_servers)
}

async fn server_list_to_sum(found_servers: HashMap<String, BattlelogServer>) -> anyhow::Result<HashMap<String, results::RegionResult>> {
    let mut all_regions: results::RegionResult = results::RegionResult { 
        metadata: results::Metadata { region: "ALL".to_string(), platform: "pc".to_string() },
        amounts: results::RegionAmounts {
            server_amount: 0,
            soldier_amount: 0,
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
    };
    
    let mut regions: HashMap<String, results::RegionResult> = HashMap::new();
    for server in found_servers.values() {
        regions.entry(server.region.to_string()).and_modify(|region| {
            region.amounts.server_amount += 1;
            region.amounts.soldier_amount += server.soldier_amount;
            region.amounts.queue_amount += server.queue_amount;
        }).or_insert({
            results::RegionResult { 
                metadata: results::Metadata { region: server.region.to_string(), platform: "pc".to_string() },
                amounts: results::RegionAmounts {
                    server_amount: 1,
                    soldier_amount: server.soldier_amount,
                    queue_amount: server.queue_amount,
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
            }
        });

        all_regions.amounts.server_amount += 1;
        all_regions.amounts.soldier_amount += server.soldier_amount;
        all_regions.amounts.queue_amount += server.queue_amount;
    }
    regions.insert("ALL".to_string(), all_regions);

    Ok(regions)
}

async fn get_region_stats(game_name: &str, base_uri: &str) -> anyhow::Result<HashMap<String, results::RegionResult>> {
    let found_servers = get_all_regions(game_name, base_uri).await?;
    let result = server_list_to_sum(found_servers).await?;

    Ok(result)
}

pub async fn gather_battlelog(mongo_client: &mut MongoClient, game_name: &str, base_uri: &str) -> anyhow::Result<results::RegionResult> {
    let game_result = match get_region_stats(game_name, base_uri).await {
        Ok(result) => {
            match mongo_client.push_to_database(game_name, &result).await {
                Ok(_) => {},
                Err(e) => log::error!("{} failed to push to influxdb: {:#?}", game_name, e),
            };
            result
        },
        Err(e) => anyhow::bail!("{} gather failed: {:#?}", game_name, e),
    };
    let result = match game_result.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("{} has no ALL region!", game_name),
    };

    Ok(result.to_owned())
}