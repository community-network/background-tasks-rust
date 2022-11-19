use crate::structs::old_games;
use std::collections::HashMap;
use bf_sparta::cookie::Cookie;
use mongodb::{Collection, Client, Database, options::ReplaceOptions, results::UpdateResult};
use serde::{Deserialize, Serialize};
use mongodb::error::Result;
use bson::Document;
use chrono::{DateTime, Utc};
use crate::structs::results;
use mongodb::results::InsertOneResult;

pub struct MongoClient {
    pub backend_cookies: Collection<BackendCookie>,
    pub community_servers:  Collection<Document>,
    pub community_groups: Collection<Document>,
    pub player_list: Collection<Document>,
    pub logging: Collection<Document>,
    pub old_games_servers: Collection<old_games::OldGameServerList>,
    pub graphing_db: Database,
    pub client: Client,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackendCookie {
    pub _id: String,
    pub sid: String,
    pub remid: String
}

impl From<BackendCookie> for Cookie {
    fn from(cookie: BackendCookie) -> Self {
        Cookie {
            remid: cookie.remid,
            sid: cookie.sid,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagerInfo {
    #[serde(rename = "communityGroups")]
    pub groups_count: i64,
    #[serde(rename = "communityServers")]
    pub server_count: i64,
    #[serde(rename = "playerList")]
    pub player_count: i64,
    #[serde(rename = "autoKickPingAmount")]
    pub auto_ping_kick_count: i64,
    #[serde(rename = "bfbanAmount")]
    pub bfban_count: i64,
    #[serde(rename = "moveAmount")]
    pub move_count: i64,
    #[serde(rename = "kickAmount")]
    pub kick_count: i64,
    #[serde(rename = "banAmount")]
    pub ban_count: i64,
    #[serde(rename = "globalBanKickAmount")]
    pub global_ban_kick_count: i64,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub timestamp: DateTime<Utc>,
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
            backend_cookies: db.collection("backendCookies"),
            community_servers: db.collection("communityServers"),
            community_groups: db.collection("communityGroups"),
            player_list: db.collection("playerList"),
            logging: db.collection("logging"),
            old_games_servers: gamestats_db.collection("oldGamesServerList"),
            graphing_db: client.database("graphing"),
            client,
        })

    }

    pub async fn gather_managerinfo(&mut self) -> Result<InsertOneResult> {
        let result = &ManagerInfo { 
            groups_count: self.community_groups.count_documents(None, None).await.unwrap_or(0) as i64,
            server_count: self.community_servers.count_documents(None, None).await.unwrap_or(0) as i64,
            player_count: self.player_list.count_documents(None, None).await.unwrap_or(0) as i64,
            auto_ping_kick_count: self.logging.count_documents(bson::doc! {"action": "autokick-ping"}, None).await.unwrap_or(0) as i64,
            bfban_count: self.logging.count_documents(bson::doc! {"action": "autokick-bfban"}, None).await.unwrap_or(0) as i64,
            move_count: self.logging.count_documents(bson::doc! {"action": "movePlayer"}, None).await.unwrap_or(0) as i64,
            kick_count: self.logging.count_documents(bson::doc! {"action": "kickPlayerg"}, None).await.unwrap_or(0) as i64,
            ban_count: self.logging.count_documents(bson::doc! {"action": "addServerBan"}, None).await.unwrap_or(0) as i64,
            global_ban_kick_count: self.logging.count_documents(bson::doc! {"action": "autokick-globalBans"}, None).await.unwrap_or(0) as i64,
            timestamp: Utc::now(),
        };
        
        let collection: Collection<ManagerInfo> = self.graphing_db.collection("manager_info");
        collection.insert_one(result, None).await
    }

    pub async fn push_new_cookies(&mut self, acc_email: &str, cookie: &Cookie) -> Result<UpdateResult> {
        let id = acc_email.split("@").collect::<Vec<&str>>()[0];
        let cookie = BackendCookie {
            _id: format!("main-{}", id),
            sid: cookie.sid.clone(),
            remid: cookie.remid.clone(),
        };
        let options = ReplaceOptions::builder().upsert(true).build();
        self.backend_cookies.replace_one(
            bson::doc! {"_id": format!("main-{}", id)},
            cookie,
            options).await
    }

    pub async fn get_cookies(&mut self, acc_email: &str) -> anyhow::Result<Cookie> {
        let backend_cookie = match self.backend_cookies.find_one(bson::doc! {"_id": format!("main-{}", acc_email.split("@").collect::<Vec<&str>>()[0])}, None).await? {
            Some(result) => result,
            None => anyhow::bail!("no cookie"),
        };
        Ok(backend_cookie.into())
    }

    pub async fn push_to_database(&mut self, frontend_game_name: &str, platform_result: &HashMap<String, results::RegionResult>) -> anyhow::Result<()> {
        let collection: Collection<results::RegionResult> = self.graphing_db.collection(frontend_game_name);
        for (key, value) in platform_result {
            match collection.insert_one(value, None).await {
                Ok(_) => {},
                Err(e) => {log::error!("Failed to push {} for {} to mongodb: {:#?}", key, frontend_game_name, e);}
            };
        }
        Ok(())
    }

    pub async fn push_totals(&mut self, global_result: results::RegionResult) -> anyhow::Result<()> {
        
        Ok(())
    }
    
    pub async fn push_old_games(&mut self, frontend_game_name: &str, game_result: results::OldGameResult) -> anyhow::Result<()> {
        let collection: Collection<results::OldGameResult> = self.graphing_db.collection(frontend_game_name);
        match collection.insert_one(game_result, None).await {
            Ok(_) => {},
            Err(e) => {log::error!("Failed to push {} to mongodb: {:#?}", frontend_game_name, e);}
        };
        Ok(())
    }

    pub async fn gather_old_title(&mut self, game_name: &str) -> Result<Option<old_games::OldGameServerList>> {
        self.old_games_servers.find_one(bson::doc! {"_id": game_name}, None).await
    }
}
