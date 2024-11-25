use chrono::Utc;
use futures::future::join_all;
use reqwest::header::HeaderMap;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{
        battlelog::BattlelogServer,
        results,
        server_info::{self, ServerInfo},
    },
};

async fn get_battlelog_keeper_data(guid: &String) -> anyhow::Result<(&String, usize)> {
    let client = reqwest::Client::new();
    let url = format!("https://keeper.battlelog.com/snapshot/{guid}");
    let mut players = 0;
    match client.get(url).send().await {
        Ok(resp) => match resp.json::<crate::structs::battlelog::Keeper>().await {
            Ok(json_res) => {
                for item in json_res.snapshot.team_info.values() {
                    players += item.players.len();
                }
            }
            Err(e) => {
                log::error!(
                    "bf4 failed to read json of snapshot of guid {}: {:#?}",
                    guid,
                    e
                );
            }
        },
        Err(e) => {
            log::error!("bf4 failed to get snapshot of guid {}: {:#?}", guid, e);
        }
    }

    Ok((guid, players))
}

async fn get_all_regions(
    game_name: &str,
    base_uri: &str,
) -> anyhow::Result<HashMap<String, BattlelogServer>> {
    let battlelog_regions = HashMap::from([
        (1, "NAm"),
        (2, "SAm"),
        (4, "AU"),
        (8, "Africa"),
        (16, "EU"),
        (32, "Asia"),
        (64, "OC"),
    ]);

    let bf3_maps = HashMap::from([
        ("MP_001", "Grand Bazaar "),
        ("MP_003", "Tehran Highway"),
        ("MP_007", "Caspian Border"),
        ("MP_011", "Seine Crossing "),
        ("MP_012", "Operation Firestorm"),
        ("MP_013", "Damavand Peak "),
        ("MP_017", "Noshahr Canals"),
        ("MP_018", "Kharg Island"),
        ("MP_Subway", "Operation Métro"),
        ("XP1_001", "Strike at Karkand"),
        ("XP1_002", "Gulf of Oman"),
        ("XP1_003", "Sharqi Peninsula"),
        ("XP1_004", "Wake Island"),
        ("XP2_Factory", "Scrapmetal"),
        ("XP2_Office", "Operation 925"),
        ("XP2_Palace", "Donya Fortress"),
        ("XP2_Skybar", "Ziba Tower"),
        ("XP3_Alborz", "Alborz Mountains"),
        ("XP3_Desert", "Bandar Desert"),
        ("XP3_Shield", "Armored Shield"),
        ("XP3_Valley", "Death Valley"),
        ("XP4_FD", "Markaz Monolith"),
        ("XP4_Parl", "Azadi Palace"),
        ("XP4_Quake", "Epicenter"),
        ("XP4_Rubble", "Talah Market"),
        ("XP5_001", "Operation Riverside"),
        ("XP5_002", "Nebandan Flats"),
        ("XP5_003", "Kiasar Railroad"),
        ("XP5_004", "Sabalan Pipeline"),
    ]);
    let bf4_maps = HashMap::from([
        ("MP_Abandoned", "Zavod 311"),
        ("MP_Damage", "Lancang Dam"),
        ("MP_Flooded", "Flood Zone"),
        ("MP_Journey", "Golmud Railway"),
        ("MP_Naval", "Paracel Storm"),
        ("MP_Prison", "Operation Locker"),
        ("MP_Resort", "Hainan Resort"),
        ("MP_Siege", "Siege of Shanghai"),
        ("MP_TheDish", "Rogue Transmission"),
        ("MP_Tremors", "Dawnbreaker"),
        ("XP0_Caspian", "CASPIAN BORDER 2014"),
        ("XP0_Firestorm", "OPERATION FIRESTORM 2014"),
        ("XP0_Metro", "OPERATION METRO 2014"),
        ("XP0_Oman", "GULF OF OMAN 2014"),
        ("XP1_001", "SILK ROAD"),
        ("XP1_002", "ALTAI RANGE"),
        ("XP1_003", "GUILIN PEAKS"),
        ("XP1_004", "DRAGON PASS"),
        ("XP2_001", "LOST ISLANDS"),
        ("XP2_002", "NANSHA STRIKE"),
        ("XP2_003", "WAVE BREAKER"),
        ("XP2_004", "OPERATION MORTAR"),
        ("XP3_MarketPl", "PEARL MARKET"),
        ("XP3_Prpganda", "PROPAGANDA"),
        ("XP3_UrbanGdn", "LUMPHINI GARDEN"),
        ("XP3_WtrFront", "SUNKEN DRAGON"),
        ("XP4_Arctic", "OPERATION WHITEOUT"),
        ("XP4_SubBase", "HAMMERHEAD"),
        ("XP4_Titan", "HANGAR 21"),
        ("XP4_WlkrFtry", "GIANTS OF KARELIA"),
        ("XP5_Night_01", "ZAVOD, GRAVEYARD SHIFT"),
        ("XP6_CMP", "OPERATION OUTBREAK"),
        ("XP7_Valley", "DRAGON VALLEY 2015"),
    ]);
    let bfh_maps = HashMap::from([
        ("mp_bank", "Bank job"),
        ("mp_bloodout", "The block"),
        ("mp_desert", "Dust bowl"),
        ("mp_downtown", "Downtown"),
        ("mp_eastside", "Derailed"),
        ("mp_everglades", "Everglades"),
        ("mp_growhouse", "Growhouse"),
        ("mp_hills", "Hollywood heights"),
        ("mp_offshore", "Riptide"),
        ("omaha_sp_assault", "Ep. 10, legacy"),
        ("omaha_sp_chopshop", "Ep. 6, out of business"),
        ("omaha_sp_copfantasy", "Ep. 1, back to school"),
        ("omaha_sp_dealgonebad", "Ep. 2, checking out"),
        ("omaha_sp_desert", "Ep. 8, sovereign land"),
        ("omaha_sp_escape", "Ep. 5, gauntlet"),
        ("omaha_sp_everglades", "Ep. 3, gator bait"),
        ("omaha_sp_heist", "Ep. 9, independence day"),
        ("omaha_sp_hollywoodhills", "Ep. 7, glass houses"),
        ("omaha_sp_prologue", "Prologue"),
        ("omaha_sp_theturn", "Ep. 4, case closed"),
        ("xp1_mallcops", "Black friday"),
        ("xp1_nights", "Code blue"),
        ("xp1_projects", "The beat"),
        ("xp1_sawmill", "Backwoods"),
        ("xp25_bank", "Night job"),
        ("xp25_sawmill", "Night woods"),
        ("xp2_cargoship", "The docks"),
        ("xp2_coastal", "Break pointe"),
        ("xp2_nh_museum", "Museum"),
        ("xp2_precinct7", "Precinct 7"),
        ("xp3_border", "Double cross"),
        ("xp3_cistern", "Diversion"),
        ("xp3_highway", "Pacific highway"),
        ("xp3_traindodge", "Train dodge"),
        ("xp4_alcatraz", "Alcatraz"),
        ("xp4_cemetery", "Cemetery"),
        ("xp4_chinatown", "Chinatown"),
        ("xp4_snowcrash", "Thin ice"),
    ]);

    let mut _offset = 0;
    let per_page = 60;
    let mut pages_since_last_unique_server = 0;
    let mut attempt = 0;
    let max_attempts = 3;
    let page_limit = 10;
    let client = reqwest::Client::new();
    let mut _server_total_before: usize = 0;

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
                        _server_total_before = found_servers.len();

                        for server in json_res["data"].as_array().unwrap_or(&vec![]) {
                            let current_map = server["map"].as_str().unwrap_or_default();
                            let found_server = BattlelogServer {
                                game_id: server["gameId"].as_str().unwrap_or_default().to_string(),
                                name: server["name"].as_str().unwrap_or_default().to_string(),
                                guid: server["guid"].as_str().unwrap_or_default().to_string(),
                                ip: server["ip"].as_str().unwrap_or_default().to_string(),
                                region: battlelog_regions
                                    .get(&server["region"].as_i64().unwrap_or_default())
                                    .unwrap_or(&"")
                                    .to_string(),
                                queue_amount: server["slots"]["1"]["current"]
                                    .as_i64()
                                    .unwrap_or_default(),
                                soldier_amount: server["slots"]["2"]["current"]
                                    .as_i64()
                                    .unwrap_or_default(),
                                map: match game_name {
                                    "bfh" => bfh_maps
                                        .get(current_map)
                                        .unwrap_or(&current_map)
                                        .to_string(),
                                    "bf3" => bf3_maps
                                        .get(current_map)
                                        .unwrap_or(&current_map)
                                        .to_string(),
                                    _ => bf4_maps
                                        .get(current_map)
                                        .unwrap_or(&current_map)
                                        .to_string(),
                                },
                            };
                            // against duplicates
                            if !found_server.ip.is_empty()
                                && !found_servers.contains_key(&found_server.guid)
                            {
                                found_servers.insert(found_server.clone().guid, found_server);
                            }
                        }

                        if found_servers.len() == _server_total_before {
                            pages_since_last_unique_server += 1;
                        } else {
                            // Found new unique server, reset
                            pages_since_last_unique_server = 0;
                        }
                        _offset += per_page;
                    }
                    Err(_) => {
                        attempt += 1;
                    }
                };
            }
            Err(_) => {
                attempt += 1;
            }
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
        for (server, result) in result.into_iter().flatten() {
            if let Some(found_server) = found_servers.get_mut(server) {
                found_server.soldier_amount = result as i64;
            }
        }
    }

    Ok(found_servers)
}

async fn server_list_to_sum(
    pool: &PgPool,
    game_name: &str,
    found_servers: HashMap<String, BattlelogServer>,
) -> anyhow::Result<HashMap<String, results::RegionResult>> {
    let mut all_regions: results::RegionResult = results::RegionResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
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
        map_players: HashMap::new(),
        mode_players: HashMap::new(),
        settings_players: HashMap::new(),
        playground: HashMap::new(),
        playground_players: HashMap::new(),
    };
    let mut server_stats: HashMap<String, Vec<server_info::ServerInfo>> = HashMap::new();

    let mut regions: HashMap<String, results::RegionResult> = HashMap::new();
    for server in found_servers.values() {
        regions
            .entry(server.region.to_string())
            .and_modify(|region| {
                region.amounts.server_amount += 1;
                region.amounts.soldier_amount += server.soldier_amount;
                region.amounts.queue_amount += server.queue_amount;
                region
                    .maps
                    .entry(server.map.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                region
                    .map_players
                    .entry(server.map.clone())
                    .and_modify(|count| *count += server.soldier_amount)
                    .or_insert(server.soldier_amount);
            })
            .or_insert({
                results::RegionResult {
                    metadata: results::Metadata {
                        region: server.region.to_string(),
                        platform: "pc".to_string(),
                    },
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
                    maps: HashMap::from([(server.map.clone(), 1)]),
                    modes: HashMap::new(),
                    settings: HashMap::new(),
                    owner_platform: HashMap::new(),
                    timestamp: Utc::now(),
                    map_players: HashMap::from([(server.map.clone(), server.soldier_amount)]),
                    mode_players: HashMap::new(),
                    settings_players: HashMap::new(),
                    playground: HashMap::new(),
                    playground_players: HashMap::new(),
                }
            });

        server_stats
            .entry(server.region.to_string())
            .and_modify(|region_info| region_info.push(server.clone().into()))
            .or_insert_with(|| vec![std::convert::Into::<ServerInfo>::into(server.clone())]);

        all_regions
            .maps
            .entry(server.map.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        all_regions
            .map_players
            .entry(server.map.clone())
            .and_modify(|count| *count += server.soldier_amount)
            .or_insert(server.soldier_amount);

        all_regions.amounts.server_amount += 1;
        all_regions.amounts.soldier_amount += server.soldier_amount;
        all_regions.amounts.queue_amount += server.queue_amount;
    }
    regions.insert("ALL".to_string(), all_regions);

    for (region, server_stat) in server_stats {
        match push_server(pool, game_name, &region, "pc", server_stat).await {
            Ok(_) => {}
            Err(e) => log::error!(
                "{} region {} failed to push specific serverinfo: {:#?}",
                game_name,
                region,
                e
            ),
        };
    }

    Ok(regions)
}

async fn get_region_stats(
    pool: &PgPool,
    game_name: &str,
    base_uri: &str,
) -> anyhow::Result<HashMap<String, results::RegionResult>> {
    let found_servers = get_all_regions(game_name, base_uri).await?;
    let result = server_list_to_sum(pool, game_name, found_servers).await?;

    Ok(result)
}

pub async fn gather_battlelog(
    pool: &PgPool,
    influx_client: &influxdb2::Client,
    game_name: &str,
    base_uri: &str,
) -> anyhow::Result<results::RegionResult> {
    let game_result = match get_region_stats(pool, game_name, base_uri).await {
        Ok(result) => {
            // influx
            match influx_db::push_to_database(influx_client, game_name, "pc", &result).await {
                Ok(_) => {}
                Err(e) => log::error!("{} failed to push to influxdb: {:#?}", game_name, e),
            };
            result
        }
        Err(e) => anyhow::bail!("{} gather failed: {:#?}", game_name, e),
    };
    let result = match game_result.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("{} has no ALL region!", game_name),
    };

    Ok(result.to_owned())
}
