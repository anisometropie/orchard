use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Tree {
    pub legacy_feature_id: Option<u32>,
    pub longitude: f64,
    pub latitude: f64,
    pub name: String,
    pub latin_name: Option<String>,
    pub planted_on: Option<String>,
    pub row_name: Option<String>,
    pub roles: Vec<String>,
    pub is_alive: bool,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
    pub adult_height_meters: Option<f64>,
    pub adult_width_meters: Option<f64>,
}
