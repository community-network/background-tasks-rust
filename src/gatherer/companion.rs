use crate::structs::companion::{ServerFilter, UnusedValue, Slots, Regions};
use super::push_to_database;
use bf_sparta::{cookie_request::CookieAuth, sparta_api};

pub async fn region_players(region: &str, game_name: &str, platform: &str) -> anyhow::Result<()> {
    let mut filters = ServerFilter {
        version: 6,
        name: "".to_string(),
        vehicles: UnusedValue {},
        weapon_classes: UnusedValue {},
        slots: Slots {
            one_to_five: "on".to_string(),
            six_to_ten: "on".to_string(),
            ten_plus: "on".to_string(),
            none: "on".to_string(),
        },
        regions: Regions {
            eu: "off".to_string(),
            asia: "off".to_string(),
            nam: "off".to_string(),
            sam: "off".to_string(),
            au: "off".to_string(),
            oc: "off".to_string(),
            afr: "off".to_string(),
            ac: "off".to_string(),
        },
        kits:  UnusedValue {},
        misc:  UnusedValue {},
        scales:  UnusedValue {},
    };

    match region {
        "EU" => filters.regions.eu = "on".to_string(),
        "Asia" => filters.regions.asia = "on".to_string(),
        "NAm" => filters.regions.nam = "on".to_string(),
        "SAm" => filters.regions.sam = "on".to_string(),
        "AU" => filters.regions.au = "on".to_string(),
        "OC" => filters.regions.oc = "on".to_string(),
        "Afr" => filters.regions.afr = "on".to_string(),
        "AC" => filters.regions.ac = "on".to_string(),
        _ => panic!("Unknown platform field")
    };

    let filter_json = serde_json::json!({
        "filterJson": serde_json::to_string(&filters)?,
        "game": game_name,
        "limit": 10000
    });
    
    sparta_api::get_data_from_ea(&session.session_id, platform, game_name, "GameServer.searchServers", filter_json).await?;

    Ok(())
}

pub async fn get_region_stats(game_name: &str, cookie_auth: &CookieAuth, platform: &str) -> anyhow::Result<String> {
    let cookie = bf_sparta::cookie::Cookie {
        sid: cookie_auth.sid,
        remid: cookie_auth.remid,
    };
    let session = sparta_api::check_gateway_session(cookie, old_session, platform, game_name, "en-us").await?;
    let sparta_regions = vec!["EU", "Asia", "NAm", "SAm", "AU", "OC", "Afr", "AC"];
    for region in sparta_regions {
        region_players(region, game_name, platform).await?;
    }
    

    Ok(session.session_id)
}

pub async fn gather_companion(influx_client: &influxdb2::Client, cookie: &CookieAuth, game_name: &str, frontend_game_name: &str) -> anyhow::Result<()> {
    let game_platforms = match &game_name.to_string()[..] {
        "tunguska" => vec!["pc", "ps4", "xboxone"],
        "casablanca" => vec!["pc", "ps4", "xboxone"],
        "bf4" => vec!["pc", "xboxone"],
        _ => vec!["pc"],
    };

    for platform in game_platforms {
       let session = match get_region_stats(game_name, cookie, platform).await {
            Ok(result) => {
                match push_to_database(influx_client, game_name, frontend_game_name, platform).await {
                    Ok(_) => {},
                    Err(_) => todo!(),
                };
                result
            },
            Err(e) => panic!("{} failed with platform {}: {:#?}", platform, game_name, e),
        };
    }
    
    Ok(())
}