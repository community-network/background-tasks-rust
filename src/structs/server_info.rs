use super::battlelog;

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub guid: String,
    pub game_id: String,
    pub soldiers: i64,
    pub queue: i64,
    pub mode: String,
    pub map: String,
    pub is_official: Option<bool>,
}

impl From<battlelog::BattlelogServer> for ServerInfo {
    fn from(server: battlelog::BattlelogServer) -> Self {
        ServerInfo {
            game_id: server.game_id.clone(),
            guid: server.guid.clone(),
            name: server.name.clone(),
            soldiers: server.soldier_amount,
            queue: server.queue_amount,
            mode: "".to_owned(),
            map: server.map,
            is_official: None,
        }
    }
}
