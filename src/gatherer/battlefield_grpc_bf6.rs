use chrono::Utc;
use grpc_rust_bf6::{
    grpc::SantiagoClient,
    modules::{
        play::play::{
            DetailedServerInfoRequest, GameFilters, GetFilteredGameServersRequest,
            ServerListResponseInner, ServerPropertyFilters,
        },
        Play,
    },
};
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::task::JoinSet;

use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{results, server_info},
};

pub async fn check_session(
    mut sessions: HashMap<String, String>,
    cookie: bf_sparta::cookie::Cookie,
    ea_access_token: String,
) -> anyhow::Result<HashMap<String, String>> {
    let mut santiago_client =
        SantiagoClient::new(sessions.get("pc").unwrap_or(&"".to_string()).to_string()).await?;
    match santiago_client
        .ea_desktop_auth(cookie.clone(), ea_access_token)
        .await
    {
        Ok(_) => {}
        Err(e) => anyhow::bail!("santiago session failed: {:#?}", e),
    };
    let servers = Play::get_filtered_game_servers(
        &santiago_client,
        GetFilteredGameServersRequest {
            game_filters: None,
            client_info: None,
            prp_filter: Some(ServerPropertyFilters {
                config_name: None,
                ping_site_list: vec![
                    "aws-bah".into(),
                    "aws-bom".into(),
                    "aws-hkg".into(),
                    "aws-nrt".into(),
                    "aws-sin".into(),
                    "aws-iad".into(),
                    "aws-pdx".into(),
                    "aws-sjc".into(),
                    "aws-brz".into(),
                    "aws-cmh".into(),
                    "aws-icn".into(),
                    "aws-cdg".into(),
                    "aws-dub".into(),
                    "aws-fra".into(),
                    "aws-lhr".into(),
                    "aws-cpt".into(),
                    "aws-syd".into(),
                ],
                query_name: None,
            }),
        },
    )
    .await?;
    let has_result = match servers.servers {
        Some(res) => res.servers.len() > 0,
        None => false,
    };
    if !has_result {
        anyhow::bail!("santiago: no servers on auth check");
    }
    sessions.insert("pc".into(), santiago_client.session_id);
    Ok(sessions)
}

async fn region_players(
    pool: PgPool,
    santiago_client: SantiagoClient,
    region: String,
    aws_regions: Vec<String>,
    run_detailed: bool,
) -> anyhow::Result<(String, results::RegionResult)> {
    let bf6_maps = HashMap::from([
        ("MP_Abbasid", "SiegeOfCairo"),
        ("MP_Aftermath", "EmpireState"),
        ("MP_Battery", "IberianOffensive"),
        ("MP_Capstone", "LiberationPeak"),
        ("MP_Dumbo", "ManhattanBridge"),
        ("MP_FireStorm", "OperationFirestorm"),
        ("MP_Limestone", "SaintsQuarter"),
        ("MP_Outskirts", "NewSobekCity"),
        ("MP_Tungsten", "MirakValley"),
    ]);
    let bf6_modes = HashMap::from([
        ("Breakthrough0", "BreakthroughLarge"),
        ("BreakthroughSmall0", "Breakthrough"),
        ("ConquestSmall0", "Conquest"),
        ("ModBuilderCustom0", "Custom"),
        ("Rush0", "Rush"),
        ("Conquest0", "ConquestLarge"),
    ]);
    let bf6_platform = HashMap::from([
        (0, "unknown"),
        (1, "pc"),
        (2, "ps4"),
        (3, "xboxone"),
        (4, "ps5"),
        (5, "xboxseries"),
        (6, "common"),
        (7, "steam"),
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
        for map in bf6_maps.keys() {
            match Play::get_filtered_game_servers(
                &santiago_client,
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
                },
            )
            .await
            {
                Ok(servers) => {
                    let server_list = match servers.servers {
                        Some(servers) => servers,
                        None => ServerListResponseInner { servers: vec![] },
                    };
                    for server in server_list.servers {
                        let server_map = bf6_maps
                            .get(&server.current_map[..])
                            .unwrap_or(&"")
                            .to_string();
                        let server_mode =
                            bf6_modes.get(&server.mode[..]).unwrap_or(&"").to_string();

                        let soldier_amount =
                            server.players.unwrap_or_default().player_amount as i64;

                        if run_detailed {
                            match Play::get_detailed_server_info(
                                &santiago_client,
                                DetailedServerInfoRequest {
                                    server_id: server.server_id.clone(),
                                },
                            )
                            .await
                            {
                                Ok(result) => {
                                    if let Some(current) = result.server_info {
                                        if let Some(current_server_info) = current.server_info {
                                            region_stats
                                                .playground
                                                .entry(current_server_info.config_name.clone())
                                                .and_modify(|count| *count += 1)
                                                .or_insert(1);
                                            region_stats
                                                .playground_players
                                                .entry(current_server_info.config_name)
                                                .and_modify(|count| *count += soldier_amount)
                                                .or_insert(soldier_amount);
                                        }
                                    }
                                }
                                Err(_) => {}
                            };
                        }

                        region_stats.amounts.server_amount += 1;
                        region_stats.amounts.soldier_amount += soldier_amount;

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
                                bf6_platform
                                    .get(&server.owner.unwrap_or_default().platform_id)
                                    .unwrap_or(&"")
                                    .to_string(),
                            )
                            .and_modify(|count| *count += 1)
                            .or_insert(1);

                        server_stats.push(server_info::ServerInfo {
                            game_id: server.blaze_game_id.to_string(),
                            guid: server.server_id,
                            name: server.prefix,
                            soldiers: soldier_amount,
                            queue: 0,
                            mode: server_mode,
                            map: server_map,
                            is_official: None,
                        });
                    }
                }
                Err(e) => log::error!(
                    "{} santiago region failed with map {}: {:#?}",
                    aws_region,
                    map,
                    e
                ),
            };
        }
    }

    match push_server(&pool, "bf6", &region, "global", server_stats).await {
        Ok(_) => {}
        Err(e) => log::error!(
            "{} region santiago failed to push specific serverinfo: {:#?}",
            region,
            e
        ),
    };

    Ok((region, region_stats))
}

async fn get_region_stats(
    pool: &PgPool,
    santiago_client: &SantiagoClient,
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
            santiago_client.clone(),
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
                log::error!("santiago region failed, with reason: {:#?}", e);
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
    ea_access_token: String,
) -> anyhow::Result<(HashMap<String, String>, results::RegionResult)> {
    let mut santiago_client =
        SantiagoClient::new(sessions.get("pc").unwrap_or(&"".to_string()).to_string()).await?;
    match santiago_client
        .ea_desktop_auth(cookie.clone(), ea_access_token)
        .await
    {
        Ok(_) => {}
        Err(e) => anyhow::bail!("santiago session failed: {:#?}", e),
    };
    let game_result = match get_region_stats(pool, &santiago_client, run_detailed).await {
        Ok(result) => {
            // influx
            match influx_db::push_to_database(influx_client, "bf6portal", "global", &result).await {
                Ok(_) => {}
                Err(e) => log::error!("santiago failed to push to influxdb: {:#?}", e),
            };
            result
        }
        Err(e) => anyhow::bail!("santiago gather failed: {:#?}", e),
    };
    sessions.insert("pc".into(), santiago_client.session_id);
    let result = match game_result.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("santiago has no ALL region!"),
    };

    Ok((sessions, result.to_owned()))
}
