use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UnusedValue {}

#[derive(Serialize, Deserialize)]
pub struct Slots {
    #[serde(rename = "oneToFive")]
    pub one_to_five: String,
    #[serde(rename = "sixToTen")]
    pub six_to_ten: String,
    #[serde(rename = "tenPlus")]
    pub ten_plus: String,
    pub none: String,
}

#[derive(Serialize, Deserialize)]
pub struct Regions {
    #[serde(rename = "EU")]
    pub eu: String,
    #[serde(rename = "Asia")]
    pub asia: String,
    #[serde(rename = "NAm")]
    pub nam: String,
    #[serde(rename = "SAm")]
    pub sam: String,
    #[serde(rename = "AU")]
    pub au: String,
    #[serde(rename = "OC")]
    pub oc: String,
    #[serde(rename = "Afr")]
    pub afr: String,
    #[serde(rename = "AC")]
    pub ac: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerFilter {
    pub version: i64,
    pub name: String,
    pub vehicles: UnusedValue,
    #[serde(rename = "weaponClasses")]
    pub weapon_classes: UnusedValue,
    pub slots: Slots,
    pub regions: Regions,
    pub kits: UnusedValue,
    pub misc: UnusedValue,
    pub scales: UnusedValue,
}
