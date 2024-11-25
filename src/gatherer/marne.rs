use crate::{
    connectors::{influx_db, timescale_db::push_server},
    structs::{
        marne::{MarneServerInfo, MarneServerList},
        results, server_info,
    },
};
use chrono::Utc;
use regex::Regex;
use sqlx::PgPool;
use std::collections::HashMap;

async fn gather_servers(game: &str) -> Vec<crate::structs::marne::MarneServerInfo> {
    let client = reqwest::Client::new();
    let url = match game {
        "bfv" => "https://marne.io/api/v/srvlst/",
        _ => "https://marne.io/api/srvlst/",
    };
    match client.get(url).send().await {
        Ok(resp) => {
            let json_string = resp.text().await.unwrap_or_default();
            match json_string == "[]" {
                true => match serde_json::from_str::<Vec<MarneServerInfo>>(&json_string) {
                    // match resp.json::<MarneServerList>().await {
                    Ok(json_res) => {
                        return json_res;
                    }
                    Err(e) => {
                        log::error!("{} marne public json array is incorrect: {:#?}", game, e);
                        return vec![];
                    }
                },
                false => match serde_json::from_str::<MarneServerList>(&json_string) {
                    // match resp.json::<MarneServerList>().await {
                    Ok(json_res) => {
                        return json_res.servers;
                    }
                    Err(e) => {
                        log::error!("{} marne public json is incorrect: {:#?}", game, e);
                        return vec![];
                    }
                },
            }
        }
        Err(e) => {
            log::error!("{} marne public url failed: {:#?}", game, e);
            return vec![];
        }
    }
}

async fn server_list_to_sum(
    found_servers: Vec<MarneServerInfo>,
) -> (
    HashMap<String, results::RegionResult>,
    HashMap<String, Vec<server_info::ServerInfo>>,
) {
    let maps = HashMap::from([
        ("MP_Amiens", "Amiens"),
        ("MP_Chateau", "Ballroom Blitz"),
        ("MP_Desert", "Sinai Desert"),
        ("MP_FaoFortress", "Fao Fortress"),
        ("MP_Forest", "Argonne Forest"),
        ("MP_ItalianCoast", "Empire's Edge"),
        ("MP_MountainFort", "Monte Grappa"),
        ("MP_Scar", "St Quentin Scar"),
        ("MP_Suez", "Suez"),
        ("MP_Giant", "Giant's Shadow"),
        ("MP_Fields", "Soissons"),
        ("MP_Graveyard", "Rupture"),
        ("MP_Underworld", "Fort De Vaux"),
        ("MP_Verdun", "Verdun Heights"),
        ("MP_ShovelTown", "Prise de Tahure"),
        ("MP_Trench", "Nivelle Nights"),
        ("MP_Bridge", "Brusilov Keep"),
        ("MP_Islands", "Albion"),
        ("MP_Ravines", "Łupków Pass"),
        ("MP_Tsaritsyn", "Tsaritsyn"),
        ("MP_Valley", "Galicia"),
        ("MP_Volga", "Volga River"),
        ("MP_Beachhead", "Cape Helles"),
        ("MP_Harbor", "Zeebrugge"),
        ("MP_Naval", "Heligoland Bight"),
        ("MP_Ridge", "Achi Baba"),
        ("MP_Alps", "Razor's Edge"),
        ("MP_Blitz", "London Calling"),
        ("MP_Hell", "Passchendaele"),
        ("MP_London", "London Calling: Scourge"),
        ("MP_Offensive", "River Somme"),
        ("MP_River", "Caporetto"),
        // BFV
        ("MP_ArcticFjell", "Fjell 652"),
        ("MP_ArcticFjord", "Narvik"),
        ("MP_Arras", "Arras"),
        ("MP_Devastation", "Devastation"),
        ("MP_Escaut", "twisted steel"),
        ("MP_Foxhunt", "Aerodrome"),
        ("MP_Halfaya", "Hamada"),
        ("MP_Rotterdam", "Rotterdam"),
        ("MP_Hannut", "Panzerstorm"),
        ("MP_Crete", "Mercury"),
        ("MP_Kalamas", "Marita"),
        ("MP_Provence", "Provence"),
        ("MP_SandAndSea", "Al sudan"),
        ("MP_Bunker", "Operation Underground"),
        ("MP_IwoJima", "Iwo jima"),
        ("MP_TropicIslands", "Pacific storm"),
        ("MP_WakeIsland", "Wake island"),
        ("MP_Jungle", "Solomon islands"),
        ("MP_Libya", "Al marj encampment"),
        ("MP_Norway", "lofoten islands"),
        // bfv special maps
        ("DK_Norway", "Halvoy"),
        ("MP_Escaut_US", "Twisted Steel US"),
        ("MP_Hannut_US", "Panzerstorm US"),
        ("MP_GOps_Chapter2_Arras", "Arras (Chapter 2)"),
        ("MP_WE_Fortress_Devastation", "Devastation (Fortress)"),
        ("MP_WE_Fortress_Halfaya", "Hamada (Fortress)"),
        ("MP_WE_Grind_ArcticFjord", "Narvik (Grind)"),
        ("MP_WE_Grind_Devastation", "Devastation (Grind)"),
        ("MP_WE_Grind_Escaut", "Twisted Steel (Grind)"),
        ("MP_WE_Grind_Rotterdam", "Rotterdam (Grind)"),
    ]);

    let modes = HashMap::from([
        ("Conquest0", "Conquest"),
        ("Rush0", "Rush"),
        ("BreakThrough0", "Shock Operations"),
        ("BreakthroughLarge0", "Operations"),
        ("Possession0", "War pigeons"),
        ("TugOfWar0", "Frontlines"),
        ("AirAssault0", "Air assault"),
        ("Domination0", "Domination"),
        ("TeamDeathMatch0", "Team Deathmatch"),
        ("ZoneControl0", "Rush"),
    ]);

    let marne_regions =
        HashMap::from([("AS", "Asia"), ("SA", "sam"), ("NA", "nam"), ("AF", "afr")]);

    let mut all_regions: results::RegionResult = results::RegionResult {
        metadata: results::Metadata {
            region: "ALL".to_string(),
            platform: "pc".to_string(),
        },
        amounts: results::RegionAmounts {
            server_amount: 0,
            soldier_amount: 0,
            queue_amount: 0,
            community_server_amount: 0,
            community_soldier_amount: 0,
            community_queue_amount: 0,
            spectator_amount: 0,
            community_spectator_amount: 0,

            // unused
            dice_server_amount: 0,
            dice_soldier_amount: 0,
            dice_queue_amount: 0,
            dice_spectator_amount: 0,
        },
        maps: HashMap::new(),
        modes: HashMap::new(),
        timestamp: Utc::now(),
        map_players: HashMap::new(),
        mode_players: HashMap::new(),
        settings: HashMap::new(),
        settings_players: HashMap::new(),
        owner_platform: HashMap::new(),
        playground: HashMap::new(),
        playground_players: HashMap::new(),
    };
    let mut regions: HashMap<String, results::RegionResult> = HashMap::new();
    let mut server_stats: HashMap<String, Vec<server_info::ServerInfo>> = HashMap::new();

    for server in found_servers {
        let server_region = marne_regions
            .get(&server.region[..])
            .unwrap_or(&&server.region[..])
            .to_string();
        let mode = modes
            .get(&server.game_mode[..])
            .unwrap_or(&&server.game_mode[..])
            .to_string();

        let internal_map = match Regex::new(r"[^\/]+$").unwrap().find(&server.map_name[..]) {
            Some(location) => location.as_str(),
            None => &server.map_name[..],
        };
        let map = maps.get(internal_map).unwrap_or(&internal_map).to_string();

        regions
            .entry(server_region.clone())
            .and_modify(|region| {
                region.amounts.server_amount += 1;
                region.amounts.soldier_amount += server.current_players;
                region.amounts.spectator_amount += server.current_spectators;
                region
                    .maps
                    .entry(map.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                region
                    .map_players
                    .entry(map.clone())
                    .and_modify(|count| *count += server.current_players)
                    .or_insert(server.current_players);
                region
                    .modes
                    .entry(mode.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                region
                    .mode_players
                    .entry(mode.clone())
                    .and_modify(|count| *count += server.current_players)
                    .or_insert(server.current_players);
            })
            .or_insert({
                results::RegionResult {
                    metadata: results::Metadata {
                        region: server_region.clone(),
                        platform: "pc".to_string(),
                    },
                    amounts: results::RegionAmounts {
                        server_amount: 1,
                        soldier_amount: server.current_players,
                        queue_amount: 0,
                        dice_server_amount: 0,
                        dice_soldier_amount: 0,
                        dice_queue_amount: 0,
                        community_server_amount: 0,
                        community_soldier_amount: 0,
                        community_queue_amount: 0,
                        spectator_amount: server.current_spectators,
                        dice_spectator_amount: 0,
                        community_spectator_amount: 0,
                    },
                    maps: HashMap::from([(map.clone(), 1)]),
                    modes: HashMap::from([(mode.clone(), 1)]),
                    timestamp: Utc::now(),
                    map_players: HashMap::from([(map.clone(), server.current_players)]),
                    mode_players: HashMap::from([(mode.clone(), server.current_players)]),
                    settings: HashMap::new(),
                    settings_players: HashMap::new(),
                    owner_platform: HashMap::new(),
                    playground: HashMap::new(),
                    playground_players: HashMap::new(),
                }
            });

        all_regions
            .maps
            .entry(map.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        all_regions
            .map_players
            .entry(map.clone())
            .and_modify(|count| *count += server.current_players)
            .or_insert(server.current_players);
        all_regions
            .modes
            .entry(mode.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        all_regions
            .mode_players
            .entry(mode.clone())
            .and_modify(|count| *count += server.current_players)
            .or_insert(server.current_players);

        all_regions.amounts.server_amount += 1;
        all_regions.amounts.soldier_amount += server.current_players;
        all_regions.amounts.spectator_amount += server.current_spectators;

        let current_server_info = server_info::ServerInfo {
            guid: server.id.to_string(),
            name: server.name,
            soldiers: server.current_players,
            queue: 0,
            mode,
            map,
            game_id: "".to_owned(),
            is_official: None,
        };
        server_stats
            .entry(server_region)
            .and_modify(|region_info| region_info.push(current_server_info.clone()))
            .or_insert_with(|| vec![current_server_info]);
    }

    regions.insert("ALL".to_string(), all_regions);
    (regions, server_stats)
}

pub async fn push_marne(
    game: &str,
    pool: &PgPool,
    influx_client: &influxdb2::Client,
) -> anyhow::Result<results::RegionResult> {
    let found_servers = gather_servers(game).await;
    let (regions, server_stats) = server_list_to_sum(found_servers).await;
    for (region, server_stat) in server_stats {
        match push_server(pool, &format!("{}_marne", game), &region, "pc", server_stat).await {
            Ok(_) => {}
            Err(e) => log::error!(
                "{} Marne region {} failed to push specific serverinfo: {:#?}",
                game,
                region,
                e
            ),
        };
    }
    match influx_db::push_to_database(influx_client, &format!("{}_marne", game), "pc", &regions)
        .await
    {
        Ok(_) => {}
        Err(e) => log::error!("{} Marne failed to push to influxdb: {:#?}", game, e),
    };

    let result = match regions.get("ALL") {
        Some(result) => result,
        None => anyhow::bail!("{} Marne has no ALL region!", game),
    };
    Ok(result.to_owned())
}
