use crate::hexagon::models::Tree;

#[derive(Debug, PartialEq)]
pub enum OrchardTransactionError {
    CouldNotBegin,
    CouldNotCheckExistingLegacyTree,
    TreeCouldNotBeSaved,
    CouldNotCommit,
}

pub trait OrchardTransaction {
    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError>;
    fn commit(self) -> Result<(), OrchardTransactionError>;
    fn rollback(self);
}

pub trait OrchardUnitOfWork {
    type Transaction: OrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError>;
}
