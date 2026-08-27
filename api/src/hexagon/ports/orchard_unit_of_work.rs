use crate::hexagon::models::{PlantIdentity, PlantIdentityId, Tree};

#[derive(Debug, PartialEq)]
pub enum OrchardTransactionError {
    CouldNotBegin,
    CouldNotCheckExistingLegacyTree,
    PlantIdentityCouldNotBeResolved,
    TreeCouldNotBeSaved,
    CouldNotCommit,
}

pub trait OrchardTransaction {
    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError>;
    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardTransactionError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError>;
    fn commit(self) -> Result<(), OrchardTransactionError>;
    fn rollback(self);
}

pub trait OrchardUnitOfWork {
    type Transaction: OrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError>;
}
