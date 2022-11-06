use std::collections::HashMap;

use crate::structs::companion::{ServerFilter, UnusedValue, Slots, Regions};
use bf_sparta::sparta_api;

use super::global_region_players;

async fn region_players(region: &str, session: &String, game_name: &str, platform: &str) -> anyhow::Result<super::RegionResult> {
    let mut filters = ServerFilter {
        version: 6,
        name: "".to_string(),
        vehicles: UnusedValue {},
        weapon_classes: UnusedValue {},
        slots: Slots {
            one_to_five: "on".to_string(),
            six_to_ten: "on".to_string(),
            ten_plus: "on".to_string(),
            none: "on".to_string(),
        },
        regions: Regions {
            eu: "off".to_string(),
            asia: "off".to_string(),
            nam: "off".to_string(),
            sam: "off".to_string(),
            au: "off".to_string(),
            oc: "off".to_string(),
            afr: "off".to_string(),
            ac: "off".to_string(),
        },
        kits:  UnusedValue {},
        misc:  UnusedValue {},
        scales:  UnusedValue {},
    };

    match region {
        "EU" => filters.regions.eu = "on".to_string(),
        "Asia" => filters.regions.asia = "on".to_string(),
        "NAm" => filters.regions.nam = "on".to_string(),
        "SAm" => filters.regions.sam = "on".to_string(),
        "AU" => filters.regions.au = "on".to_string(),
        "OC" => filters.regions.oc = "on".to_string(),
        "Afr" => filters.regions.afr = "on".to_string(),
        "AC" => filters.regions.ac = "on".to_string(),
        _ => panic!("Unknown platform field")
    };

    let filter_json = serde_json::json!({
        "filterJson": serde_json::to_string(&filters)?,
        "game": game_name,
        "limit": 10000
    });
    
    let result = sparta_api::get_data_from_ea(session, platform, game_name, "GameServer.searchServers", filter_json).await?;

    let default = &vec![];
    let servers = result["result"]["gameservers"].as_array().unwrap_or(default);

    let mut region_amounts = super::RegionAmounts {
        server_amount: servers.len() as i64,
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
    };
    let mut map_amounts: HashMap<String, i64> = HashMap::new();
    let mut mode_amounts: HashMap<String, i64> = HashMap::new();

    for server in servers {
        mode_amounts.entry(server["mapMode"].as_str().unwrap_or_default().to_string())
            .and_modify(|count| *count += 1).or_insert(1);

        map_amounts.entry(server["mapNamePretty"].as_str().unwrap_or_default().to_string())
            .and_modify(|count| *count += 1).or_insert(1);
        
        
        region_amounts.soldier_amount += server["slots"]["Soldier"]["current"].as_i64().unwrap_or_default();
        region_amounts.queue_amount += server["slots"]["Queue"]["current"].as_i64().unwrap_or_default();
        region_amounts.spectator_amount += server["slots"]["Spectator"]["current"].as_i64().unwrap_or_default();
        if server["serverType"].as_str().unwrap_or_default() == "OFFICIAL" && server["game"].as_str().unwrap_or_default() == "tunguska" {
            region_amounts.dice_server_amount += 1;
            region_amounts.dice_soldier_amount += server["slots"]["Soldier"]["current"].as_i64().unwrap_or_default();
            region_amounts.dice_queue_amount += server["slots"]["Queue"]["current"].as_i64().unwrap_or_default();
            region_amounts.dice_spectator_amount += server["slots"]["Spectator"]["current"].as_i64().unwrap_or_default();
        } else if server["ownerId"].as_str().is_none() && server["game"].as_str().unwrap_or_default() == "casablanca" {
            region_amounts.dice_server_amount += 1;
            region_amounts.dice_soldier_amount += server["slots"]["Soldier"]["current"].as_i64().unwrap_or_default();
            region_amounts.dice_queue_amount += server["slots"]["Queue"]["current"].as_i64().unwrap_or_default();
            region_amounts.dice_spectator_amount += server["slots"]["Spectator"]["current"].as_i64().unwrap_or_default();
        } else {
            region_amounts.community_server_amount += 1;
            region_amounts.community_soldier_amount += server["slots"]["Soldier"]["current"].as_i64().unwrap_or_default();
            region_amounts.community_queue_amount += server["slots"]["Queue"]["current"].as_i64().unwrap_or_default();
            region_amounts.community_spectator_amount += server["slots"]["Spectator"]["current"].as_i64().unwrap_or_default();
        }
    }

    Ok(super::RegionResult {
        region: region.to_string(),
        amounts: region_amounts,
        maps: map_amounts,
        modes: mode_amounts,
        settings: todo!(),
        owner_platform: todo!(),
    })
}

async fn get_region_stats(game_name: &str, old_session: String, cookie: bf_sparta::cookie::Cookie, platform: &str) -> anyhow::Result<(String, HashMap<String, super::RegionResult>)> {
    let session = sparta_api::check_gateway_session(cookie, &old_session, platform, game_name, "en-us").await?;
    let sparta_regions = vec!["EU", "Asia", "NAm", "SAm", "AU", "OC", "Afr", "AC"];
    let mut platform_result: HashMap<String, super::RegionResult> = HashMap::new();
    for region in sparta_regions {
        let result = region_players(region, &session.session_id, game_name, platform).await?;
        platform_result.insert(region.to_string(), result);
    }
    let all_regions = global_region_players(&platform_result).await?;
    platform_result.insert("ALL".to_string(), all_regions);
    Ok((session.session_id, platform_result))
}

pub async fn gather_companion(influx_client: &influxdb2::Client, mut sessions: HashMap<String, String>, cookie: bf_sparta::cookie::Cookie, game_name: &str, frontend_game_name: &str) -> anyhow::Result<(HashMap<String, String>, HashMap<String, HashMap<String, super::RegionResult>>)> {
    let game_platforms = match &game_name.to_string()[..] {
        "tunguska" => vec!["pc", "ps4", "xboxone"],
        "casablanca" => vec!["pc", "ps4", "xboxone"],
        "bf4" => vec!["ps4", "xboxone"],
        _ => vec!["pc"],
    };
    
    let mut game_result: HashMap<String, HashMap<String, super::RegionResult>> = HashMap::new();
    for platform in game_platforms {
        let (session, platform_result) = match get_region_stats(game_name, sessions.get(platform).unwrap_or(&"".to_string()).to_string(), cookie.clone(), platform).await {
            Ok((sessions, platform_result)) => {
                match super::push_to_database(influx_client, frontend_game_name, platform, &platform_result).await {
                    Ok(_) => {},
                    Err(_) => todo!(),
                };
                (sessions, platform_result)
            },
            Err(e) => panic!("{} failed with platform {}: {:#?}", platform, game_name, e),
        };
        sessions.insert(platform.into(), session);
        game_result.insert(platform.into(), platform_result);
    }
    
    Ok((sessions, game_result))
}