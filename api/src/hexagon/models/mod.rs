mod legacy_tree_source;
mod map_configuration;
mod orchard_access;
mod plant_identity;
mod tree;
mod watering;

pub use legacy_tree_source::{LegacyPlantIdentification, LegacyTreeSource};
pub use map_configuration::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, GeoPoint, MapConfiguration,
};
pub use orchard_access::{
    AuthenticatedSession, Orchard, OrchardId, OrchardShareAccess, OrchardSharePermission, User,
    UserId,
};
pub use plant_identity::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestDataOrigin, HarvestScheduleOwner,
    HarvestedPart, IdentificationStatus, InfraspecificRank, InfraspecificTaxon, NamedTaxon,
    PlantCultivar, PlantCultivarId, PlantIdentification, PlantIdentity, PlantIdentityId,
    PlantIdentityReference,
};
pub use tree::{ReproductiveRole, Tree, TreeId};
pub use watering::{WateringRun, WateringRunId, WateringRunTarget};

#[derive(Clone, Debug, PartialEq)]
pub struct OrchardTree {
    pub id: TreeId,
    pub row_rank: Option<u32>,
    pub tree: Tree,
    pub plant_identity: PlantIdentity,
    pub plant_cultivar: Option<PlantCultivar>,
    pub harvest_windows: Vec<AnnualHarvestWindow>,
}
