mod harvest;
mod legacy_tree_source;
mod map_configuration;
mod orchard_access;
mod plant_identity;
mod tree;
mod tree_photo;
mod watering;

pub use harvest::{
    CompletedHarvestRun, HarvestDate, HarvestPeriod, HarvestRun, HarvestRunId, HarvestRunTarget,
    HarvestRunTree, HarvestTreeActionUndo, HarvestTreeOutcome, HarvestTreeOutcomeRecord,
    HarvestWindowExtension,
};
pub(crate) use harvest::{
    EligibleHarvestTree, concrete_harvest_period, current_harvest_part_periods,
    current_harvest_parts_and_period, eligible_harvest_trees, normalized_harvested_parts,
};
pub use legacy_tree_source::{LegacyPlantIdentification, LegacyTreeSource};
pub use map_configuration::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, GeoPoint, MapConfiguration,
};
pub use orchard_access::{
    AuthenticatedSession, CreatedOrchardShareToken, IssuedOrchardShareToken, Orchard, OrchardId,
    OrchardShareAccess, OrchardSharePermission, OrchardSharePermissions, OrchardShareTokenId, User,
    UserId,
};
pub use plant_identity::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestDataOrigin, HarvestScheduleOwner,
    HarvestedPart, IdentificationStatus, InfraspecificRank, InfraspecificTaxon, NamedTaxon,
    PlantCultivar, PlantCultivarId, PlantIdentification, PlantIdentity, PlantIdentityId,
    PlantIdentityReference,
};
pub use tree::{ReproductiveRole, Tree, TreeId};
pub use tree_photo::{TreePhoto, TreePhotoId, TreePhotoSummary, TreePhotoVariant};
pub use watering::{
    CompletedWateringRun, CompletedWateringRunTree, WateringRun, WateringRunId, WateringRunTarget,
};

#[derive(Clone, Debug, PartialEq)]
pub struct OrchardTree {
    pub id: TreeId,
    pub row_rank: Option<u32>,
    pub has_photo: bool,
    pub tree: Tree,
    pub plant_identity: PlantIdentity,
    pub plant_cultivar: Option<PlantCultivar>,
    pub harvest_windows: Vec<AnnualHarvestWindow>,
}
