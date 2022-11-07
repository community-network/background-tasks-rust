use std::collections::HashMap;
use futures::stream;
use influxdb2::models::{DataPoint, data_point::DataPointError};

use crate::structs::results;

pub fn build_data_point(frontend_game_name: &str, data_type: &str, region: &str, platform: &str, field: &str, amount: &i64) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(frontend_game_name)
        .tag("type", data_type)
        .tag("region", region)
        .tag("platform", platform)
        .field(field, *amount)
        .build()
}
pub async fn push_to_database(influx_client: &influxdb2::Client, frontend_game_name: &str, platform: &str, platform_result: &HashMap<String, results::RegionResult>) -> anyhow::Result<()> {
    let bucket = "bfStatus";
    for (key, value) in platform_result {
        let mut points = vec![
            build_data_point(frontend_game_name, "amounts", key, platform, "serverAmount", &value.amounts.server_amount)?,
            build_data_point(frontend_game_name, "amounts", key, platform, "soldierAmount", &value.amounts.soldier_amount)?,
            build_data_point(frontend_game_name, "amounts", key, platform, "queueAmount", &value.amounts.queue_amount)?,
        ];
        if vec!["bf1", "bfv", "bf4"].contains(&frontend_game_name) {
            points.append(&mut vec![
                build_data_point(frontend_game_name, "amounts", key, platform, "spectatorAmount", &value.amounts.spectator_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "diceServerAmount", &value.amounts.dice_server_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "diceSoldierAmount", &value.amounts.dice_soldier_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "diceQueueAmount", &value.amounts.dice_queue_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "diceSpectatorAmount", &value.amounts.dice_soldier_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "communityServerAmount", &value.amounts.community_server_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "communitySoldierAmount", &value.amounts.community_soldier_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "communityQueueAmount", &value.amounts.community_queue_amount)?,
                build_data_point(frontend_game_name, "amounts", key, platform, "communitySpectatorAmount", &value.amounts.community_spectator_amount)?,
            ]);
        }

        for (key, value) in &value.maps {
            points.push(build_data_point(frontend_game_name, "maps", key, platform, key, value)?);
        }
        for (key, value) in &value.modes {
            points.push(build_data_point(frontend_game_name, "modes", key, platform, key, value)?);
        }
        for (key, value) in &value.owner_platform {
            points.push(build_data_point(frontend_game_name, "ownerPlatform", key, platform, key, value)?);
        }
        for (key, value) in &value.settings {
            points.push(build_data_point(frontend_game_name, "settings", key, platform, key, value)?);
        }

        influx_client.write(bucket, stream::iter(points)).await?;
    }
    Ok(())
}

pub async fn push_totals(influx_client: &influxdb2::Client, global_result: results::RegionResult) -> anyhow::Result<()> {
    let bucket = "bfStatus";
    let points = vec![
        build_data_point("global", "amounts", "ALL", "global", "serverAmount", &global_result.amounts.server_amount)?,
        build_data_point("global", "amounts", "ALL", "global", "soldierAmount", &global_result.amounts.soldier_amount)?,
    ];
    influx_client.write(bucket, stream::iter(points)).await?;
    Ok(())
}