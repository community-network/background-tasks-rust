use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{
        companion::{Regions, ServerFilter, Slots, UnusedValue},
        results, server_info,
    },
};
use bf_sparta::sparta_api;
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::task::JoinSet;

async fn filter_maps(game_name: &str) -> HashMap<String, String> {
    match game_name {
        "tunguska" => HashMap::from([
            ("MP_ItalianCoast".into(), "off".into()),
            ("MP_Hell".into(), "off".into()),
            ("MP_Desert".into(), "off".into()),
            ("MP_River".into(), "off".into()),
            ("MP_FaoFortress".into(), "off".into()),
            ("MP_Islands".into(), "off".into()),
            ("MP_ShovelTown".into(), "off".into()),
            ("MP_MountainFort".into(), "off".into()),
            ("MP_Underworld".into(), "off".into()),
            ("MP_Amiens".into(), "off".into()),
            ("MP_Verdun".into(), "off".into()),
            ("MP_Suez".into(), "off".into()),
            ("MP_Scar".into(), "off".into()),
            ("MP_Chateau".into(), "off".into()),
            ("MP_Giant".into(), "off".into()),
            ("MP_Forest".into(), "off".into()),
            ("MP_Valley".into(), "off".into()),
            ("MP_Ridge".into(), "off".into()),
            ("MP_Harbor".into(), "off".into()),
            ("MP_Volga".into(), "off".into()),
            ("MP_Beachhead".into(), "off".into()),
            ("MP_Graveyard".into(), "off".into()),
            ("MP_Blitz".into(), "off".into()),
            ("MP_Ravines".into(), "off".into()),
            ("MP_Bridge".into(), "off".into()),
            ("MP_Offensive".into(), "off".into()),
            ("MP_Fields".into(), "off".into()),
            ("MP_Tsaritsyn".into(), "off".into()),
            ("MP_Trench".into(), "off".into()),
            ("MP_Alps".into(), "off".into()),
        ]),
        "casablanca" => HashMap::from([
            ("MP_ArcticFjell".into(), "off".into()),
            ("MP_ArcticFjord".into(), "off".into()),
            ("MP_Arras".into(), "off".into()),
            ("MP_Devastation".into(), "off".into()),
            ("MP_Escaut".into(), "off".into()),
            ("MP_Foxhunt".into(), "off".into()),
            ("MP_Halfaya".into(), "off".into()),
            ("MP_Rotterdam".into(), "off".into()),
            ("MP_Hannut".into(), "off".into()),
            ("MP_Crete".into(), "off".into()),
            ("MP_Kalamas".into(), "off".into()),
            // "MP_Norway".into(), "off".into()),
            ("MP_Provence".into(), "off".into()),
            ("MP_SandAndSea".into(), "off".into()),
            ("MP_Bunker".into(), "off".into()),
            ("MP_IwoJima".into(), "off".into()),
            ("MP_TropicIslands".into(), "off".into()),
            ("MP_WakeIsland".into(), "off".into()),
            ("MP_Jungle".into(), "off".into()),
            ("MP_Libya".into(), "off".into()),
        ]),
        _ => HashMap::from([
            ("MP_Abandoned".into(), "off".into()),
            ("MP_Damage".into(), "off".into()),
            ("MP_Flooded".into(), "off".into()),
            ("MP_Journey".into(), "off".into()),
            ("MP_Naval".into(), "off".into()),
            ("MP_Prison".into(), "off".into()),
            ("MP_Resort".into(), "off".into()),
            ("MP_Siege".into(), "off".into()),
            ("MP_TheDish".into(), "off".into()),
            ("MP_Tremors".into(), "off".into()),
            ("XP0_Caspian".into(), "off".into()),
            ("XP0_Firestorm".into(), "off".into()),
            ("XP0_Metro".into(), "off".into()),
            ("XP0_Oman".into(), "off".into()),
            ("XP1_001".into(), "off".into()),
            ("XP1_002".into(), "off".into()),
            ("XP1_003".into(), "off".into()),
            ("XP1_004".into(), "off".into()),
            ("XP2_001".into(), "off".into()),
            ("XP2_002".into(), "off".into()),
            ("XP2_003".into(), "off".into()),
            ("XP2_004".into(), "off".into()),
            ("XP3_MarketPl".into(), "off".into()),
            ("XP3_Prpganda".into(), "off".into()),
            ("XP3_UrbanGdn".into(), "off".into()),
            ("XP3_WtrFront".into(), "off".into()),
            ("XP4_Arctic".into(), "off".into()),
            ("XP4_SubBase".into(), "off".into()),
            ("XP4_Titan".into(), "off".into()),
            ("XP4_WlkrFtry".into(), "off".into()),
            ("XP5_Night_01".into(), "off".into()),
            ("XP6_CMP".into(), "off".into()),
            ("XP7_Valley".into(), "off".into()),
        ]),
    }
}

// async fn gather_map_players()

async fn region_players(
    pool: PgPool,
    region: String,
    session: String,
    game_name: String,
    frontend_game_name: String,
    platform: String,
) -> anyhow::Result<results::RegionResult> {
    let game_maps = filter_maps(&game_name).await;
    let default = &vec![];

    let mut server_stats = vec![];

    let mut region_amounts = results::RegionAmounts {
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
    };
    let mut map_amounts: HashMap<String, i64> = HashMap::new();
    let mut mode_amounts: HashMap<String, i64> = HashMap::new();
    let mut map_player_amounts: HashMap<String, i64> = HashMap::new();
    let mut mode_player_amounts: HashMap<String, i64> = HashMap::new();

    for current_map in game_maps.clone().keys() {
        let off = "off";
        let on = "on";
        let mut filters = ServerFilter {
            version: 6,
            name: "".to_string(),
            vehicles: UnusedValue {},
            weapon_classes: UnusedValue {},
            slots: Slots {
                one_to_five: on,
                six_to_ten: on,
                ten_plus: on,
                none: on,
            },
            regions: Regions {
                eu: off,
                asia: off,
                nam: off,
                sam: off,
                au: off,
                oc: off,
                afr: off,
                ac: off,
            },
            maps: game_maps.clone(),
            kits: UnusedValue {},
            misc: UnusedValue {},
            scales: UnusedValue {},
        };

        match &region[..] {
            "EU" => filters.regions.eu = on,
            "Asia" => filters.regions.asia = on,
            "NAm" => filters.regions.nam = on,
            "SAm" => filters.regions.sam = on,
            "AU" => filters.regions.au = on,
            "OC" => filters.regions.oc = on,
            "Afr" => filters.regions.afr = on,
            "AC" => filters.regions.ac = on,
            _ => anyhow::bail!("Unknown platform field: {} for {}", &region, &game_name),
        };

        *filters
            .maps
            .entry(current_map.clone().to_string())
            .or_insert_with(|| on.to_string()) = on.to_string();

        let filter_json = serde_json::json!({
            "filterJson": serde_json::to_string(&filters)?,
            "game": game_name,
            "limit": 10000
        });
        let result = sparta_api::get_data_from_ea(
            &session,
            &platform,
            &game_name,
            "GameServer.searchServers",
            filter_json,
        )
        .await?;
        let servers = result["result"]["gameservers"]
            .as_array()
            .unwrap_or(default);
        region_amounts.server_amount += servers.len() as i64;

        for server in servers {
            let slots = &server["slots"];
            let server_soldier_amount = slots["Soldier"]["current"].as_i64().unwrap_or_default();
            let server_queue_amount = slots["Queue"]["current"].as_i64().unwrap_or_default();
            let server_spectator_amount =
                slots["Spectator"]["current"].as_i64().unwrap_or_default();

            let map_name = server["mapNamePretty"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let mode_name = server["mapMode"].as_str().unwrap_or_default().to_string();

            mode_amounts
                .entry(mode_name.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            mode_player_amounts
                .entry(mode_name.clone())
                .and_modify(|count| *count += server_soldier_amount)
                .or_insert(server_soldier_amount);

            map_amounts
                .entry(map_name.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            map_player_amounts
                .entry(map_name.clone())
                .and_modify(|count| *count += server_soldier_amount)
                .or_insert(server_soldier_amount);

            region_amounts.soldier_amount += server_soldier_amount;
            region_amounts.queue_amount += server_queue_amount;
            region_amounts.spectator_amount += server_spectator_amount;

            let mut is_official = false;
            // add dice server count
            if (server["serverType"].as_str().unwrap_or_default() == "OFFICIAL"
                && server["game"].as_str().unwrap_or_default() == "tunguska")
                || (server["ownerId"].as_str().is_none()
                    && server["game"].as_str().unwrap_or_default() == "casablanca")
            {
                is_official = true;
                region_amounts.dice_server_amount += 1;
                region_amounts.dice_soldier_amount += server_soldier_amount;
                region_amounts.dice_queue_amount += server_queue_amount;
                region_amounts.dice_spectator_amount += server_spectator_amount;
            // community server count
            } else {
                region_amounts.community_server_amount += 1;
                region_amounts.community_soldier_amount += server_soldier_amount;
                region_amounts.community_queue_amount += server_queue_amount;
                region_amounts.community_spectator_amount += server_spectator_amount;
            }

            server_stats.push(server_info::ServerInfo {
                game_id: server["gameId"].as_str().unwrap_or_default().to_owned(),
                guid: server["guid"].as_str().unwrap_or_default().to_owned(),
                name: server["name"].as_str().unwrap_or_default().to_owned(),
                soldiers: server_soldier_amount,
                queue: server_queue_amount,
                mode: mode_name,
                map: map_name,
                is_official: Some(is_official),
            });
        }
    }

    match push_server(&pool, &frontend_game_name, &region, &platform, server_stats).await {
        Ok(_) => {}
        Err(e) => log::error!(
            "{} region {} failed to push specific serverinfo: {:#?}",
            game_name,
            region,
            e
        ),
    };

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
        map_players: map_player_amounts,
        mode_players: mode_player_amounts,
        settings_players: HashMap::new(),
        playground: HashMap::new(),
        playground_players: HashMap::new(),
    };

    Ok(region_result)
}

async fn get_region_stats(
    pool: &PgPool,
    game_names: (&str, &str),
    old_session: String,
    cookie: bf_sparta::cookie::Cookie,
    platform: &str,
) -> anyhow::Result<(String, HashMap<String, results::RegionResult>)> {
    let (game_name, frontend_game_name) = game_names;
    let session =
        match sparta_api::check_gateway_session(cookie, &old_session, platform, game_name, "en-us")
            .await
        {
            Ok(session) => session,
            Err(e) => anyhow::bail!("{} session failed: {:#?}", game_name, e),
        };
    let sparta_regions = vec!["EU", "Asia", "NAm", "SAm", "AU", "OC", "Afr", "AC"];
    let mut platform_result: HashMap<String, results::RegionResult> = HashMap::new();

    let mut set = JoinSet::new();
    for region in sparta_regions {
        set.spawn(region_players(
            (*pool).to_owned(),
            (*region).to_owned(),
            session.session_id.clone(),
            (*game_name).to_owned(),
            (*frontend_game_name).to_owned(),
            (*platform).to_owned(),
        ));
    }

    while let Some(res) = set.join_next().await {
        let out = res?;
        match out {
            Ok(region_result) => {
                platform_result.insert(region_result.clone().metadata.region, region_result);
            }
            Err(e) => {
                log::error!("{} region failed: {:#?}", game_name, e);
            }
        };
    }

    let all_regions = results::combine_region_players("ALL", platform, &platform_result).await;
    platform_result.insert("ALL".to_string(), all_regions);
    Ok((session.session_id, platform_result))
}

pub async fn gather_companion(
    pool: &PgPool,
    influx_client: &influxdb2::Client,
    mut sessions: HashMap<String, String>,
    cookie: bf_sparta::cookie::Cookie,
    game_name: &str,
    frontend_game_name: &str,
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
            pool,
            (game_name, frontend_game_name),
            sessions
                .get(platform)
                .unwrap_or(&"".to_string())
                .to_string(),
            cookie.clone(),
            platform,
        )
        .await
        {
            Ok((sessions, platform_result)) => {
                // influx
                match influx_db::push_to_database(
                    influx_client,
                    frontend_game_name,
                    platform,
                    &platform_result,
                )
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
    // influx
    match influx_db::push_to_database(
        influx_client,
        frontend_game_name,
        "global",
        &combined_platform_regions,
    )
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
