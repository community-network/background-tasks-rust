use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattlebitServer {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Map")]
    pub map: String,
    #[serde(rename = "MapSize")]
    pub map_size: String,
    #[serde(rename = "Gamemode")]
    pub gamemode: String,
    #[serde(rename = "Region")]
    pub region: String,
    #[serde(rename = "Players")]
    pub players: i64,
    #[serde(rename = "QueuePlayers")]
    pub queue_players: i64,
    #[serde(rename = "MaxPlayers")]
    pub max_players: i64,
    #[serde(rename = "Hz")]
    pub hz: i64,
    #[serde(rename = "DayNight")]
    pub day_night: String,
    #[serde(rename = "IsOfficial")]
    pub is_official: bool,
    #[serde(rename = "HasPassword")]
    pub has_password: bool,
    #[serde(rename = "AntiCheat")]
    pub anti_cheat: String,
    #[serde(rename = "Build")]
    pub build: String,
}
