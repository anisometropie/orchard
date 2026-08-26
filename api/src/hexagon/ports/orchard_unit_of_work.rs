use crate::hexagon::models::Tree;

#[derive(Debug, PartialEq)]
pub enum OrchardTransactionError {
    CouldNotBegin,
    CouldNotCheckExistingLegacyFeature,
    TreeCouldNotBeSaved,
    CouldNotCommit,
}

pub trait OrchardImportTransaction {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError>;
    fn commit(self) -> Result<(), OrchardTransactionError>;
    fn rollback(self);
}

pub trait OrchardUnitOfWork {
    type Transaction: OrchardImportTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError>;
}
