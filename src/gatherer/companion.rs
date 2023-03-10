use crate::{
    structs::{
        companion::{Regions, ServerFilter, Slots, UnusedValue},
        results,
    },
    MongoClient,
};
use bf_sparta::sparta_api;
use chrono::Utc;
use std::collections::HashMap;

async fn region_players(
    region: &str,
    session: &str,
    game_name: &str,
    platform: &str,
    managed_server_ids: &[String],
) -> anyhow::Result<(results::RegionResult, results::ManagedInfo)> {
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
        kits: UnusedValue {},
        misc: UnusedValue {},
        scales: UnusedValue {},
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
        _ => anyhow::bail!("Unknown platform field: {} for {}", region, game_name),
    };

    let filter_json = serde_json::json!({
        "filterJson": serde_json::to_string(&filters)?,
        "game": game_name,
        "limit": 10000
    });
    let result = sparta_api::get_data_from_ea(
        session,
        platform,
        game_name,
        "GameServer.searchServers",
        filter_json,
    )
    .await?;

    let default = &vec![];
    let servers = result["result"]["gameservers"]
        .as_array()
        .unwrap_or(default);

    let mut region_amounts = results::RegionAmounts {
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

    let mut managed_result = results::ManagedInfo {
        unmanaged_servers: vec![],
    };

    for server in servers {
        let server_soldier_amount = server["slots"]["Soldier"]["current"]
            .as_i64()
            .unwrap_or_default();
        let game_id = server["gameId"].as_str().unwrap_or_default().to_string();

        if !managed_server_ids.contains(&game_id) && server_soldier_amount > 0 {
            managed_result
                .unmanaged_servers
                .push(game_id.parse::<i64>().unwrap());
        }

        mode_amounts
            .entry(server["mapMode"].as_str().unwrap_or_default().to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        map_amounts
            .entry(
                server["mapNamePretty"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            )
            .and_modify(|count| *count += 1)
            .or_insert(1);

        region_amounts.soldier_amount += server_soldier_amount;
        region_amounts.queue_amount += server["slots"]["Queue"]["current"]
            .as_i64()
            .unwrap_or_default();
        region_amounts.spectator_amount += server["slots"]["Spectator"]["current"]
            .as_i64()
            .unwrap_or_default();

        // add dice server count
        if (server["serverType"].as_str().unwrap_or_default() == "OFFICIAL"
            && server["game"].as_str().unwrap_or_default() == "tunguska")
            || (server["ownerId"].as_str().is_none()
                && server["game"].as_str().unwrap_or_default() == "casablanca")
        {
            region_amounts.dice_server_amount += 1;
            region_amounts.dice_soldier_amount += server["slots"]["Soldier"]["current"]
                .as_i64()
                .unwrap_or_default();
            region_amounts.dice_queue_amount += server["slots"]["Queue"]["current"]
                .as_i64()
                .unwrap_or_default();
            region_amounts.dice_spectator_amount += server["slots"]["Spectator"]["current"]
                .as_i64()
                .unwrap_or_default();
        // community server count
        } else {
            region_amounts.community_server_amount += 1;
            region_amounts.community_soldier_amount += server["slots"]["Soldier"]["current"]
                .as_i64()
                .unwrap_or_default();
            region_amounts.community_queue_amount += server["slots"]["Queue"]["current"]
                .as_i64()
                .unwrap_or_default();
            region_amounts.community_spectator_amount += server["slots"]["Spectator"]["current"]
                .as_i64()
                .unwrap_or_default();
        }
    }

    let region_result = results::RegionResult {
        metadata: results::Metadata {
            region: region.to_string(),
            platform: platform.to_string(),
        },
        amounts: region_amounts,
        maps: map_amounts,
        modes: mode_amounts,
        settings: HashMap::new(),
        owner_platform: HashMap::new(),
        timestamp: Utc::now(),
    };

    Ok((region_result, managed_result))
}

async fn get_region_stats(
    game_name: &str,
    old_session: String,
    cookie: bf_sparta::cookie::Cookie,
    platform: &str,
    managed_server_ids: &[String],
    mongo_client: &mut MongoClient,
) -> anyhow::Result<(String, HashMap<String, results::RegionResult>)> {
    let session =
        match sparta_api::check_gateway_session(cookie, &old_session, platform, game_name, "en-us")
            .await
        {
            Ok(session) => session,
            Err(e) => anyhow::bail!("{} session failed: {:#?}", game_name, e),
        };
    let sparta_regions = vec!["EU", "Asia", "NAm", "SAm", "AU", "OC", "Afr", "AC"];
    let mut platform_result: HashMap<String, results::RegionResult> = HashMap::new();
    let mut managed_results = results::ManagedInfo {
        unmanaged_servers: vec![],
    };
    for region in sparta_regions {
        match region_players(
            region,
            &session.session_id,
            game_name,
            platform,
            managed_server_ids,
        )
        .await
        {
            Ok((region_result, managed_result)) => {
                managed_results
                    .unmanaged_servers
                    .append(&mut managed_result.unmanaged_servers.clone());
                platform_result.insert(region_result.clone().metadata.region, region_result);
            }
            Err(e) => {
                log::error!("{} {} region failed: {:#?}", region, game_name, e);
            }
        };
    }

    if platform == "pc" && game_name != "bf4" {
        let unmanaged_players =
            super::gateway_players::gather_players(game_name, managed_results).await?;
        mongo_client
            .push_unmanaged_players(game_name, unmanaged_players)
            .await?;
    }

    let all_regions = results::combine_region_players("ALL", platform, &platform_result).await;
    platform_result.insert("ALL".to_string(), all_regions);
    Ok((session.session_id, platform_result))
}

pub async fn gather_companion(
    mongo_client: &mut MongoClient,
    mut sessions: HashMap<String, String>,
    cookie: bf_sparta::cookie::Cookie,
    game_name: &str,
    frontend_game_name: &str,
    managed_server_ids: &[String],
) -> anyhow::Result<(HashMap<String, String>, results::RegionResult)> {
    let game_platforms = match &game_name.to_string()[..] {
        "tunguska" => vec!["pc", "ps4", "xboxone"],
        "casablanca" => vec!["pc", "ps4", "xboxone"],
        "bf4" => vec!["ps4", "xboxone"],
        _ => vec!["pc"],
    };

    let mut game_result: HashMap<String, HashMap<String, results::RegionResult>> = HashMap::new();
    for platform in game_platforms {
        let (session, platform_result) = match get_region_stats(
            game_name,
            sessions
                .get(platform)
                .unwrap_or(&"".to_string())
                .to_string(),
            cookie.clone(),
            platform,
            managed_server_ids,
            mongo_client,
        )
        .await
        {
            Ok((sessions, platform_result)) => {
                match mongo_client
                    .push_to_database(frontend_game_name, &platform_result)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => log::error!("{} failed to push to influxdb: {:#?}", game_name, e),
                };
                (sessions, platform_result)
            }
            Err(e) => anyhow::bail!("{} failed with platform {}: {:#?}", platform, game_name, e),
        };
        sessions.insert(platform.into(), session);
        game_result.insert(platform.into(), platform_result);
    }

    let combined_platform_regions = results::combine_region_platforms(&game_result).await;
    match mongo_client
        .push_to_database(frontend_game_name, &combined_platform_regions)
        .await
    {
        Ok(_) => {}
        Err(e) => log::error!("{} failed to push to influxdb: {:#?}", game_name, e),
    };
    let result = match combined_platform_regions.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("{} has no ALL region!", game_name),
    };

    Ok((sessions, result.to_owned()))
}
