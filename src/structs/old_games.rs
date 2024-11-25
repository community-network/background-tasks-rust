use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServer {
    #[serde(rename = "serverIp")]
    pub server_ip: Option<String>, // bf2 bf1942 bf2142 bfvietnam
    #[serde(rename = "serverPort")]
    pub server_port: Option<String>, // bf1942 bf2142 bfvietnam
    pub hostport: Option<String>, // bf2
    pub numplayers: String,
    pub hostname: Option<String>, // bf2 bf1942 bf2142 bfvietnam
    pub mapname: Option<String>,  // bf1942 bf2 bf2142 bfvietnam
    pub gametype: Option<String>, // bf2
    #[serde(rename = "I")]
    pub bfbc2_ip: Option<String>, // bfbc2
    #[serde(rename = "P")]
    pub bfbc2_port: Option<String>, // bfbc2
    #[serde(rename = "N")]
    pub bfbc2_name: Option<String>, // bfbc2
    #[serde(rename = "B-U-level")]
    pub bfbc2_map: Option<String>, // bfbc2
    #[serde(rename = "B-U-gamemode")]
    pub bfbc2_mode: Option<String>, // bfbc2
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServerList {
    #[serde(rename = "serverList")]
    pub server_list: Vec<OldGameServer>,
}
