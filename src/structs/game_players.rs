use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GamePlayerInfo {
    pub rank: u64,
    pub latency: u64,
    pub slot: u64,
    pub join_time: u64,
    pub localization: String,
    pub user_id: u64,
    pub player_id: u64,
    pub name: String,
    pub platform: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GamePlayers {
    pub players: std::collections::HashMap<u64, GamePlayerInfo>,
    pub team_1: Vec<u64>,
    pub team_2: Vec<u64>,
    pub spectators: Vec<u64>,
    pub loading: Vec<u64>,
    pub que: Vec<u64>,
    pub server_info: ServerInfo,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerInfo {
    pub server_name: String,
    pub admins: Vec<String>,
    pub country: String,
    pub description: String,
    pub experience: String,
    pub fairfight: String,
    pub level: String,
    pub mode: String,
    pub lowrankonly: String,
    pub maps: Vec<String>,
    pub owner: String,
    pub settings: Vec<String>,
    pub vips: Vec<String>,
    pub region: String,
    pub servertype: String,
}
