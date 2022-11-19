mod mongo;
mod gatherer;
mod structs;

use gatherer::{old_games, companion, battlelog, battlefield_grpc};
use structs::results;
use chrono::Utc;
use std::{collections::HashMap, sync::{atomic, Arc}};
use tokio::time::{sleep, Duration};
use bf_sparta::{cookie_request, sparta_api};
use mongo::MongoClient;
use warp::Filter;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let last_update = Arc::new(atomic::AtomicI64::new(Utc::now().timestamp() / 60));
    let last_update_clone = Arc::clone(&last_update);

    flexi_logger::Logger::try_with_str("info")?.start()?;
    log::info!("Starting...");
    
    tokio::spawn(async move {
        let hello = warp::any().map(move || {
            let last_update_i64 = last_update_clone.load(atomic::Ordering::Relaxed);
            let now_minutes = Utc::now().timestamp() / 60;

            // error if 10 minutes without updates
            if (now_minutes - last_update_i64) > 10 {
                return warp::reply::with_status(format!("{}", now_minutes - last_update_i64), warp::http::StatusCode::SERVICE_UNAVAILABLE);
            } else {
                return warp::reply::with_status(format!("{}", now_minutes - last_update_i64), warp::http::StatusCode::OK);
            }
        });
        warp::serve(hello).run(([0, 0, 0, 0], 3030)).await;
    });

    let mut mongo_client = MongoClient::connect().await?;

    let mut cookie = match mongo_client.get_cookies("api4@gametools.network").await {
        Ok(result) => result,
        Err(_) => {
            bf_sparta::cookie::Cookie {
                sid: "".to_string(),
                remid: "".to_string(),
            }
        },
    };

    cookie = match sparta_api::get_token(cookie.clone(), "pc", "tunguska", "en-us").await {
        Ok(_) => cookie.clone(),
        Err(_) => {
            let cookie_auth = cookie_request::request_cookie(cookie_request::Login {
                email: "api4@gametools.network".to_string(),
                pass: "bqgPAHJDphaTpPcbRk8jfHhbCecnTnRN".to_string(),
            })
            .await?;
            let cookie = bf_sparta::cookie::Cookie {
                sid: cookie_auth.sid,
                remid: cookie_auth.remid,
            };
            mongo_client.push_new_cookies("api4@gametools.network", &cookie).await?;
            cookie
        }
    };

    let empty_game_hash: HashMap<String, String> = HashMap::new();
    let mut sessions: HashMap<String, HashMap<String, String>> = HashMap::new();

    log::info!("Started");

    loop {
        match mongo_client.gather_managerinfo().await {
            Ok(_) => {},
            Err(e) => log::error!("Failed to send new manager info {:#?}", e),
        };

        let mut game_results: HashMap<String, results::RegionResult> = HashMap::new();
        let mut failed_games: Vec<&str> = vec![];

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
            match old_games::push_old_games(&mut mongo_client, key, value).await {
                Ok(game_result) => {
                    game_results.insert(key.to_string(), game_result);
                },
                Err(e) => {
                    log::error!("Failed oldgame: {}, with reason: {:#?}", key, e);
                    failed_games.push(key);
                },
            };
        }

        let sparta_games = HashMap::from([
            ("tunguska", "bf1"),
            ("casablanca", "bfv"),
            ("bf4", "bf4")
        ]);
        for (key, value) in sparta_games.into_iter() {
            match companion::gather_companion(&mut mongo_client, sessions.get(key).unwrap_or(&empty_game_hash).to_owned(), cookie.clone(), key, value).await {
                Ok((session, platform_result)) => {
                    sessions.insert(key.to_string(), session);
                    game_results.insert(key.to_string(), platform_result);
                },
                Err(e) => {
                    log::error!("Failed sparta_game: {}, with reason: {:#?}", key, e);
                    failed_games.push(key);
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
            match battlelog::gather_battlelog(&mut mongo_client, key, value).await {
                Ok(game_result) => {
                    game_results.insert(key.to_string(), game_result);
                },
                Err(e) => {
                    log::error!("Failed battlelog_game: {}, with reason: {:#?}", key, e);
                    failed_games.push(key);
                },
            };
        }
        match battlefield_grpc::gather_grpc(&mut mongo_client, sessions.get("kingston").unwrap_or(&empty_game_hash).to_owned(), cookie.clone()).await {
            Ok((session, game_result)) => {
                sessions.insert("kingston".to_string(), session);
                game_results.insert("kingston".to_string(), game_result);
            },
            Err(e) => {
                log::error!("Failed kingston_grpc, with reason: {:#?}", e);
                failed_games.push("kingston");
            },
        };
        
        // if no games failed, make global array
        if failed_games.iter().any(|&value| vec!["bf3", "bf4", "bfh", "tunguska", "casablanca", "kingston"].contains(&value)) {
            log::error!("1 of the important games failed to gather, skipping global array...");
        } else {
            let global_result = results::combine_region_players("global", "global", &game_results).await;
            match mongo_client.push_totals(global_result).await {
                Ok(_) => log::info!("successfully made global array"),
                Err(e) => log::error!("Failed to push global games array: {:#?}", e),
            };
        }

        last_update.store(Utc::now().timestamp() / 60, atomic::Ordering::Relaxed);
        sleep(Duration::from_secs(240)).await;
    }
}
