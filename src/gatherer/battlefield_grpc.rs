use std::collections::HashMap;
use grpc_rust::{grpc::KingstonClient, modules::{communitygames::{ServerPropertyFilters, GetFilteredGameServersRequest, QName, GameFilters}, CommunityGames}};

async fn get_region_stats(kingston_client: &KingstonClient) -> anyhow::Result<()> {
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

    for (region, aws_regions) in grpc_regions {
        for aws_region in aws_regions {
            for map in bf2042_maps.keys() {
                let servers = CommunityGames::get_filtered_game_servers(
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
                .await?;
                println!("{:#?}", servers);
            }
        }
    }
    Ok(())
}

pub async fn gather_grpc(influx_client: &influxdb2::Client, mut sessions: HashMap<String, String>, cookie: bf_sparta::cookie::Cookie) -> anyhow::Result<(HashMap<String, String>, HashMap<String, HashMap<String, super::RegionResult>>)> {
    let mut game_result: HashMap<String, HashMap<String, super::RegionResult>> = HashMap::new();
    let mut kingston_client = KingstonClient::new("".to_string()).await?;
    let session_res = kingston_client.auth(cookie.clone()).await;

    get_region_stats(&kingston_client).await?;
    //game_result.insert("global".into(), result);
    sessions.insert("pc".into(), kingston_client.session_id);
    Ok((sessions, game_result))
}