use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionAmounts {
    #[serde(rename = "serverAmount")]
    pub server_amount: i64,
    #[serde(rename = "soldierAmount")]
    pub soldier_amount: i64,
    #[serde(rename = "queueAmount")]
    pub queue_amount: i64,
    #[serde(rename = "spectatorAmount")]
    pub spectator_amount: i64,
    #[serde(rename = "diceServerAmount")]
    pub dice_server_amount: i64,
    #[serde(rename = "diceSoldierAmount")]
    pub dice_soldier_amount: i64,
    #[serde(rename = "diceQueueAmount")]
    pub dice_queue_amount: i64,
    #[serde(rename = "diceSpectatorAmount")]
    pub dice_spectator_amount: i64,
    #[serde(rename = "communityServerAmount")]
    pub community_server_amount: i64,
    #[serde(rename = "communitySoldierAmount")]
    pub community_soldier_amount: i64,
    #[serde(rename = "communityQueueAmount")]
    pub community_queue_amount: i64,
    #[serde(rename = "communitySpectatorAmount")]
    pub community_spectator_amount: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionResult {
    pub region: String,
    pub amounts: RegionAmounts,
    pub maps: HashMap<String, i64>,
    pub modes: HashMap<String, i64>,
    pub settings: HashMap<String, i64>,
    #[serde(rename = "ownerPlatform")]
    pub owner_platform: HashMap<String, i64>,
}

fn combine_regions(first_region: &RegionResult, second_region: &RegionResult) -> RegionResult {
    let mut combined_regions = first_region.clone();
    combined_regions.amounts.server_amount += second_region.amounts.server_amount;
    combined_regions.amounts.soldier_amount += second_region.amounts.soldier_amount;
    combined_regions.amounts.queue_amount += second_region.amounts.queue_amount;
    combined_regions.amounts.spectator_amount += second_region.amounts.spectator_amount;
    combined_regions.amounts.dice_server_amount += second_region.amounts.dice_server_amount;
    combined_regions.amounts.dice_soldier_amount += second_region.amounts.dice_soldier_amount;
    combined_regions.amounts.dice_queue_amount += second_region.amounts.dice_queue_amount;
    combined_regions.amounts.dice_spectator_amount += second_region.amounts.dice_spectator_amount;
    combined_regions.amounts.community_server_amount += second_region.amounts.community_server_amount;
    combined_regions.amounts.community_soldier_amount += second_region.amounts.community_soldier_amount;
    combined_regions.amounts.community_queue_amount += second_region.amounts.community_queue_amount;
    combined_regions.amounts.community_spectator_amount += second_region.amounts.community_spectator_amount;

    for (key, value) in &second_region.maps {
        combined_regions.maps.entry(key.to_string())
            .and_modify(|count| *count += value).or_insert(*value);
    }
    for (key, value) in &second_region.modes {
        combined_regions.modes.entry(key.to_string())
            .and_modify(|count| *count += value).or_insert(*value);
    }
    for (key, value) in &second_region.settings {
        combined_regions.settings.entry(key.to_string())
            .and_modify(|count| *count += value).or_insert(*value);
    }
    for (key, value) in &second_region.owner_platform {
        combined_regions.owner_platform.entry(key.to_string())
            .and_modify(|count| *count += value).or_insert(*value);
    }

    combined_regions
}

// the "ALL" region
pub async fn combine_region_players(region_name: &str, region_results: &HashMap<String, RegionResult>) -> RegionResult {
    let mut all_regions = RegionResult { 
        region: region_name.to_string(),
        amounts: RegionAmounts {
            server_amount: 0,
            soldier_amount: 0,
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
    };

    for region in region_results.values() {
        all_regions = combine_regions(&all_regions, region);
    }

    all_regions
}

// global platform for game
pub async fn combine_region_platforms(platform_results: &HashMap<String, HashMap<String, RegionResult>>) -> HashMap<String, RegionResult> {
    let mut all_platforms: HashMap<String, RegionResult> = HashMap::new();

    for (_, platform_result) in platform_results {
        for (region_name, region_result) in platform_result {
            all_platforms.entry(region_name.to_string()).and_modify(|all_regions| {
                let result = combine_regions(&all_regions, region_result);
                all_regions.amounts = result.amounts;
                all_regions.maps = result.maps;
                all_regions.modes = result.modes;
                all_regions.region = result.region;
                all_regions.settings = result.settings;
                all_regions.owner_platform = result.owner_platform;
            }).or_insert(region_result.to_owned());
        }
    }

    all_platforms
}
