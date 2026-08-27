use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LegacyPlantIdentification {
    pub name: String,
    pub latin_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LegacyTreeSource {
    pub feature_id: u32,
    pub name: String,
    pub latin_name: String,
    pub legacy_identification: Option<LegacyPlantIdentification>,
    pub source_url: Option<String>,
}
