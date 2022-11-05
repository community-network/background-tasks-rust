use crate::structs::old_games;

use mongodb::{Collection, Client};
use mongodb::error::Result;
use bson::Document;

pub struct MongoClient {
    pub community_servers:  Collection<Document>,
    pub community_groups: Collection<Document>,
    pub player_list: Collection<Document>,
    pub logging: Collection<Document>,
    pub old_games_servers: Collection<old_games::OldGameServerList>,
    pub client: Client,
}

pub struct ManagerInfo {
    pub groups_count: u64,
    pub server_count: u64,
    pub player_count: u64,
    pub auto_ping_kick_count: u64,
    pub bfban_count: u64,
    pub move_count: u64,
    pub kick_count: u64,
    pub ban_count: u64,
    pub global_ban_kick_count: u64
}

impl MongoClient {
    pub async fn connect() -> Result<Self> {

        // Possible env
        const MONGO_URL: &str = "mongodb://zjobse:x4YfuHA2jYWAnj3d@194.242.57.80:27017,194.242.57.81:27017,194.242.57.82:27017/serverManager?replicaSet=replica01&authSource=admin";
        // Try connect to mongo client
        let client = Client::with_uri_str(MONGO_URL).await?;

        // Server manager DB
        let db = client.database("serverManager");
        let gamestats_db = client.database("gamestats");
        
        Ok(MongoClient {
            community_servers: db.collection("communityServers"),
            community_groups: db.collection("communityGroups"),
            player_list: db.collection("playerList"),
            logging: db.collection("logging"),
            old_games_servers: gamestats_db.collection("oldGamesServerList"),
            client,
        })

    }

    pub async fn gather_managerinfo(&mut self) -> Result<ManagerInfo> {
        Ok(ManagerInfo { 
            groups_count: self.community_groups.count_documents(None, None).await.unwrap_or(0),
            server_count: self.community_servers.count_documents(None, None).await.unwrap_or(0),
            player_count: self.player_list.count_documents(None, None).await.unwrap_or(0),
            auto_ping_kick_count: self.logging.count_documents(bson::doc! {"action": "autokick-ping"}, None).await.unwrap_or(0),
            bfban_count: self.logging.count_documents(bson::doc! {"action": "autokick-bfban"}, None).await.unwrap_or(0),
            move_count: self.logging.count_documents(bson::doc! {"action": "movePlayer"}, None).await.unwrap_or(0),
            kick_count: self.logging.count_documents(bson::doc! {"action": "kickPlayerg"}, None).await.unwrap_or(0),
            ban_count: self.logging.count_documents(bson::doc! {"action": "addServerBan"}, None).await.unwrap_or(0),
            global_ban_kick_count: self.logging.count_documents(bson::doc! {"action": "autokick-globalBans"}, None).await.unwrap_or(0)
        })
    }

    pub async fn gather_old_title(&mut self, game_name: &str) -> Result<Option<old_games::OldGameServerList>> {
        self.old_games_servers.find_one(bson::doc! {"_id": game_name}, None).await
    }
}
