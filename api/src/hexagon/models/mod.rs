mod legacy_tree_source;
mod map_configuration;
mod plant_identity;
mod tree;

pub use legacy_tree_source::{LegacyPlantIdentification, LegacyTreeSource};
pub use map_configuration::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, GeoPoint, MapConfiguration,
};
pub use plant_identity::{
    BotanicalTaxon, IdentificationStatus, InfraspecificRank, InfraspecificTaxon, NamedTaxon,
    PlantIdentity, PlantIdentityId,
};
pub use tree::{ReproductiveRole, Tree, TreeId};

#[derive(Clone, Debug, PartialEq)]
pub struct OrchardTree {
    pub id: TreeId,
    pub tree: Tree,
    pub plant_identity: PlantIdentity,
}
