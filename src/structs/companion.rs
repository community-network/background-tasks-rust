use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UnusedValue {}

#[derive(Serialize)]
pub struct Slots<'a> {
    #[serde(rename = "oneToFive")]
    pub one_to_five: &'a str,
    #[serde(rename = "sixToTen")]
    pub six_to_ten: &'a str,
    #[serde(rename = "tenPlus")]
    pub ten_plus: &'a str,
    pub none: &'a str,
}

#[derive(Serialize)]
pub struct Regions<'a> {
    #[serde(rename = "EU")]
    pub eu: &'a str,
    #[serde(rename = "Asia")]
    pub asia: &'a str,
    #[serde(rename = "NAm")]
    pub nam: &'a str,
    #[serde(rename = "SAm")]
    pub sam: &'a str,
    #[serde(rename = "AU")]
    pub au: &'a str,
    #[serde(rename = "OC")]
    pub oc: &'a str,
    #[serde(rename = "Afr")]
    pub afr: &'a str,
    #[serde(rename = "AC")]
    pub ac: &'a str,
}

#[derive(Serialize)]
pub struct ServerFilter<'a> {
    pub version: i64,
    pub name: String,
    pub vehicles: UnusedValue,
    #[serde(rename = "weaponClasses")]
    pub weapon_classes: UnusedValue,
    pub slots: Slots<'a>,
    pub regions: Regions<'a>,
    pub kits: UnusedValue,
    pub misc: UnusedValue,
    pub scales: UnusedValue,
    pub maps: std::collections::HashMap<String, String>,
}
