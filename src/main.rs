mod mongo;
mod gatherer;
mod structs;

use gatherer::{server_manager, old_games, companion};
use std::collections::HashMap;

use influxdb2::Client;
use bf_sparta::cookie_request;
use mongo::MongoClient;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let influx_client = Client::new("https://europe-west1-1.gcp.cloud2.influxdata.com", "Gametools network", "uWe8oo4ykDMatlYX2g_mJWt3jitcxIOaJU9rNaJUZGQuLPmi0KL_eIS8QqHq9EEjLkNTOoRdnZMFdARuzOIigw==");
    let mut mongo_client = MongoClient::connect().await?;

    // https://github.com/aprimadi/influxdb2
    let cookie = cookie_request::request_cookie(cookie_request::Login {
        email: "api4@gametools.network".to_string(),
        pass: "bqgPAHJDphaTpPcbRk8jfHhbCecnTnRN".to_string(),
    })
    .await?;

    loop {
        match server_manager::save_server_manager_info(&influx_client, &mut mongo_client).await {
            Ok(_) => {},
            Err(e) => println!("Failed to send new manager info {:#?}", e),
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
                Err(e) => println!("Failed oldgame: {}, with reason: {:#?}", key, e),
            };
        }

        let sparta_games = HashMap::from([
            ("tunguska", "bf1"),
            ("casablanca", "bfv"),
            ("bf4", "bf4")
        ]);
        for (key, value) in sparta_games.into_iter() {
            match companion::gather_companion(&influx_client, &cookie, key, value).await {
                Ok(_) => {},
                Err(_) => todo!(),
            };
        }
    }
}
