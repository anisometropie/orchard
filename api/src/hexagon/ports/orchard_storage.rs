use crate::hexagon::models::{
    AnnualHarvestWindow, OrchardTree, PlantIdentification, PlantIdentityId, PlantIdentityReference,
    Tree, TreeId,
};

#[derive(Debug, PartialEq)]
pub enum OrchardStorageError {
    AtomicOperationCouldNotBegin,
    ExistingLegacyTreeCouldNotBeChecked,
    PlantIdentityCouldNotBeResolved,
    PlantIdentityHarvestWindowCouldNotBeChanged,
    TreeCouldNotBeSaved,
    TreeCouldNotBeRead,
    TreeDangerCouldNotBeChanged,
    TreeLifeStatusCouldNotBeChanged,
    AtomicOperationCouldNotCommit,
    TreesCouldNotBeRead,
}

pub trait OrchardStorage {
    fn transaction<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<OrchardStorageError>;

    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardStorageError>;
    fn resolve_plant_identification(
        &mut self,
        plant_identification: PlantIdentification,
    ) -> Result<PlantIdentityReference, OrchardStorageError>;
    fn change_plant_identity_harvest_window(
        &mut self,
        plant_identity_id: PlantIdentityId,
        harvest_window: AnnualHarvestWindow,
    ) -> Result<bool, OrchardStorageError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError>;
    fn tree_is_alive(&mut self, tree_id: TreeId) -> Result<Option<bool>, OrchardStorageError>;
    fn change_tree_danger(
        &mut self,
        tree_id: TreeId,
        is_in_danger: bool,
    ) -> Result<(), OrchardStorageError>;
    fn change_tree_life_status(
        &mut self,
        tree_id: TreeId,
        is_alive: bool,
    ) -> Result<(), OrchardStorageError>;
    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardStorageError>;
}
