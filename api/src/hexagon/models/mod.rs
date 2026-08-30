mod legacy_tree_source;
mod map_configuration;
mod plant_identity;
mod tree;

pub use legacy_tree_source::{LegacyPlantIdentification, LegacyTreeSource};
pub use map_configuration::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, GeoPoint, MapConfiguration,
};
pub use plant_identity::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestScheduleOwner, IdentificationStatus,
    InfraspecificRank, InfraspecificTaxon, NamedTaxon, PlantCultivar, PlantCultivarId,
    PlantIdentification, PlantIdentity, PlantIdentityId, PlantIdentityReference,
};
pub use tree::{ReproductiveRole, Tree, TreeId};

#[derive(Clone, Debug, PartialEq)]
pub struct OrchardTree {
    pub id: TreeId,
    pub tree: Tree,
    pub plant_identity: PlantIdentity,
    pub plant_cultivar: Option<PlantCultivar>,
    pub harvest_windows: Vec<AnnualHarvestWindow>,
}
