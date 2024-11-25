use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{battlebit::BattlebitServer, results, server_info},
};
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;

async fn gather_servers() -> Vec<crate::structs::battlebit::BattlebitServer> {
    let client = reqwest::Client::new();
    let url = "https://publicapi.battlebit.cloud/Servers/GetServerList";
    match client.get(url).send().await {
        Ok(resp) => {
            let mut json_string = resp.text().await.unwrap_or_default();
            // remove weird 0 width character
            // https://github.com/seanmonstar/reqwest/issues/426
            let json_bytes = json_string.as_bytes();
            if json_bytes[0] == 239 {
                json_string.remove(0);
            }
            match serde_json::from_str::<Vec<crate::structs::battlebit::BattlebitServer>>(
                &json_string,
            ) {
                Ok(json_res) => {
                    return json_res;
                }
                Err(e) => {
                    log::error!("BattleBit public json is incorrect: {:#?}", e);
                    return vec![];
                }
            }
        }
        Err(e) => {
            log::error!("Battlebit public url failed: {:#?}", e);
            return vec![];
        }
    }
}

async fn server_list_to_sum(
    found_servers: Vec<BattlebitServer>,
) -> (
    HashMap<String, results::RegionResult>,
    HashMap<String, Vec<server_info::ServerInfo>>,
) {
    let modes = HashMap::from([
        ("CONQ", "Conquest"),
        ("INFCONQ", "Infantry Conquest"),
        ("FRONTLINE", "Frontlines"),
        ("RUSH", "Rush"),
        ("DOMI", "Domination"),
        ("TDM", "Teamdeathmatch"),
        ("GunGameFFA", "Gungame free-for-all"),
        ("FFA", "Free-for-all"),
        ("ELI", "Elimination"),
        ("GunGameTeam", "Gungame team"),
    ]);
    let server_regions = HashMap::from([
        ("Europe_Central", "Europe"),
        ("America_Central", "America"),
        ("Japan_Central", "Japan"),
        ("Australia_Central", "Australia"),
        ("Brazil_Central", "Brazil"),
    ]);

    let mut all_regions: results::RegionResult = results::RegionResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
        amounts: results::RegionAmounts {
            server_amount: 0,
            soldier_amount: 0,
            queue_amount: 0,
            community_server_amount: 0,
            community_soldier_amount: 0,
            community_queue_amount: 0,
            // unused
            spectator_amount: 0,
            dice_server_amount: 0,
            dice_soldier_amount: 0,
            dice_queue_amount: 0,
            dice_spectator_amount: 0,
            community_spectator_amount: 0,
        },
        maps: HashMap::new(),
        modes: HashMap::new(),
        timestamp: Utc::now(),
        map_players: HashMap::new(),
        mode_players: HashMap::new(),
        settings: HashMap::new(),
        settings_players: HashMap::new(),
        owner_platform: HashMap::new(),
        playground: HashMap::new(),
        playground_players: HashMap::new(),
    };
    let mut regions: HashMap<String, results::RegionResult> = HashMap::new();
    let mut server_stats: HashMap<String, Vec<server_info::ServerInfo>> = HashMap::new();

    for server in found_servers {
        let mode = modes
            .get(&server.gamemode[..])
            .unwrap_or(&&server.gamemode[..])
            .to_string();
        let server_region = server_regions
            .get(&server.region[..])
            .unwrap_or(&&server.region[..])
            .to_string();

        regions
            .entry(server_region.to_string())
            .and_modify(|region| {
                region.amounts.server_amount += 1;
                region.amounts.soldier_amount += server.players;
                region.amounts.queue_amount += server.queue_players;
                region
                    .maps
                    .entry(server.map.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                region
                    .map_players
                    .entry(server.map.clone())
                    .and_modify(|count| *count += server.players)
                    .or_insert(server.players);
                region
                    .modes
                    .entry(mode.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                region
                    .mode_players
                    .entry(mode.clone())
                    .and_modify(|count| *count += server.players)
                    .or_insert(server.players);
                if server.is_official {
                    region.amounts.dice_server_amount += 1;
                    region.amounts.dice_soldier_amount += server.players;
                    region.amounts.dice_queue_amount += server.queue_players;
                } else {
                    region.amounts.community_server_amount += 1;
                    region.amounts.community_soldier_amount += server.players;
                    region.amounts.community_queue_amount += server.queue_players;
                }
            })
            .or_insert({
                results::RegionResult {
                    metadata: results::Metadata {
                        region: server_region.to_string(),
                        platform: "pc".to_string(),
                    },
                    amounts: results::RegionAmounts {
                        server_amount: 1,
                        soldier_amount: server.players,
                        queue_amount: server.queue_players,
                        dice_server_amount: match server.is_official {
                            true => 1,
                            false => 0,
                        },
                        dice_soldier_amount: match server.is_official {
                            true => server.players,
                            false => 0,
                        },
                        dice_queue_amount: match server.is_official {
                            true => server.queue_players,
                            false => 0,
                        },
                        community_server_amount: match !server.is_official {
                            true => 1,
                            false => 0,
                        },
                        community_soldier_amount: match !server.is_official {
                            true => server.players,
                            false => 0,
                        },
                        community_queue_amount: match !server.is_official {
                            true => server.queue_players,
                            false => 0,
                        },
                        spectator_amount: 0,
                        dice_spectator_amount: 0,
                        community_spectator_amount: 0,
                    },
                    maps: HashMap::from([(server.map.clone(), 1)]),
                    modes: HashMap::from([(mode.clone(), 1)]),
                    timestamp: Utc::now(),
                    map_players: HashMap::from([(server.map.clone(), server.players)]),
                    mode_players: HashMap::from([(mode.clone(), server.players)]),
                    settings: HashMap::new(),
                    settings_players: HashMap::new(),
                    owner_platform: HashMap::new(),
                    playground: HashMap::new(),
                    playground_players: HashMap::new(),
                }
            });

        all_regions
            .maps
            .entry(server.map.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        all_regions
            .map_players
            .entry(server.map.clone())
            .and_modify(|count| *count += server.players)
            .or_insert(server.players);
        all_regions
            .modes
            .entry(mode.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        all_regions
            .mode_players
            .entry(mode.clone())
            .and_modify(|count| *count += server.players)
            .or_insert(server.players);

        all_regions.amounts.server_amount += 1;
        all_regions.amounts.soldier_amount += server.players;
        all_regions.amounts.queue_amount += server.queue_players;
        if server.is_official {
            all_regions.amounts.dice_server_amount += 1;
            all_regions.amounts.dice_soldier_amount += server.players;
            all_regions.amounts.dice_queue_amount += server.queue_players;
        } else {
            all_regions.amounts.community_server_amount += 1;
            all_regions.amounts.community_soldier_amount += server.players;
            all_regions.amounts.community_queue_amount += server.queue_players;
        }

        let current_server_info = server_info::ServerInfo {
            guid: "".to_owned(),
            name: server.name,
            soldiers: server.players,
            queue: 0,
            mode,
            map: server.map,
            game_id: "".to_owned(),
            is_official: Some(server.is_official),
        };
        server_stats
            .entry(server.region.to_string())
            .and_modify(|region_info| region_info.push(current_server_info.clone()))
            .or_insert_with(|| vec![current_server_info]);
    }

    regions.insert("ALL".to_string(), all_regions);
    (regions, server_stats)
}

pub async fn push_battlebit(
    pool: &PgPool,
    influx_client: &influxdb2::Client,
) -> anyhow::Result<()> {
    let found_servers = gather_servers().await;
    let (regions, server_stats) = server_list_to_sum(found_servers).await;
    for (region, server_stat) in server_stats {
        match push_server(pool, "battlebit", &region, "pc", server_stat).await {
            Ok(_) => {}
            Err(e) => log::error!(
                "battlebit region {} failed to push specific serverinfo: {:#?}",
                region,
                e
            ),
        };
    }
    match influx_db::push_to_database(influx_client, "battlebit", "pc", &regions).await {
        Ok(_) => {}
        Err(e) => log::error!("battlebit failed to push to influxdb: {:#?}", e),
    };
    Ok(())
}
