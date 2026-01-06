mod check_ea_desktop_session;
mod connectors;
mod gatherer;
mod structs;

use gatherer::battlefield_grpc_bf2042;
use grpc_rust::access_token::ea_desktop_access_token;
use std::{
    collections::HashMap,
    env,
    sync::{atomic, Arc},
    time::Duration,
};
use tokio::time::sleep;
use warp::Filter;

use crate::connectors::mongo::MongoClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(_) => log::info!(".env not found, using env variables..."),
    };

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
    let mut mongo_client = MongoClient::connect().await?;

    let api_bf2042_account = env::var("API_BF2042_ACCOUNT").expect("API_BF2042_ACCOUNT wasn't set");

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

    let empty_game_hash: HashMap<String, String> = HashMap::new();
    let mut sessions: HashMap<String, HashMap<String, String>> = HashMap::new();

    log::info!("Started");

    loop {
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
                last_update.store(
                    chrono::Utc::now().timestamp() / 60,
                    atomic::Ordering::Relaxed,
                );
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
