mod check_ea_desktop_session;
mod connectors;
mod gatherer;
mod structs;

use crate::connectors::influx_db;
use bf_sparta::{cookie_request, sparta_api};
use connectors::mongo::MongoClient;
use gatherer::{
    battlebit, battlefield_grpc_bf2042, battlefield_grpc_bf6, battlelog, companion, marne,
    old_games,
};
use grpc_rust::access_token::ea_desktop_access_token;
use influxdb2::Client;
use sqlx::postgres::PgPool;
use std::{
    collections::HashMap,
    env,
    ops::Add,
    sync::{atomic, Arc},
    time::Duration,
};
use structs::results;
use tokio::time::sleep;
use warp::Filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(_) => log::info!(".env not found, using env variables..."),
    };

    let mins_between_runs = 5;
    let hours_between_detailed_runs = 12;
    let last_update = Arc::new(atomic::AtomicI64::new(chrono::Utc::now().timestamp() / 60));
    let last_update_clone = Arc::clone(&last_update);

    flexi_logger::Logger::try_with_str("info")?.start()?;
    log::info!("Starting...");

    tokio::spawn(async move {
        let hello = warp::any().map(move || {
            let last_update_i64 = last_update_clone.load(atomic::Ordering::Relaxed);
            let now_minutes = chrono::Utc::now().timestamp() / 60;

            // error if 10 minutes without updates
            if (now_minutes - last_update_i64) > 10 {
                warp::reply::with_status(
                    format!("{}", now_minutes - last_update_i64),
                    warp::http::StatusCode::SERVICE_UNAVAILABLE,
                )
            } else {
                warp::reply::with_status(
                    format!("{}", now_minutes - last_update_i64),
                    warp::http::StatusCode::OK,
                )
            }
        });
        warp::serve(hello).run(([0, 0, 0, 0], 3030)).await;
    });

    let influx_client = Client::new(
        env::var("INFLUX_URL").expect("INFLUX_URL wasn't set"),
        env::var("INFLUX_USER").expect("INFLUX_USER wasn't set"),
        env::var("INFLUX_PASS").expect("INFLUX_PASS wasn't set"),
    );
    let mut mongo_client = MongoClient::connect().await?;

    let pool = PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL wasn't set")).await?;

    let api_main_account = env::var("API_MAIN_ACCOUNT").expect("API_MAIN_ACCOUNT wasn't set");
    let api_bf2042_account = env::var("API_BF2042_ACCOUNT").expect("API_BF2042_ACCOUNT wasn't set");

    let (mut cookie, _) = match mongo_client.get_cookies(&api_main_account).await {
        Ok(result) => result,
        Err(e) => {
            log::warn!("Cookie failed, {}", e);
            (
                bf_sparta::cookie::Cookie {
                    sid: "".to_string(),
                    remid: "".to_string(),
                },
                "".to_string(),
            )
        }
    };

    let (mut bf2042_cookie, mut ea_access_token) =
        match mongo_client.get_cookies(&api_bf2042_account).await {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Cookie failed, {}", e);
                (
                    bf_sparta::cookie::Cookie {
                        sid: "".to_string(),
                        remid: "".to_string(),
                    },
                    "".to_string(),
                )
            }
        };

    if ea_access_token.is_empty() {
        match ea_desktop_access_token(bf2042_cookie.clone()).await {
            Ok(res) => {
                (ea_access_token, bf2042_cookie) = res;
                mongo_client
                    .push_new_cookies(&api_bf2042_account, &bf2042_cookie, ea_access_token.clone())
                    .await?;
            }
            Err(e) => log::error!("access_token for ea desktop failed: {:#?}", e),
        };
    }

    cookie = match sparta_api::get_token(cookie.clone(), "pc", "tunguska", "en-us").await {
        Ok(_) => cookie.clone(),
        Err(e) => {
            log::warn!("Cookie failed, {} - requesting new cookie", e);
            match cookie_request::request_cookie(cookie_request::Login {
                email: api_main_account.clone(),
                pass: env::var("API_MAIN_ACCOUNT_PASSWORD")
                    .expect("API_MAIN_ACCOUNT_PASSWORD wasn't set"),
            })
            .await
            {
                Ok(cookie_auth) => {
                    let cookie = bf_sparta::cookie::Cookie {
                        sid: cookie_auth.sid,
                        remid: cookie_auth.remid,
                    };
                    mongo_client
                        .push_new_cookies(&api_main_account, &cookie, "".to_string())
                        .await?;
                    cookie
                }
                Err(e) => {
                    log::warn!(
                        "Requesting new cookie failed, {} - using one from the manager",
                        e
                    );

                    let cookie = mongo_client.get_random_cookie().await?;
                    cookie
                }
            }
        }
    };

    let empty_game_hash: HashMap<String, String> = HashMap::new();
    let mut sessions: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut last_ran_detailed = chrono::Utc::now();
    let mut last_ran = chrono::Utc::now() - chrono::Duration::minutes(mins_between_runs);

    log::info!("Started");

    loop {
        let run = last_ran.add(chrono::Duration::minutes(mins_between_runs)) <= chrono::Utc::now();
        if run {
            log::info!("Starting new run");
            last_ran = chrono::Utc::now();

            match mongo_client.gather_managerinfo().await {
                Ok(result) => {
                    match gatherer::server_manager::save_server_manager_info(&influx_client, result)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("Failed to send new manager info to influxdb {:#?}", e)
                        }
                    };
                }
                Err(e) => log::error!("Failed to send new manager info {:#?}", e),
            };
            log::info!("manager done");

            let mut game_results: HashMap<String, results::RegionResult> = HashMap::new();
            let mut failed_games: Vec<&str> = vec![];

            let old_games = HashMap::from([
                ("bf2-playbf2", "playbf2"),
                ("bf2-bf2hub", "bf2hub"),
                ("bfield1942-bf1942org", "bfield1942"),
                ("bf2142-openspy", "bf2142"),
                ("bf2142-play2142", "play2142"),
                // ("bfbc2", "bfbc2"),
                ("bfvietnam-qtracker", "bfvietnam"),
                ("bfvietnam-openspy", "openspy"),
            ]);
            for (key, value) in old_games.into_iter() {
                match old_games::push_old_games(
                    &pool,
                    &influx_client,
                    &mut mongo_client,
                    key,
                    value,
                )
                .await
                {
                    Ok(game_result) => {
                        game_results.insert(key.to_string(), game_result);
                    }
                    Err(e) => {
                        log::error!("Failed oldgame: {}, with reason: {:#?}", key, e);
                        failed_games.push(key);
                    }
                };
            }
            log::info!("oldgames done");

            let sparta_games =
                HashMap::from([("tunguska", "bf1"), ("casablanca", "bfv"), ("bf4", "bf4")]);
            for (key, value) in sparta_games.into_iter() {
                match companion::gather_companion(
                    &pool,
                    &influx_client,
                    sessions.get(key).unwrap_or(&empty_game_hash).to_owned(),
                    cookie.clone(),
                    key,
                    value,
                )
                .await
                {
                    Ok((session, platform_result)) => {
                        sessions.insert(key.to_string(), session);
                        game_results.insert(key.to_string(), platform_result);
                    }
                    Err(e) => {
                        log::error!("Failed sparta_game: {}, with reason: {:#?}", key, e);
                        failed_games.push(key);
                    }
                };
            }
            log::info!("sparta done");

            // pc only!
            let battlelog_games = HashMap::from([
                (
                    "bf3",
                    "https://battlelog.battlefield.com/bf3/servers/getAutoBrowseServers/",
                ),
                (
                    "bf4",
                    "https://battlelog.battlefield.com/bf4/servers/getServers/pc/",
                ),
                (
                    "bfh",
                    "https://battlelog.battlefield.com/bfh/servers/getServers/pc/",
                ),
            ]);
            for (key, value) in battlelog_games {
                match battlelog::gather_battlelog(&pool, &influx_client, key, value).await {
                    Ok(game_result) => {
                        game_results.insert(key.to_string(), game_result);
                    }
                    Err(e) => {
                        log::error!("Failed battlelog_game: {}, with reason: {:#?}", key, e);
                        failed_games.push(key);
                    }
                };
            }
            log::info!("battlelog done");

            let run_detailed = last_ran_detailed
                .add(chrono::Duration::hours(hours_between_detailed_runs))
                <= chrono::Utc::now();
            if run_detailed {
                log::info!("Running grpc detailed");
                last_ran_detailed = chrono::Utc::now();
            }

            match battlefield_grpc_bf2042::gather_grpc(
                &pool,
                &influx_client,
                sessions
                    .get("kingston")
                    .unwrap_or(&empty_game_hash)
                    .to_owned(),
                bf2042_cookie.clone(),
                run_detailed,
                ea_access_token.clone(),
            )
            .await
            {
                Ok((session, game_result)) => {
                    sessions.insert("kingston".to_string(), session);
                    game_results.insert("kingston".to_string(), game_result);
                }
                Err(e) => {
                    log::error!("Failed kingston_grpc, with reason: {:#?}", e);
                    match ea_desktop_access_token(bf2042_cookie.clone()).await {
                        Ok(res) => {
                            (ea_access_token, bf2042_cookie) = res;
                            mongo_client
                                .push_new_cookies(
                                    &api_bf2042_account,
                                    &bf2042_cookie,
                                    ea_access_token.clone(),
                                )
                                .await?;
                        }
                        Err(e) => log::error!("access_token for ea desktop failed: {:#?}", e),
                    };
                    failed_games.push("kingston");
                }
            };
            match battlefield_grpc_bf6::gather_grpc(
                &pool,
                &influx_client,
                sessions
                    .get("santiago")
                    .unwrap_or(&empty_game_hash)
                    .to_owned(),
                bf2042_cookie.clone(),
                run_detailed,
                ea_access_token.clone(),
            )
            .await
            {
                Ok((session, game_result)) => {
                    sessions.insert("santiago".to_string(), session);
                    game_results.insert("santiago".to_string(), game_result);
                }
                Err(e) => {
                    log::error!("Failed santiago_grpc, with reason: {:#?}", e);
                    match ea_desktop_access_token(bf2042_cookie.clone()).await {
                        Ok(res) => {
                            (ea_access_token, bf2042_cookie) = res;
                            mongo_client
                                .push_new_cookies(
                                    &api_bf2042_account,
                                    &bf2042_cookie,
                                    ea_access_token.clone(),
                                )
                                .await?;
                        }
                        Err(e) => log::error!("access_token for ea desktop failed: {:#?}", e),
                    };
                    failed_games.push("santiago");
                }
            };
            log::info!("grpc done");
            for game in vec!["bf1", "bfv"] {
                match marne::push_marne(game, &pool, &influx_client).await {
                    Ok(game_result) => {
                        game_results.insert(format!("{}_marne", game), game_result);
                    }
                    Err(e) => {
                        log::error!("{} Marne failed with reason: {:#?}", game, e);
                    }
                };
            }
            log::info!("Marne done");

            // if no games failed, make global array
            if failed_games.iter().any(|&value| {
                vec![
                    "bf3",
                    "bf4",
                    "bfh",
                    "tunguska",
                    "casablanca",
                    "kingston",
                    "santiago",
                ]
                .contains(&value)
            }) {
                log::error!("1 of the important games failed to gather, skipping global array...");
            } else {
                let global_result =
                    results::combine_region_players("global", "global", &game_results).await;

                // influx
                match influx_db::push_totals(&influx_client, &global_result).await {
                    Ok(_) => log::info!("successfully made global array"),
                    Err(e) => log::error!("Failed to push global games array: {:#?}", e),
                };
            }
            log::info!("global done");

            match battlebit::push_battlebit(&pool, &influx_client).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Battlebit failed with reason: {:#?}", e);
                }
            };
            log::info!("Battlebit done");

            last_update.store(
                chrono::Utc::now().timestamp() / 60,
                atomic::Ordering::Relaxed,
            );
        } else {
            // rotating ea desktop token
            // for i in 6..11 {
            //     let id = &format!("desktop-api{}", i);
            //     let (mut ea_cookie, mut list_ea_access_token, cookie_valid) =
            //         match mongo_client.get_cookies_by_id(id).await {
            //             Ok(result) => result,
            //             Err(e) => {
            //                 log::warn!("Cookie failed, {}", e);
            //                 (
            //                     bf_sparta::cookie::Cookie {
            //                         sid: "".to_string(),
            //                         remid: "".to_string(),
            //                     },
            //                     "".to_string(),
            //                     false,
            //                 )
            //             }
            //         };
            //     if cookie_valid {
            //         let access_token_valid = match check_ea_desktop_session::get_session_info(
            //             list_ea_access_token.clone(),
            //         )
            //         .await
            //         {
            //             Ok(valid) => valid,
            //             Err(e) => {
            //                 log::error!("Failed ea desktop, auth_check reason: {:#?}", e);
            //                 false
            //             }
            //         };
            //         if access_token_valid {
            //             log::info!("ea desktop {}: Finished auth check!", i);
            //         } else {
            //             log::error!("getting new access token for ea desktop");
            //             match ea_desktop_access_token(ea_cookie.clone()).await {
            //                 Ok(res) => {
            //                     (list_ea_access_token, ea_cookie) = res;
            //                     mongo_client
            //                         .push_new_id_cookies(
            //                             id,
            //                             &ea_cookie,
            //                             list_ea_access_token.clone(),
            //                             true,
            //                         )
            //                         .await?;
            //                 }
            //                 Err(e) => {
            //                     log::error!("access_token for ea desktop failed: {:#?}", e);
            //                     mongo_client
            //                         .push_new_id_cookies(
            //                             id,
            //                             &ea_cookie,
            //                             list_ea_access_token.clone(),
            //                             false,
            //                         )
            //                         .await?;
            //                 }
            //             };
            //         }
            //     }
            // }

            let ten_mins = last_ran + chrono::Duration::minutes(10);
            log::info!(
                "Waiting {:#?} minutes before next run",
                (ten_mins - last_ran).num_minutes()
            );
            match battlefield_grpc_bf2042::check_session(
                sessions
                    .get("kingston")
                    .unwrap_or(&empty_game_hash)
                    .to_owned(),
                bf2042_cookie.clone(),
                ea_access_token.clone(),
            )
            .await
            {
                Ok(session) => {
                    sessions.insert("kingston".to_string(), session);
                    log::info!("kingston: Finished auth check!");
                }
                Err(e) => {
                    log::error!("Failed kingston_grpc, auth_check reason: {:#?}", e);
                    match ea_desktop_access_token(bf2042_cookie.clone()).await {
                        Ok(res) => {
                            (ea_access_token, bf2042_cookie) = res;
                            mongo_client
                                .push_new_cookies(
                                    &api_bf2042_account,
                                    &bf2042_cookie,
                                    ea_access_token.clone(),
                                )
                                .await?;
                        }
                        Err(e) => log::error!("access_token for ea desktop failed: {:#?}", e),
                    };
                }
            };
            sleep(Duration::from_secs(30)).await;
        }
    }
}
