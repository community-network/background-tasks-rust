use futures::stream;
use influxdb2::models::{DataPoint, data_point::DataPointError};
use crate::mongo::MongoClient;

pub fn build_data_point(game_name: &str, field: &str, amount: i64) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(game_name)
        .tag("type", "amounts")
        .tag("region", "ALL")
        .tag("platform", "pc")
        .field(field, amount)
        .build()
}

pub async fn push_old_games(influx_client: &influxdb2::Client, mongo_client: &mut MongoClient, mongo_game_name: &str, frontend_game_name: &str) -> anyhow::Result<()> {
    let servers = match mongo_client.gather_old_title(mongo_game_name).await? {
        Some(servers) => servers,
        None => anyhow::bail!("No serverinfo gotten"),
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
    influx_client.write(bucket, stream::iter(points)).await?;
    Ok(())
}