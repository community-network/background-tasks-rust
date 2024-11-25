use chrono::Utc;
use grpc_rust::{
    grpc::KingstonClient,
    modules::{
        communitygames::{
            DetailedServerInfoRequest, GameFilters, GetFilteredGameServersRequest,
            ServerPropertyFilters,
        },
        CommunityGames,
    },
};
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::task::JoinSet;

use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{results, server_info},
};

async fn region_players(
    pool: PgPool,
    kingston_client: KingstonClient,
    region: String,
    aws_regions: Vec<String>,
    run_detailed: bool,
) -> anyhow::Result<(String, results::RegionResult)> {
    let bf2042_maps = HashMap::from([
        ("MP_Harbor", "AricaHarbor"),
        ("MP_LightHouse", "Valparaiso"),
        ("MP_Frost", "BattleoftheBulge"),
        ("MP_Oasis", "ElAlamein"),
        ("MP_Rural", "CaspianBorder"),
        ("MP_Port", "NoshahrCanals"),
        ("MP_Orbital", "Orbital"),
        ("MP_Hourglass", "Hourglass"),
        ("MP_Kaleidoscope", "Kaleidoscope"),
        ("MP_Irreversible", "Breakaway"),
        ("MP_Discarded", "Discarded"),
        ("MP_LongHaul", "Manifest"),
        ("MP_TheWall", "Renewal"),
        ("MP_Ridge", "Exposure"),
        ("MP_LightsOut", "Spearhead"),
        ("MP_Boulder", "Flashpoint"),
        ("MP_Scarred", "Reclaimed"),
    ]);
    let bf2042_modes = HashMap::from([
        ("Breakthrough", "Breakthrough"),
        ("BreakthroughSmall", "Breakthroughsmall"),
        ("ConquestSmall", "Conquest"),
        ("ModBuilderCustom", "Custom"),
        ("Rush", "Rush"),
        ("Conquest", "Conquestlarge"),
    ]);
    let bf2042_platform = HashMap::from([
        (0, "unknown"),
        (1, "pc"),
        (2, "ps4"),
        (3, "xboxone"),
        (4, "ps5"),
        (5, "xboxseries"),
    ]);
    let mut server_stats = vec![];

    let mut region_stats: results::RegionResult = results::RegionResult {
        metadata: results::Metadata {
            region: region.clone(),
            platform: "global".to_string(),
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

    for aws_region in aws_regions {
        for map in bf2042_maps.keys() {
            match CommunityGames::get_filtered_game_servers(
                &kingston_client,
                GetFilteredGameServersRequest {
                    game_filters: Some(GameFilters {
                        gamemodes: vec![],
                        levels: vec![map.to_string()],
                    }),
                    client_info: None,
                    prp_filter: Some(ServerPropertyFilters {
                        config_name: None,
                        ping_site_list: vec![aws_region.to_string()],
                        query_name: None,
                    }),
                    limit: 250,
                },
            )
            .await
            {
                Ok(servers) => {
                    for server in servers.servers {
                        let mut current_game_id = 0;
                        if let Some(game_id) = server.bid {
                            let blaze_id = game_id.blaze_game_id;
                            if blaze_id != 0 {
                                current_game_id = blaze_id;
                            }
                        }

                        let server_map = bf2042_maps
                            .get(&server.current_map[..])
                            .unwrap_or(&"")
                            .to_string();
                        let server_mode = bf2042_modes
                            .get(&server.mode[..])
                            .unwrap_or(&"")
                            .to_string();

                        let soldier_amount =
                            server.players.unwrap_or_default().player_amount as i64;
                        let queue_amount = server.que.unwrap_or_default().in_que as i64;

                        if run_detailed {
                            match CommunityGames::get_detailed_server_info_v2(
                                &kingston_client,
                                DetailedServerInfoRequest {
                                    server_id: server.server_id.clone(),
                                },
                            )
                            .await
                            {
                                Ok(result) => {
                                    if let Some(current) = result.server_info {
                                        if let Some(current_server_info) = current.server_info {
                                            if let Some(config_name) =
                                                current_server_info.config_name
                                            {
                                                region_stats
                                                    .playground
                                                    .entry(config_name.config_name.clone())
                                                    .and_modify(|count| *count += 1)
                                                    .or_insert(1);
                                                region_stats
                                                    .playground_players
                                                    .entry(config_name.config_name)
                                                    .and_modify(|count| *count += soldier_amount)
                                                    .or_insert(soldier_amount);
                                            }
                                        }
                                    }
                                }
                                Err(_) => {}
                            };
                        }

                        region_stats.amounts.server_amount += 1;
                        region_stats.amounts.soldier_amount += soldier_amount;

                        region_stats.amounts.queue_amount += queue_amount;
                        region_stats
                            .maps
                            .entry(server_map.clone())
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                        region_stats
                            .map_players
                            .entry(server_map.clone())
                            .and_modify(|count| *count += soldier_amount)
                            .or_insert(soldier_amount);
                        region_stats
                            .modes
                            .entry(server_mode.clone())
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                        region_stats
                            .mode_players
                            .entry(server_mode.clone())
                            .and_modify(|count| *count += soldier_amount)
                            .or_insert(soldier_amount);
                        region_stats
                            .owner_platform
                            .entry(
                                bf2042_platform
                                    .get(&server.owner.unwrap_or_default().platform_id)
                                    .unwrap_or(&"")
                                    .to_string(),
                            )
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                        for setting in server.settings {
                            region_stats
                                .settings
                                .entry(setting.param.clone())
                                .and_modify(|count| *count += 1)
                                .or_insert(1);
                            region_stats
                                .settings_players
                                .entry(setting.param)
                                .and_modify(|count| *count += soldier_amount)
                                .or_insert(soldier_amount);
                        }

                        server_stats.push(server_info::ServerInfo {
                            game_id: current_game_id.to_string(),
                            guid: server.server_id,
                            name: server.prefix,
                            soldiers: soldier_amount,
                            queue: queue_amount,
                            mode: server_mode,
                            map: server_map,
                            is_official: None,
                        });
                    }
                }
                Err(e) => log::error!(
                    "{} kingston region failed with map {}: {:#?}",
                    aws_region,
                    map,
                    e
                ),
            };
        }
    }

    match push_server(&pool, "bf2042", &region, "global", server_stats).await {
        Ok(_) => {}
        Err(e) => log::error!(
            "{} region kingston failed to push specific serverinfo: {:#?}",
            region,
            e
        ),
    };

    Ok((region, region_stats))
}

async fn get_region_stats(
    pool: &PgPool,
    kingston_client: &KingstonClient,
    run_detailed: bool,
) -> anyhow::Result<HashMap<String, results::RegionResult>> {
    let grpc_regions = HashMap::from([
        (
            "Asia",
            vec![
                "aws-bah".into(),
                "aws-bom".into(),
                "aws-hkg".into(),
                "aws-nrt".into(),
                "aws-sin".into(),
            ],
        ),
        (
            "NAm",
            vec!["aws-iad".into(), "aws-pdx".into(), "aws-sjc".into()],
        ),
        (
            "SAm",
            vec!["aws-brz".into(), "aws-cmh".into(), "aws-icn".into()],
        ),
        (
            "EU",
            vec![
                "aws-cdg".into(),
                "aws-dub".into(),
                "aws-fra".into(),
                "aws-lhr".into(),
            ],
        ),
        ("Afr", vec!["aws-cpt".into()]),
        ("OC", vec!["aws-syd".into()]),
    ]);

    let mut region_result: HashMap<String, results::RegionResult> = HashMap::new();

    let mut set = JoinSet::new();
    for (region, aws_regions) in grpc_regions {
        set.spawn(region_players(
            pool.to_owned(),
            kingston_client.clone(),
            region.to_owned(),
            aws_regions,
            run_detailed.clone(),
        ));
    }

    while let Some(res) = set.join_next().await {
        let out = res?;
        match out {
            Ok((region, region_stats)) => {
                region_result.insert(region.to_string(), region_stats);
            }
            Err(e) => {
                log::error!("Kingston region failed, with reason: {:#?}", e);
            }
        }
    }

    let all_regions = results::combine_region_players("ALL", "global", &region_result).await;
    region_result.insert("ALL".to_string(), all_regions);
    Ok(region_result)
}

pub async fn gather_grpc(
    pool: &PgPool,
    influx_client: &influxdb2::Client,
    mut sessions: HashMap<String, String>,
    cookie: bf_sparta::cookie::Cookie,
    run_detailed: bool,
) -> anyhow::Result<(HashMap<String, String>, results::RegionResult)> {
    let mut kingston_client =
        KingstonClient::new(sessions.get("pc").unwrap_or(&"".to_string()).to_string()).await?;
    match kingston_client.auth(cookie.clone()).await {
        Ok(_) => {}
        Err(e) => anyhow::bail!("kingston session failed: {:#?}", e),
    };
    let game_result = match get_region_stats(pool, &kingston_client, run_detailed).await {
        Ok(result) => {
            // influx
            match influx_db::push_to_database(influx_client, "bf2042portal", "global", &result)
                .await
            {
                Ok(_) => {}
                Err(e) => log::error!("kingston failed to push to influxdb: {:#?}", e),
            };
            result
        }
        Err(e) => anyhow::bail!("kingston gather failed: {:#?}", e),
    };
    sessions.insert("pc".into(), kingston_client.session_id);
    let result = match game_result.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("kingston has no ALL region!"),
    };

    Ok((sessions, result.to_owned()))
}
