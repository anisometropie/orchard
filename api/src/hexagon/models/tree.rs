use serde::{Deserialize, Serialize};

use super::{LegacyTreeSource, PlantIdentityId};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductiveRole {
    Female,
    Male,
    SelfFertile,
    Parthenocarpic,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Tree {
    pub legacy_source: Option<LegacyTreeSource>,
    pub plant_identity_id: PlantIdentityId,
    pub longitude: f64,
    pub latitude: f64,
    pub planted_on: Option<String>,
    pub row_name: Option<String>,
    pub roles: Vec<String>,
    pub is_alive: bool,
    pub reproductive_role: Option<ReproductiveRole>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
    pub adult_height_meters: Option<f64>,
    pub adult_width_meters: Option<f64>,
}
