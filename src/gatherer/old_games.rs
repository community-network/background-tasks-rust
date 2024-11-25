use crate::{
    connectors::{mongo::MongoClient, timescale_db::push_server},
    structs::{old_games::OldGameServerList, results, server_info},
};
use chrono::Utc;
use futures::stream;
use influxdb2::models::{data_point::DataPointError, DataPoint};
use sqlx::PgPool;
use std::collections::HashMap;

pub fn build_data_point(
    game_name: &str,
    field: &str,
    amount: i64,
) -> Result<DataPoint, DataPointError> {
    DataPoint::builder(game_name)
        .tag("platform", "pc")
        .tag("region", "ALL")
        .tag("type", "amounts")
        .field(field, amount)
        .build()
}

pub async fn push_to_influx(
    influx_client: &influxdb2::Client,
    servers: &OldGameServerList,
    soldier_amount: &i64,
    frontend_game_name: &str,
) -> anyhow::Result<()> {
    let bucket = "Game info";
    let points = vec![
        build_data_point(
            frontend_game_name,
            "serverAmount",
            servers.server_list.len() as i64,
        )?,
        build_data_point(frontend_game_name, "soldierAmount", *soldier_amount)?,
    ];
    match influx_client
        .write_with_precision(
            bucket,
            stream::iter(points),
            influxdb2::api::write::TimestampPrecision::Seconds,
        )
        .await
    {
        Ok(_) => {}
        Err(e) => log::error!(
            "{} failed to push to influxdb: {:#?}",
            frontend_game_name,
            e
        ),
    };
    Ok(())
}

pub async fn push_old_games(
    pool: &PgPool,
    influx_client: &influxdb2::Client,
    mongo_client: &mut MongoClient,
    mongo_game_name: &str,
    frontend_game_name: &str,
) -> anyhow::Result<results::RegionResult> {
    let servers = match mongo_client.gather_old_title(mongo_game_name).await? {
        Some(servers) => servers,
        None => anyhow::bail!("No serverinfo gotten {}", frontend_game_name),
    };

    let bfbc2_maps = HashMap::from([
        (
            "levels/bc1_harvest_day".to_string(),
            "Harvest Day".to_string(),
        ),
        ("levels/bc1_oasis".to_string(), "Oasis".to_string()),
        ("levels/mp_001".to_string(), "Panama Canal".to_string()),
        ("levels/mp_002".to_string(), "Valparaiso".to_string()),
        ("levels/mp_003".to_string(), "Laguna Alta".to_string()),
        ("levels/mp_004".to_string(), "Isla Inocentes".to_string()),
        ("levels/mp_005".to_string(), "Atacama Desert".to_string()),
        ("levels/mp_006".to_string(), "Arica Harbour".to_string()),
        ("levels/mp_007".to_string(), "White Pass".to_string()),
        ("levels/mp_008".to_string(), "Nelson Bay".to_string()),
        ("levels/mp_009".to_string(), "Laguna Presa".to_string()),
        ("levels/mp_012".to_string(), "Port Valdez".to_string()),
        ("levels/mp_sp_002".to_string(), "Cold War".to_string()),
        ("levels/mp_sp_005".to_string(), "Heavy Metal".to_string()),
        ("levels/nam_mp_002".to_string(), "Vantage Point".to_string()),
        ("levels/nam_mp_003".to_string(), "Hill 137".to_string()),
        (
            "levels/nam_mp_005".to_string(),
            "Cai Son Temple".to_string(),
        ),
        (
            "levels/nam_mp_006".to_string(),
            "Phu Bai Valley".to_string(),
        ),
        (
            "levels/nam_mp_007".to_string(),
            "Operation Hastings".to_string(),
        ),
    ]);
    let bfbc2_gamemodes = HashMap::from([
        ("conquest".to_string(), "Conquest".to_string()),
        ("rush".to_string(), "Rush".to_string()),
        ("sqdm".to_string(), "Squad Deathmatch".to_string()),
        ("sqrush".to_string(), "Squad Rush".to_string()),
    ]);

    let bf2_modes = HashMap::from([
        ("gpm_cq".to_string(), "Conquest".to_string()),
        ("gpm_coop".to_string(), "Co-op".to_string()),
    ]);
    let bf1942_modes = HashMap::from([
        ("conquest".to_string(), "Conquest".to_string()),
        ("coop".to_string(), "Co-op".to_string()),
        ("ctf".to_string(), "Capture the Flag".to_string()),
        ("objectivemode".to_string(), "Objective Mode".to_string()),
        ("tdm".to_string(), "Team Deathmatch".to_string()),
    ]);
    let bf2142_modes = HashMap::from([
        ("gpm_cq".to_string(), "Conquest".to_string()),
        ("gpm_coop".to_string(), "Conquest Co-op".to_string()),
        ("gpm_sl".to_string(), "Assault Lines".to_string()),
        ("gpm_ti".to_string(), "Titan".to_string()),
        ("gpm_ca".to_string(), "Conquest Assault".to_string()),
        ("gpm_nv".to_string(), "No Vehicles".to_string()),
    ]);
    let bfvietnam_modes = HashMap::from([
        ("conquest".to_string(), "Conquest".to_string()),
        ("coop".to_string(), "Co-op".to_string()),
        ("customcombat".to_string(), "Custom Combat".to_string()),
        ("evolution".to_string(), "Evolution".to_string()),
    ]);

    let mut server_stats = vec![];
    let mut soldier_amount: i64 = 0;
    for server in &servers.server_list {
        let mut server_solier_amount = server.numplayers.parse::<i64>().unwrap_or_default();
        // bf2hub soldieramount can bug out for some reason
        if server_solier_amount > 200 {
            server_solier_amount = 0;
        }
        if frontend_game_name == "bfbc2" {
            let current_map: &String = &server.bfbc2_map.to_owned().unwrap_or_default();
            let current_mode: &String = &server.bfbc2_mode.to_owned().unwrap_or_default();

            server_stats.push(server_info::ServerInfo {
                guid: format!(
                    "{}:{}",
                    &server.bfbc2_ip.to_owned().unwrap_or_default(),
                    &server.bfbc2_port.to_owned().unwrap_or_default()
                ),
                name: server.bfbc2_name.to_owned().unwrap_or_default(),
                soldiers: server_solier_amount,
                queue: 0,
                mode: bfbc2_gamemodes
                    .get(&*current_mode)
                    .unwrap_or(&*current_mode)
                    .to_string(),
                map: bfbc2_maps
                    .get(&*current_map)
                    .unwrap_or(&*current_map)
                    .to_string(),
                game_id: "".to_owned(),
                is_official: None,
            });
        } else {
            let mut guid = format!(
                "{}:{}",
                &server.server_ip.to_owned().unwrap_or_default(),
                &server.server_port.to_owned().unwrap_or_default()
            );

            let current_mode = server.gametype.to_owned().unwrap_or_default();

            let mut translated_mode = bfvietnam_modes
                .get(&*current_mode)
                .unwrap_or(&&current_mode);

            if vec!["playbf2", "bf2hub"].contains(&frontend_game_name) {
                let server_ip: &String = &server.server_ip.to_owned().unwrap_or_default();
                let server_port: &String = &server.hostport.to_owned().unwrap_or_default();
                guid = format!("{}:{}", server_ip, server_port);
                translated_mode = bf2_modes.get(&*current_mode).unwrap_or(&&current_mode);
            } else if vec!["bf2142", "play2142"].contains(&frontend_game_name) {
                translated_mode = bf2142_modes.get(&*current_mode).unwrap_or(&&current_mode);
            } else if frontend_game_name == "bfield1942" {
                translated_mode = bf1942_modes.get(&*current_mode).unwrap_or(&&current_mode);
            }

            server_stats.push(server_info::ServerInfo {
                guid,
                name: server.hostname.to_owned().unwrap_or_default(),
                soldiers: server_solier_amount,
                queue: 0,
                mode: translated_mode.to_owned(),
                map: server.mapname.to_owned().unwrap_or_default(),
                game_id: "".to_owned(),
                is_official: None,
            });
        }

        soldier_amount += server_solier_amount;
    }

    match push_server(pool, frontend_game_name, "ALL", "pc", server_stats).await {
        Ok(_) => {}
        Err(e) => log::error!(
            "{} region failed to push specific serverinfo: {:#?}",
            frontend_game_name,
            e
        ),
    };

    push_to_influx(influx_client, &servers, &soldier_amount, frontend_game_name).await?;

    Ok(results::RegionResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
        amounts: results::RegionAmounts {
            server_amount: servers.server_list.len() as i64,
            soldier_amount: soldier_amount as i64,
            queue_amount: 0,
            spectator_amount: 0,
            dice_server_amount: 0,
            dice_soldier_amount: 0,
            dice_queue_amount: 0,
            dice_spectator_amount: 0,
            community_server_amount: 0,
            community_soldier_amount: 0,
            community_queue_amount: 0,
            community_spectator_amount: 0,
        },
        maps: HashMap::new(),
        modes: HashMap::new(),
        settings: HashMap::new(),
        owner_platform: HashMap::new(),
        timestamp: Utc::now(),
        map_players: HashMap::new(),
        mode_players: HashMap::new(),
        settings_players: HashMap::new(),
        playground: HashMap::new(),
        playground_players: HashMap::new(),
    })
}
