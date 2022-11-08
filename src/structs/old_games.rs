use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServer {
    pub numplayers: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OldGameServerList {
    #[serde(rename = "serverList")]
    pub server_list: Vec<OldGameServer>
}