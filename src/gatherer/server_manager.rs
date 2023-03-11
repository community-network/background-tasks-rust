use crate::connectors::mongo::ManagerInfo;
use futures::stream;
use influxdb2::models::{data_point::DataPointError, DataPoint};

pub fn build_data_point(field: &str, amount: i64) -> Result<DataPoint, DataPointError> {
    DataPoint::builder("serverManager")
        .tag("type", "amounts")
        .tag("region", "all")
        .tag("platform", "pc")
        .field(field, amount)
        .build()
}

pub async fn save_server_manager_info(
    influx_client: &influxdb2::Client,
    manager_info: ManagerInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let bucket = "Game info";
    let points = vec![
        build_data_point("communityGroups", manager_info.groups_count)?,
        build_data_point("communityServers", manager_info.server_count)?,
        build_data_point("playerList", manager_info.player_count)?,
        build_data_point("autoKickPingAmount", manager_info.auto_ping_kick_count)?,
        build_data_point("bfbanAmount", manager_info.bfban_count)?,
        build_data_point("moveAmount", manager_info.move_count)?,
        build_data_point("kickAmount", manager_info.kick_count)?,
        build_data_point("banAmount", manager_info.ban_count)?,
        build_data_point("globalBanKickAmount", manager_info.global_ban_kick_count)?,
    ];

    influx_client.write(bucket, stream::iter(points)).await?;
    Ok(())
}
