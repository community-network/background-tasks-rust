use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServer {
    pub numplayers: u64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServerList {
    #[serde(rename = "serverList")]
    pub server_list: Vec<OldGameServer>
}