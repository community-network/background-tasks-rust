use std::collections::HashMap;
use futures::future::join_all;
use grpc_rust::{grpc::KingstonClient, modules::{communitygames::{ServerPropertyFilters, GetFilteredGameServersRequest, QName, GameFilters}, CommunityGames}};

use super::global_region_players;

async fn get_region_stats(kingston_client: &KingstonClient) -> anyhow::Result<HashMap<String, super::RegionResult>> {
    let grpc_regions = HashMap::from([
        ("Asia", vec!["aws-bah", "aws-bom", "aws-hkg", "aws-nrt", "aws-sin"]),
        ("NAm", vec!["aws-iad", "aws-pdx", "aws-sjc"]),
        ("SAm", vec!["aws-brz", "aws-cmh", "aws-icn"]),
        ("EU", vec!["aws-cdg", "aws-dub", "aws-fra", "aws-lhr"]),
        ("Afr", vec!["aws-cpt"]),
        ("OC", vec!["aws-syd"])
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
        ("MP_Ridge", "Exposure")
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

    let mut regions: HashMap<String, super::RegionResult> = HashMap::new();

    for (region, aws_regions) in grpc_regions {
        let mut region_stats: super::RegionResult = super::RegionResult { 
            region: region.to_string(),
            amounts: super::RegionAmounts {
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
        };

        let mut tasks = vec![];

        for aws_region in aws_regions {
            for map in bf2042_maps.keys() {
                tasks.push(CommunityGames::get_filtered_game_servers(
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
                ));
            }
        }

        let result = join_all(tasks).await;
        for item in result {
            match item {
                Ok(servers) => {
                    for server in servers.servers {
                        region_stats.amounts.server_amount += 1;
                        region_stats.amounts.soldier_amount += server.players.unwrap_or_default().player_amount as i64;
                        region_stats.amounts.queue_amount += server.que.unwrap_or_default().in_que as i64;
                        region_stats.maps.entry(bf2042_maps.get(&server.current_map[..]).unwrap_or(&"").to_string())
                            .and_modify(|count| *count += 1).or_insert(1);
                        region_stats.modes.entry(bf2042_modes.get(&server.mode[..]).unwrap_or(&"").to_string())
                            .and_modify(|count| *count += 1).or_insert(1);
                        region_stats.owner_platform.entry(bf2042_platform.get(&server.owner.unwrap_or_default().platform_id).unwrap_or(&"").to_string())
                            .and_modify(|count| *count += 1).or_insert(1);
                        for setting in server.settings {
                            region_stats.settings.entry(setting.param).and_modify(|count| *count += 1).or_insert(1);
                        }
                    }
                },
                Err(_) => {},
            }
        }

        regions.insert(region.to_string(), region_stats);
    }

    let all_regions = global_region_players(&regions).await?;
    regions.insert("ALL".to_string(), all_regions);
    Ok(regions)
}

pub async fn gather_grpc(influx_client: &influxdb2::Client, mut sessions: HashMap<String, String>, cookie: bf_sparta::cookie::Cookie) -> anyhow::Result<(HashMap<String, String>, HashMap<String, super::RegionResult>)> {
    let mut kingston_client = KingstonClient::new(sessions.get("pc").unwrap_or(&"".to_string()).to_string()).await?;
    match kingston_client.auth(cookie.clone()).await {
        Ok(_) => {},
        Err(e) => panic!("kingston session failed: {:#?}", e),
    };
    let game_result = match get_region_stats(&kingston_client).await {
        Ok(result) => {
            match super::push_to_database(influx_client, "bf2042portal", "global", &result).await {
                Ok(_) => {},
                Err(_) => todo!(),
            };
            result
        },
        Err(e) => panic!("kingston failed: {:#?}", e),
    };
    sessions.insert("pc".into(), kingston_client.session_id);
    Ok((sessions, game_result))
}