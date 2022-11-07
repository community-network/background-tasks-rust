use std::collections::HashMap;

use futures::stream;
use influxdb2::models::{DataPoint, data_point::DataPointError};
use crate::{mongo::MongoClient, structs::results};

pub fn build_data_point(game_name: &str, field: &str, amount: i64) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(game_name)
        .tag("type", "amounts")
        .tag("region", "ALL")
        .tag("platform", "pc")
        .field(field, amount)
        .build()
}

pub async fn push_old_games(influx_client: &influxdb2::Client, mongo_client: &mut MongoClient, mongo_game_name: &str, frontend_game_name: &str) -> anyhow::Result<results::RegionResult> {
    let servers = match mongo_client.gather_old_title(mongo_game_name).await? {
        Some(servers) => servers,
        None => anyhow::bail!("No serverinfo gotten {}", frontend_game_name),
    };

    let mut soldier_amount = 0;
    for server in &servers.server_list {
        soldier_amount += server.numplayers;
    }

    let bucket = "bfStatus";
    let points = vec![
        build_data_point(frontend_game_name, "serverAmount", servers.server_list.len() as i64)?,
        build_data_point(frontend_game_name, "soldierAmount", soldier_amount as i64)?,
    ];
    match influx_client.write(bucket, stream::iter(points)).await {
        Ok(_) => {},
        Err(e) => log::error!("{} failed to push to influxdb: {:#?}", frontend_game_name, e),
    };
    Ok(results::RegionResult { 
        region: "ALL".to_string(),
        amounts: results::RegionAmounts {
            server_amount: servers.server_list.len() as i64,
            soldier_amount: soldier_amount as i64,
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
    })
}