use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlantIdentityId(pub u64);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlantIdentity {
    pub common_name: String,
    pub botanical_taxon: BotanicalTaxon,
    pub cultivar: Option<String>,
    pub trade_name: Option<String>,
    pub identification_status: IdentificationStatus,
}

impl PlantIdentity {
    pub fn has_same_catalog_identity_as(&self, other: &Self) -> bool {
        self.botanical_taxon == other.botanical_taxon
            && self.cultivar == other.cultivar
            && self.identification_status == other.identification_status
    }

    pub fn catalog_key(&self) -> String {
        #[derive(Serialize)]
        struct CatalogKey<'a> {
            botanical_taxon: &'a BotanicalTaxon,
            cultivar: &'a Option<String>,
            identification_status: &'a IdentificationStatus,
        }

        serde_json::to_string(&CatalogKey {
            botanical_taxon: &self.botanical_taxon,
            cultivar: &self.cultivar,
            identification_status: &self.identification_status,
        })
        .expect("a plant identity made of serializable fields should have a catalog key")
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum IdentificationStatus {
    Confirmed,
    Uncertain,
}
