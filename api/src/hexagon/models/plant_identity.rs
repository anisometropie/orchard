use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlantIdentityId(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlantCultivarId(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlantIdentityReference {
    pub plant_identity_id: PlantIdentityId,
    pub cultivar_id: Option<PlantCultivarId>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnnualDate {
    pub month: u8,
    pub day: u8,
}

impl AnnualDate {
    pub fn new(month: u8, day: u8) -> Option<Self> {
        let last_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => 29,
            _ => return None,
        };
        (day >= 1 && day <= last_day).then_some(Self { month, day })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnnualHarvestWindow {
    pub start: AnnualDate,
    pub end: AnnualDate,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlantIdentity {
    pub common_name: String,
    pub botanical_taxon: BotanicalTaxon,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlantCultivar {
    pub cultivar: String,
    pub trade_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlantIdentification {
    #[serde(flatten)]
    pub plant_identity: PlantIdentity,
    #[serde(flatten)]
    pub plant_cultivar: Option<PlantCultivar>,
    pub identification_status: IdentificationStatus,
}

impl PlantIdentity {
    pub fn has_same_taxon_as(&self, other: &Self) -> bool {
        self.botanical_taxon == other.botanical_taxon
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum BotanicalTaxon {
    Named(NamedTaxon),
    HybridFormula { parents: [NamedTaxon; 2] },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NamedTaxon {
    pub genus: String,
    pub species: Option<String>,
    pub species_is_hybrid: bool,
    pub infraspecific: Option<InfraspecificTaxon>,
    pub is_aggregate: bool,
    pub cultivar_group: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InfraspecificTaxon {
    pub rank: InfraspecificRank,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum InfraspecificRank {
    Variety,
    Subspecies,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IdentificationStatus {
    Confirmed,
    Uncertain,
}
