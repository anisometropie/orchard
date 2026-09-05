use serde::{Deserialize, Serialize};

use super::{IdentificationStatus, LegacyTreeSource, PlantCultivarId, PlantIdentityId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TreeId(pub u64);

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
    pub cultivar_id: Option<PlantCultivarId>,
    pub identification_status: IdentificationStatus,
    pub longitude: f64,
    pub latitude: f64,
    pub planted_on: Option<String>,
    pub row_name: Option<String>,
    pub roles: Vec<String>,
    pub is_alive: bool,
    pub is_in_danger: bool,
    pub reproductive_role: Option<ReproductiveRole>,
    pub adult_height_meters: Option<f64>,
    pub adult_width_meters: Option<f64>,
}
