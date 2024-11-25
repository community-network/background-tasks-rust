// use crate::structs::server_info;
// use questdb::ingress::{Buffer, Sender};

// let mut sender = SenderBuilder::new("", 9009).connect()?;

// pub fn push_server(
//     frontend_game_name: &str,
//     region: &str,
//     platform: &str,
//     server_infos: Vec<server_info::ServerInfo>,
// ) -> anyhow::Result<()> {
//     let mut quest_sender = Sender::from_conf("http::addr=:9009;")?;
//     for server_info in server_infos {
//         if !server_info.name.is_empty() {
//             let mut buffer = &mut Buffer::new();
//             buffer
//                 .table("Battlefield servers")?
//                 .symbol("game", frontend_game_name)?
//                 .symbol("region", region)?
//                 .symbol("platform", platform)?
//                 .column_str("serverName", server_info.name)?;

//             buffer = match !server_info.guid.is_empty() {
//                 true => buffer.column_str("guid", &server_info.guid)?,
//                 false => buffer,
//             };
//             buffer = match !server_info.game_id.is_empty() {
//                 true => buffer.column_str("gameId", &server_info.game_id)?,
//                 false => buffer,
//             };

//             buffer = match !server_info.mode.is_empty() {
//                 true => buffer.column_str("mode", server_info.mode)?,
//                 false => buffer,
//             };
//             buffer = match !server_info.map.is_empty() {
//                 true => buffer.column_str("map", server_info.map)?,
//                 false => buffer,
//             };
//             buffer = match server_info.is_official {
//                 Some(result) => buffer.column_bool("isOfficial", result)?,
//                 None => buffer,
//             };

//             buffer
//                 .column_i64("soldierAmount", server_info.soldiers)?
//                 .column_i64("queueAmount", server_info.queue)?
//                 .at_now()?;
//             quest_sender.flush(buffer)?;
//         }
//     }
//     Ok(())
// }

// pub fn push_totals(global_result: &results::RegionResult) -> anyhow::Result<()> {
//     let mut quest_sender = SenderBuilder::new("", 9009).connect()?;
//     let buffer = &mut Buffer::new();
//     buffer
//         .table("Game info")?
//         .symbol("game", "global")?
//         .symbol("region", "ALL")?
//         .symbol("platform", "global")?
//         .column_i64("serverAmount", global_result.amounts.server_amount)?
//         .column_i64("soldierAmount", global_result.amounts.soldier_amount)?
//         .column_i64("queueAmount", global_result.amounts.queue_amount)?
//         .at_now()?;
//     quest_sender.flush(buffer)?;
//     Ok(())
// }

// pub fn push_to_database(frontend_game_name: &str,
//     platform: &str,
//     platform_result: &HashMap<String, results::RegionResult>
// ) -> anyhow::Result<()> {
//     let mut quest_sender = SenderBuilder::new("", 9009).connect()?;
//     for (region, region_result) in platform_result {
//         let buffer = &mut Buffer::new();
//         if vec!["bf1", "bfv", "bf4", "battlebit"].contains(&frontend_game_name) {
//             buffer
//                 .column_i64("spectatorAmount", region_result.amounts.spectator_amount)?
//                 .column_i64("diceServerAmount", region_result.amounts.dice_server_amount)?
//                 .column_i64("diceSoldierAmount", region_result.amounts.dice_soldier_amount)?
//                 .column_i64("diceQueueAmount", region_result.amounts.dice_queue_amount)?
//                 .column_i64("diceSpectatorAmount", region_result.amounts.dice_spectator_amount)?
//                 .column_i64("communityServerAmount", region_result.amounts.community_server_amount)?
//                 .column_i64("communitySoldierAmount", region_result.amounts.community_soldier_amount)?
//                 .column_i64("communityQueueAmount", region_result.amounts.community_queue_amount)?
//                 .column_i64("communitySpectatorAmount", region_result.amounts.community_spectator_amount)?;
//         }
//         buffer
//             .table("Game info")?
//             .symbol("game", frontend_game_name)?
//             .symbol("region", region)?
//             .symbol("platform", platform)?
//             .column_i64("serverAmount", region_result.amounts.server_amount)?
//             .column_i64("soldierAmount", region_result.amounts.soldier_amount)?
//             .column_i64("queueAmount", region_result.amounts.queue_amount)?
//             .at_now()?;

//         quest_sender.flush(buffer)?;

//         for (key, value) in &region_result.maps {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "maps",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }
//         for (key, value) in &region_result.map_players {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "mapPlayers",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }

//         for (key, value) in &region_result.modes {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "modes",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }
//         for (key, value) in &region_result.mode_players {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "modePlayers",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }

//         for (key, value) in &region_result.owner_platform {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "ownerPlatform",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }

//         for (key, value) in &region_result.settings {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "settings",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }
//         for (key, value) in &region_result.settings_players {
//             if !key.is_empty() {
//                 points.push(build_data_point(
//                     frontend_game_name,
//                     "settingPlayers",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }

//         for (key, value) in &region_result.playground {
//             if !key.is_empty() {
//                 points.push(build_playground_data_point(
//                     frontend_game_name,
//                     "playground",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }
//         for (key, value) in &region_result.playground_players {
//             if !key.is_empty() {
//                 points.push(build_playground_data_point(
//                     frontend_game_name,
//                     "playgroundPlayers",
//                     region,
//                     platform,
//                     key,
//                     value,
//                 )?);
//             }
//         }
//     }

//     Ok(())
// }
