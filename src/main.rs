mod mongo;
mod gatherer;
mod structs;

use gatherer::{server_manager, old_games, companion, battlelog, battlefield_grpc};
use std::collections::HashMap;

use influxdb2::Client;
use bf_sparta::cookie_request;
use mongo::MongoClient;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    flexi_logger::Logger::try_with_str("info")?.start()?;
    
    let influx_client = Client::new("https://europe-west1-1.gcp.cloud2.influxdata.com", "Gametools network", "uWe8oo4ykDMatlYX2g_mJWt3jitcxIOaJU9rNaJUZGQuLPmi0KL_eIS8QqHq9EEjLkNTOoRdnZMFdARuzOIigw==");
    let mut mongo_client = MongoClient::connect().await?;

    // https://github.com/aprimadi/influxdb2
    let cookie_auth = cookie_request::request_cookie(cookie_request::Login {
        email: "api4@gametools.network".to_string(),
        pass: "bqgPAHJDphaTpPcbRk8jfHhbCecnTnRN".to_string(),
    })
    .await?;
    let cookie = &bf_sparta::cookie::Cookie {
        sid: cookie_auth.sid,
        remid: cookie_auth.remid,
    };

    let empty_game_hash: HashMap<String, String> = HashMap::new();
    let mut sessions: HashMap<String, HashMap<String, String>> = HashMap::new();

    loop {
        match server_manager::save_server_manager_info(&influx_client, &mut mongo_client).await {
            Ok(_) => {},
            Err(e) => log::error!("Failed to send new manager info {:#?}", e),
        };

        let old_games =  HashMap::from([
            ("bf2-playbf2", "playbf2"),
            ("bf2-bf2hub", "bf2hub"),
            ("bfield1942-bf1942org", "bfield1942"),
            ("bf2142-openspy", "bf2142"),
            ("bf2142-play2142", "play2142"),
            ("bfbc2", "bfbc2"),
            ("bfvietnam-qtracker", "bfvietnam"),
            ("bfvietnam-openspy", "openspy")
        ]);
        for (key, value) in old_games.into_iter() {
            match old_games::push_old_games(&influx_client, &mut mongo_client, key, value).await {
                Ok(_) => {},
                Err(e) => log::error!("Failed oldgame: {}, with reason: {:#?}", key, e),
            };
        }

        let sparta_games = HashMap::from([
            ("tunguska", "bf1"),
            ("casablanca", "bfv"),
            ("bf4", "bf4")
        ]);
        for (key, value) in sparta_games.into_iter() {
            let platform_result = match companion::gather_companion(&influx_client, sessions.get(key).unwrap_or(&empty_game_hash).to_owned(), cookie.clone(), key, value).await {
                Ok((session, platform_result)) => {
                    sessions.insert(key.to_string(), session);
                    Some(platform_result)
                },
                Err(e) => {
                    log::error!("Failed sparta_game: {}, with reason: {:#?}", key, e);
                    None
                },
            };
        }
        // pc only!
        let battlelog_games = HashMap::from([
            ("bf3", "https://battlelog.battlefield.com/bf3/servers/getAutoBrowseServers/"),
            ("bf4", "https://battlelog.battlefield.com/bf4/servers/getServers/pc/"),
            ("bfh", "https://battlelog.battlefield.com/bfh/servers/getServers/pc/")
        ]);
        for (key, value) in battlelog_games {
            let game_result = match battlelog::gather_battlelog(&influx_client, key, value).await {
                Ok(game_result) => Some(game_result),
                Err(e) => {
                    log::error!("Failed battlelog_game: {}, with reason: {:#?}", key, e);
                    None
                },
            };
        }
        let grpc_result = match battlefield_grpc::gather_grpc(&influx_client, sessions.get("kingston").unwrap_or(&empty_game_hash).to_owned(), cookie.clone()).await {
            Ok(grpc_result) => Some(grpc_result),
            Err(e) => {
                log::error!("Failed kingston_grpc, with reason: {:#?}", e);
                None
            },
        };
        
    }
}
