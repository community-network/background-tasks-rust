use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattlelogServer {
    pub name: String,
    pub guid: String,
    pub game_id: String,
    pub ip: String,
    pub region: String,
    #[serde(rename = "queueAmount")]
    pub queue_amount: i64,
    #[serde(rename = "soldierAmount")]
    pub soldier_amount: i64,
    pub map: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInfo {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamInfo {
    pub players: HashMap<String, PlayerInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Snapshot {
    #[serde(rename = "teamInfo")]
    pub team_info: HashMap<String, TeamInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keeper {
    pub snapshot: Snapshot,
}
