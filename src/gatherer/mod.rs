pub mod old_games;
pub mod server_manager;
pub mod companion;
pub mod battlelog;
pub mod battlefield_grpc;

use std::collections::HashMap;
use futures::stream;
use influxdb2::models::{DataPoint, data_point::DataPointError};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionAmounts {
    #[serde(rename = "serverAmount")]
    pub server_amount: i64,
    #[serde(rename = "soldierAmount")]
    pub soldier_amount: i64,
    #[serde(rename = "queueAmount")]
    pub queue_amount: i64,
    #[serde(rename = "spectatorAmount")]
    pub spectator_amount: i64,
    #[serde(rename = "diceServerAmount")]
    pub dice_server_amount: i64,
    #[serde(rename = "diceSoldierAmount")]
    pub dice_soldier_amount: i64,
    #[serde(rename = "diceQueueAmount")]
    pub dice_queue_amount: i64,
    #[serde(rename = "diceSpectatorAmount")]
    pub dice_spectator_amount: i64,
    #[serde(rename = "communityServerAmount")]
    pub community_server_amount: i64,
    #[serde(rename = "communitySoldierAmount")]
    pub community_soldier_amount: i64,
    #[serde(rename = "communityQueueAmount")]
    pub community_queue_amount: i64,
    #[serde(rename = "communitySpectatorAmount")]
    pub community_spectator_amount: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionResult {
    pub region: String,
    pub amounts: RegionAmounts,
    pub maps: HashMap<String, i64>,
    pub modes: HashMap<String, i64>
}

pub fn build_data_point(frontend_game_name: &str, data_type: &str, region: &str, platform: &str, field: &str, amount: &i64) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(frontend_game_name)
        .tag("type", data_type)
        .tag("region", region)
        .tag("platform", platform)
        .field(field, *amount)
        .build()
}

pub async fn push_to_database(influx_client: &influxdb2::Client, frontend_game_name: &str, platform: &str, platform_result: &HashMap<String, RegionResult>) -> anyhow::Result<()> {
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
        influx_client.write(bucket, stream::iter(points)).await?;
    }
    Ok(())
}