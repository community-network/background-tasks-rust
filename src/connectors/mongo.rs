use std::env;

use crate::structs::old_games;
use bf_sparta::cookie::Cookie;
use bf_sparta::sparta_api;
use bson::Document;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use mongodb::error::Result;
use mongodb::options::FindOptions;
use mongodb::{options::ReplaceOptions, results::UpdateResult, Client, Collection};
use serde::{Deserialize, Serialize};

pub struct MongoClient {
    pub backend_cookies: Collection<BackendCookie>,
    pub community_cookies: Collection<CommunityCookie>,
    pub cookie_check: Collection<CookieCheck>,
    pub community_servers: Collection<Document>,
    pub community_groups: Collection<Document>,
    pub player_list: Collection<Document>,
    pub logging: Collection<Document>,
    pub old_games_servers: Collection<old_games::OldGameServerList>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CookieCheck {
    pub _id: String,
    pub prefix: String,
    #[serde(rename = "personaId")]
    pub persona_id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunityCookie {
    pub _id: String,
    pub sid: String,
    pub remid: String,
    #[serde(rename = "personaId")]
    pub persona_id: String,
    pub username: String,
    #[serde(rename = "supportedGames")]
    pub supported_games: Vec<String>,
}

impl From<CommunityCookie> for Cookie {
    fn from(cookie: CommunityCookie) -> Self {
        Cookie {
            remid: cookie.remid,
            sid: cookie.sid,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackendCookie {
    pub _id: String,
    pub sid: String,
    pub remid: String,
    pub ea_access_token: Option<String>,
    pub valid: Option<bool>,
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
        // Try connect to mongo client
        let client = Client::with_uri_str(
            env::var("MONGO_DETAILS_STRING").expect("MONGO_DETAILS_STRING wasn't set"),
        )
        .await?;

        // Server manager DB
        let db = client.database("serverManager");
        let gamestats_db = client.database("gamestats");

        Ok(MongoClient {
            cookie_check: db.collection("cookieCheck"),
            community_cookies: db.collection("communityCookies"),
            backend_cookies: db.collection("backendCookies"),
            community_servers: db.collection("communityServers"),
            community_groups: db.collection("communityGroups"),
            player_list: db.collection("playerList"),
            logging: db.collection("logging"),
            old_games_servers: gamestats_db.collection("oldGamesServerList"),
        })
    }

    pub async fn gather_managerinfo(&mut self) -> Result<ManagerInfo> {
        let result = ManagerInfo {
            groups_count: self
                .community_groups
                .count_documents(bson::doc! {})
                .await
                .unwrap_or(0) as i64,
            server_count: self
                .community_servers
                .count_documents(bson::doc! {})
                .await
                .unwrap_or(0) as i64,
            player_count: self
                .player_list
                .count_documents(bson::doc! {})
                .await
                .unwrap_or(0) as i64,
            auto_ping_kick_count: self
                .logging
                .count_documents(bson::doc! {"action": "autokick-ping"})
                .await
                .unwrap_or(0) as i64,
            bfban_count: self
                .logging
                .count_documents(bson::doc! {"action": "autokick-bfban"})
                .await
                .unwrap_or(0) as i64,
            move_count: self
                .logging
                .count_documents(bson::doc! {"action": "movePlayer"})
                .await
                .unwrap_or(0) as i64,
            kick_count: self
                .logging
                .count_documents(bson::doc! {"action": "kickPlayerg"})
                .await
                .unwrap_or(0) as i64,
            ban_count: self
                .logging
                .count_documents(bson::doc! {"action": "addServerBan"})
                .await
                .unwrap_or(0) as i64,
            global_ban_kick_count: self
                .logging
                .count_documents(bson::doc! {"action": "autokick-globalBans"})
                .await
                .unwrap_or(0) as i64,
            timestamp: Utc::now(),
        };

        // let collection: Collection<ManagerInfo> = self.graphing_db.collection("manager_info");
        // collection.insert_one(&result, None).await?;
        Ok(result)
    }

    pub async fn push_new_cookies(
        &mut self,
        acc_email: &str,
        cookie: &Cookie,
        ea_access_token: String,
    ) -> Result<UpdateResult> {
        let id = acc_email.split('@').collect::<Vec<&str>>()[0];
        let cookie = BackendCookie {
            _id: format!("main-{}", id),
            sid: cookie.sid.clone(),
            remid: cookie.remid.clone(),
            ea_access_token: Some(ea_access_token.clone()),
            valid: Some(true),
        };
        let options = ReplaceOptions::builder().upsert(true).build();
        self.backend_cookies
            .replace_one(bson::doc! {"_id": format!("main-{}", id)}, cookie)
            .with_options(options)
            .await
    }

    pub async fn push_new_id_cookies(
        &mut self,
        id: &str,
        cookie: &Cookie,
        ea_access_token: String,
        valid: bool,
    ) -> Result<UpdateResult> {
        let cookie = BackendCookie {
            _id: id.to_string(),
            sid: cookie.sid.clone(),
            remid: cookie.remid.clone(),
            ea_access_token: Some(ea_access_token.clone()),
            valid: Some(valid),
        };
        let options = ReplaceOptions::builder().upsert(true).build();
        self.backend_cookies
            .replace_one(bson::doc! {"_id": id}, cookie)
            .with_options(options)
            .await
    }

    pub async fn get_cookies(&mut self, acc_email: &str) -> anyhow::Result<(Cookie, String)> {
        let backend_cookie = match self.backend_cookies.find_one(bson::doc! {"_id": format!("main-{}", acc_email.split('@').collect::<Vec<&str>>()[0])}).await? {
            Some(result) => result,
            None => anyhow::bail!("no cookie"),
        };
        Ok((
            backend_cookie.clone().into(),
            backend_cookie.ea_access_token.unwrap_or_default(),
        ))
    }

    pub async fn get_cookies_by_id(&mut self, id: &str) -> anyhow::Result<(Cookie, String, bool)> {
        let backend_cookie = match self
            .backend_cookies
            .find_one(bson::doc! {"_id": id})
            .await?
        {
            Some(result) => result,
            None => anyhow::bail!("no cookie"),
        };
        Ok((
            backend_cookie.clone().into(),
            backend_cookie.ea_access_token.unwrap_or_default(),
            backend_cookie.valid.unwrap_or_default(),
        ))
    }

    pub async fn get_random_cookie(&mut self) -> anyhow::Result<Cookie> {
        let mut cookie_check = self
            .cookie_check
            .find(bson::doc! {})
            .with_options(
                FindOptions::builder()
                    .sort(bson::doc! {"timeStamp": -1})
                    .build(),
            )
            .await?;
        let mut result_cookie = None;
        while let Some(maybe_check) = cookie_check.next().await {
            let check = maybe_check?;
            match self
                .community_cookies
                .find_one(bson::doc! {"_id": check._id})
                .await
            {
                Ok(e) => {
                    if let Some(cookie) = e {
                        match sparta_api::get_token(
                            cookie.clone().into(),
                            "pc",
                            "tunguska",
                            "en-us",
                        )
                        .await
                        {
                            Ok(_) => {
                                result_cookie = Some(cookie);
                                break;
                            }
                            Err(e) => {
                                log::info!("Cookie not valid, trying another one - {}", e);
                            }
                        }
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };
        }

        Ok(match result_cookie {
            Some(cookie) => cookie.into(),
            None => anyhow::bail!("no cookie found"),
        })
    }

    pub async fn gather_old_title(
        &mut self,
        game_name: &str,
    ) -> Result<Option<old_games::OldGameServerList>> {
        self.old_games_servers
            .find_one(bson::doc! {"_id": game_name})
            .await
    }
}
