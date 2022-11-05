use std::collections::HashMap;

pub async fn get_region_stats(game_name: &str, platform: &str) -> anyhow::Result<String> {
    let battlelog_regions = HashMap::from([
        (1, "NAm"),
        (2, "SAm"),
        (4, "AU"),
        (8, "Africa"),
        (16, "EU"),
        (32, "Asia"),
        (64, "OC"),
    ]);
    
    Ok("oof".to_string())
}
