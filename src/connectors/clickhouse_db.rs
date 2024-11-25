// use crate::structs::server_info;
// use clickhouse::{Client, Row};
// use serde::Serialize;
// use time::OffsetDateTime;

// #[derive(Row, Serialize)]
// struct GameServer {
//     #[serde(with = "clickhouse::serde::time::datetime")]
//     timestamp: OffsetDateTime,
//     soldier_amount: u32,
//     queue_amount: u32,
//     game: String,
//     guid: Option<String>,
//     game_id: Option<String>,
//     server_name: String,
//     platform: String,
//     region: String,
//     mode: Option<String>,
//     map: Option<String>,
//     is_official: Option<bool>,
// }

// pub async fn push_server(
//     frontend_game_name: &str,
//     region: &str,
//     platform: &str,
//     server_infos: Vec<server_info::ServerInfo>,
// ) -> anyhow::Result<()> {
//     let client = Client::default().with_url("");
//     let mut insert = client.insert("game_servers")?;
//     for server_info in server_infos {
//         if !server_info.name.is_empty() {
//             insert
//                 .write(&GameServer {
//                     timestamp: OffsetDateTime::now_utc(),
//                     soldier_amount: server_info.soldiers as u32,
//                     queue_amount: server_info.queue as u32,
//                     game: frontend_game_name.to_owned(),
//                     guid: match !server_info.guid.is_empty() {
//                         true => Some(server_info.guid),
//                         false => None,
//                     },
//                     game_id: match !server_info.game_id.is_empty() {
//                         true => Some(server_info.game_id),
//                         false => None,
//                     },
//                     server_name: server_info.name,
//                     platform: platform.to_owned(),
//                     region: region.to_owned(),
//                     mode: match !server_info.mode.is_empty() {
//                         true => Some(server_info.mode),
//                         false => None,
//                     },
//                     map: match !server_info.map.is_empty() {
//                         true => Some(server_info.map),
//                         false => None,
//                     },
//                     is_official: server_info.is_official,
//                 })
//                 .await?;
//         }
//     }
//     insert.end().await?;
//     Ok(())
// }
