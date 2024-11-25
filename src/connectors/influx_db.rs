use futures::stream;
use influxdb2::models::{data_point::DataPointError, DataPoint};
use std::collections::HashMap;

use crate::structs::results;

pub fn build_data_point(
    frontend_game_name: &str,
    data_type: &str,
    region: &str,
    platform: &str,
    field: &str,
    amount: &i64,
) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(frontend_game_name)
        .tag("platform", platform)
        .tag("region", region)
        .tag("type", data_type)
        .field(field, *amount)
        .build()
}

pub fn build_playground_data_point(
    frontend_game_name: &str,
    data_type: &str,
    region: &str,
    platform: &str,
    playground_name: &str,
    amount: &i64,
) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(frontend_game_name)
        .tag("platform", platform)
        .tag("region", region)
        .tag("type", data_type)
        .field("playground", playground_name)
        .field("count", *amount)
        .build()
}

pub async fn push_to_database(
    influx_client: &influxdb2::Client,
    frontend_game_name: &str,
    platform: &str,
    platform_result: &HashMap<String, results::RegionResult>,
) -> anyhow::Result<()> {
    let bucket = "Game info";
    for (region, region_result) in platform_result {
        let mut points = vec![
            build_data_point(
                frontend_game_name,
                "amounts",
                region,
                platform,
                "serverAmount",
                &region_result.amounts.server_amount,
            )?,
            build_data_point(
                frontend_game_name,
                "amounts",
                region,
                platform,
                "soldierAmount",
                &region_result.amounts.soldier_amount,
            )?,
            build_data_point(
                frontend_game_name,
                "amounts",
                region,
                platform,
                "queueAmount",
                &region_result.amounts.queue_amount,
            )?,
        ];
        if vec!["bf1", "bfv", "bf4", "battlebit"].contains(&frontend_game_name) {
            points.append(&mut vec![
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "spectatorAmount",
                    &region_result.amounts.spectator_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "diceServerAmount",
                    &region_result.amounts.dice_server_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "diceSoldierAmount",
                    &region_result.amounts.dice_soldier_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "diceQueueAmount",
                    &region_result.amounts.dice_queue_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "diceSpectatorAmount",
                    &region_result.amounts.dice_spectator_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "communityServerAmount",
                    &region_result.amounts.community_server_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "communitySoldierAmount",
                    &region_result.amounts.community_soldier_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "communityQueueAmount",
                    &region_result.amounts.community_queue_amount,
                )?,
                build_data_point(
                    frontend_game_name,
                    "amounts",
                    region,
                    platform,
                    "communitySpectatorAmount",
                    &region_result.amounts.community_spectator_amount,
                )?,
            ]);
        }

        for (key, value) in &region_result.maps {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "maps",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }
        for (key, value) in &region_result.map_players {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "mapPlayers",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }

        for (key, value) in &region_result.modes {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "modes",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }
        for (key, value) in &region_result.mode_players {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "modePlayers",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }

        for (key, value) in &region_result.owner_platform {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "ownerPlatform",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }

        for (key, value) in &region_result.settings {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "settings",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }
        for (key, value) in &region_result.settings_players {
            if !key.is_empty() {
                points.push(build_data_point(
                    frontend_game_name,
                    "settingPlayers",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }

        for (key, value) in &region_result.playground {
            if !key.is_empty() {
                points.push(build_playground_data_point(
                    frontend_game_name,
                    "playground",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }
        for (key, value) in &region_result.playground_players {
            if !key.is_empty() {
                points.push(build_playground_data_point(
                    frontend_game_name,
                    "playgroundPlayers",
                    region,
                    platform,
                    key,
                    value,
                )?);
            }
        }
        influx_client
            .write_with_precision(
                bucket,
                stream::iter(points),
                influxdb2::api::write::TimestampPrecision::Seconds,
            )
            .await?;
    }
    Ok(())
}

pub async fn push_totals(
    influx_client: &influxdb2::Client,
    global_result: &results::RegionResult,
) -> anyhow::Result<()> {
    let bucket = "Game info";
    let points = vec![
        build_data_point(
            "global",
            "amounts",
            "ALL",
            "global",
            "serverAmount",
            &global_result.amounts.server_amount,
        )?,
        build_data_point(
            "global",
            "amounts",
            "ALL",
            "global",
            "soldierAmount",
            &global_result.amounts.soldier_amount,
        )?,
        build_data_point(
            "global",
            "amounts",
            "ALL",
            "global",
            "queueAmount",
            &global_result.amounts.queue_amount,
        )?,
    ];
    influx_client
        .write_with_precision(
            bucket,
            stream::iter(points),
            influxdb2::api::write::TimestampPrecision::Seconds,
        )
        .await?;
    Ok(())
}

// pub fn build_server_data_point(
//     frontend_game_name: &str,
//     data_type: &str,
//     server_info: &server_info::ServerInfo,
//     region: &str,
//     platform: &str,
//     field: &str,
//     amount: &i64,
// ) -> Result<DataPoint, DataPointError> {
//     let mut data_point = DataPoint::builder(frontend_game_name);

//     data_point = match !server_info.guid.is_empty() {
//         true => data_point.field("guid", server_info.guid.clone()),
//         false => data_point,
//     };
//     data_point = match !server_info.game_id.is_empty() {
//         true => data_point.field("gameId", server_info.game_id.clone()),
//         false => data_point,
//     };
//     data_point = match server_info.is_official {
//         Some(result) => data_point.tag("isOfficial", result.to_string()),
//         None => data_point,
//     };

//     data_point
//         .tag("platform", platform)
//         .tag("region", region)
//         .tag("type", data_type)
//         .field("serverName", server_info.name.clone())
//         .field(field, *amount)
//         .build()
// }

// pub async fn push_server(
//     frontend_game_name: &str,
//     influx_client: &influxdb2::Client,
//     region: &str,
//     platform: &str,
//     server_infos: Vec<server_info::ServerInfo>,
// ) -> anyhow::Result<()> {
//     let bucket = "Battlefield servers";
//     let mut points = vec![];
//     for server_info in server_infos {
//         if !server_info.name.is_empty() {
//             points.extend(vec![
//                 build_server_data_point(
//                     frontend_game_name,
//                     "amounts",
//                     &server_info,
//                     region,
//                     platform,
//                     "soldierAmount",
//                     &server_info.soldiers,
//                 )?,
//                 build_server_data_point(
//                     frontend_game_name,
//                     "amounts",
//                     &server_info,
//                     region,
//                     platform,
//                     "queueAmount",
//                     &server_info.queue,
//                 )?,
//             ]);
//             if !server_info.mode.is_empty() {
//                 points.push(build_server_data_point(
//                     frontend_game_name,
//                     "mode",
//                     &server_info,
//                     region,
//                     platform,
//                     &server_info.mode,
//                     &1,
//                 )?);
//             }
//             if !server_info.map.is_empty() {
//                 points.push(build_server_data_point(
//                     frontend_game_name,
//                     "map",
//                     &server_info,
//                     region,
//                     platform,
//                     &server_info.map,
//                     &1,
//                 )?);
//             }
//         }
//     }
//     influx_client
//         .write_with_precision(
//             bucket,
//             stream::iter(points),
//             influxdb2::api::write::TimestampPrecision::Seconds,
//         )
//         .await?;
//     Ok(())
// }
