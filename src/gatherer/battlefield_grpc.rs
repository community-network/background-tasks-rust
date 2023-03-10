use chrono::Utc;
use grpc_rust::{
    grpc::KingstonClient,
    modules::{
        communitygames::{GameFilters, GetFilteredGameServersRequest, ServerPropertyFilters},
        CommunityGames,
    },
};
use std::collections::HashMap;

use crate::{mongo::MongoClient, structs::results};

async fn get_region_stats(
    kingston_client: &KingstonClient,
    managed_server_ids: &[String],
) -> anyhow::Result<(HashMap<String, results::RegionResult>, results::ManagedInfo)> {
    let grpc_regions = HashMap::from([
        (
            "Asia",
            vec!["aws-bah", "aws-bom", "aws-hkg", "aws-nrt", "aws-sin"],
        ),
        ("NAm", vec!["aws-iad", "aws-pdx", "aws-sjc"]),
        ("SAm", vec!["aws-brz", "aws-cmh", "aws-icn"]),
        ("EU", vec!["aws-cdg", "aws-dub", "aws-fra", "aws-lhr"]),
        ("Afr", vec!["aws-cpt"]),
        ("OC", vec!["aws-syd"]),
    ]);
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
    ]);
    let bf2042_modes = HashMap::from([
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

    let mut region_result: HashMap<String, results::RegionResult> = HashMap::new();
    let mut managed_result = results::ManagedInfo {
        unmanaged_servers: vec![],
    };

    for (region, aws_regions) in grpc_regions {
        let mut region_stats: results::RegionResult = results::RegionResult {
            metadata: results::Metadata {
                region: region.to_string(),
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
        };

        for aws_region in aws_regions {
            for map in bf2042_maps.keys() {
                match CommunityGames::get_filtered_game_servers(
                    kingston_client,
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
                            if let Some(game_id) = server.bid {
                                let blaze_id = game_id.blaze_game_id;
                                if !managed_server_ids.contains(&blaze_id.to_string()) {
                                    managed_result.unmanaged_servers.push(blaze_id)
                                }
                            }

                            region_stats.amounts.server_amount += 1;
                            region_stats.amounts.soldier_amount +=
                                server.players.unwrap_or_default().player_amount as i64;
                            region_stats.amounts.queue_amount +=
                                server.que.unwrap_or_default().in_que as i64;
                            region_stats
                                .maps
                                .entry(
                                    bf2042_maps
                                        .get(&server.current_map[..])
                                        .unwrap_or(&"")
                                        .to_string(),
                                )
                                .and_modify(|count| *count += 1)
                                .or_insert(1);
                            region_stats
                                .modes
                                .entry(
                                    bf2042_modes
                                        .get(&server.mode[..])
                                        .unwrap_or(&"")
                                        .to_string(),
                                )
                                .and_modify(|count| *count += 1)
                                .or_insert(1);
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
                                    .entry(setting.param)
                                    .and_modify(|count| *count += 1)
                                    .or_insert(1);
                            }
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

        region_result.insert(region.to_string(), region_stats);
    }

    let all_regions = results::combine_region_players("ALL", "global", &region_result).await;
    region_result.insert("ALL".to_string(), all_regions);
    Ok((region_result, managed_result))
}

pub async fn gather_grpc(
    mongo_client: &mut MongoClient,
    mut sessions: HashMap<String, String>,
    cookie: bf_sparta::cookie::Cookie,
    managed_server_ids: &[String],
) -> anyhow::Result<(HashMap<String, String>, results::RegionResult)> {
    let mut kingston_client =
        KingstonClient::new(sessions.get("pc").unwrap_or(&"".to_string()).to_string()).await?;
    match kingston_client.auth(cookie.clone()).await {
        Ok(_) => {}
        Err(e) => anyhow::bail!("kingston session failed: {:#?}", e),
    };
    let mut managed_results = results::ManagedInfo {
        unmanaged_servers: vec![],
    };
    let game_result = match get_region_stats(&kingston_client, managed_server_ids).await {
        Ok((result, managed_result)) => {
            managed_results
                .unmanaged_servers
                .append(&mut managed_result.unmanaged_servers.clone());
            match mongo_client.push_to_database("bf2042portal", &result).await {
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

    let unmanaged_players =
        super::gateway_players::gather_players("kingston", managed_results).await?;
    mongo_client
        .push_unmanaged_players("kingston", unmanaged_players)
        .await?;

    Ok((sessions, result.to_owned()))
}
