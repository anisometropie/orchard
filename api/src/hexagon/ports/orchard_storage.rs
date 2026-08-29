use crate::hexagon::models::{OrchardTree, PlantIdentity, PlantIdentityId, Tree};

#[derive(Debug, PartialEq)]
pub enum OrchardStorageError {
    AtomicOperationCouldNotBegin,
    ExistingLegacyTreeCouldNotBeChecked,
    PlantIdentityCouldNotBeResolved,
    TreeCouldNotBeSaved,
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
    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardStorageError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError>;
    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardStorageError>;
}
