use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub name: String,
    pub team: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mod {
    pub category: String,
    pub file_name: String,
    pub link: String,
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ModType {
    Vec(Vec<Mod>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PlayerType {
    Vec(Vec<Player>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarneServerInfo {
    pub id: i64,
    pub name: String,
    #[serde(rename = "mapName")]
    pub map_name: String,
    #[serde(rename = "gameMode")]
    pub game_mode: String,
    #[serde(rename = "maxPlayers")]
    pub max_players: i64,
    #[serde(rename = "natType")]
    pub nat_type: i64,
    #[serde(rename = "tickRate")]
    pub tick_rate: i64,
    pub password: i64,
    #[serde(rename = "needSameMods")]
    pub need_same_mods: i64,
    #[serde(rename = "allowMoreMods")]
    pub allow_more_mods: i64,
    #[serde(rename = "isModded")]
    pub is_modded: bool,
    #[serde(rename = "currentPlayers")]
    pub current_players: i64,
    #[serde(rename = "currentSpectators")]
    pub current_spectators: i64,
    pub region: String,
    pub country: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarneServerList {
    pub servers: Vec<MarneServerInfo>,
}
