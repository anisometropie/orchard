use std::sync::{Arc, Mutex};

use crate::hexagon::models::Tree;
use crate::hexagon::ports::{
    OrchardTransaction, OrchardTransactionError, OrchardUnitOfWork, TreeRepository,
    TreeRepositoryError,
};

/// One in-memory storage family: normal repository calls and transactions use
/// the same committed orchard state.
pub struct InMemoryOrchardStorage {
    trees: Arc<Mutex<Vec<Tree>>>,
    failing_legacy_feature_id: Option<u32>,
    fail_to_begin: bool,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
}

impl InMemoryOrchardStorage {
    pub fn new() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(Vec::new(), None, false, false, false)
    }

    pub fn failing_when_saving_tree_with_legacy_feature_id(
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(Vec::new(), Some(legacy_feature_id), false, false, false)
    }

    pub fn failing_on_commit() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(Vec::new(), None, false, false, true)
    }

    pub fn failing_to_begin() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(Vec::new(), None, true, false, false)
    }

    pub fn failing_when_checking_legacy_feature_ids() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(Vec::new(), None, false, true, false)
    }

    pub fn with_existing_trees_failing_when_saving_tree_with_legacy_feature_id(
        existing_trees: Vec<Tree>,
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(existing_trees, Some(legacy_feature_id), false, false, false)
    }

    pub fn with_existing_trees(existing_trees: Vec<Tree>) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(existing_trees, None, false, false, false)
    }

    fn with_configuration(
        initial_trees: Vec<Tree>,
        failing_legacy_feature_id: Option<u32>,
        fail_to_begin: bool,
        fail_when_checking_legacy_feature_ids: bool,
        fail_on_commit: bool,
    ) -> (Self, InMemoryOrchardObserver) {
        let trees = Arc::new(Mutex::new(initial_trees));
        (
            Self {
                trees: Arc::clone(&trees),
                failing_legacy_feature_id,
                fail_to_begin,
                fail_when_checking_legacy_feature_ids,
                fail_on_commit,
            },
            InMemoryOrchardObserver { trees },
        )
    }
}

impl TreeRepository for InMemoryOrchardStorage {
    fn save(&mut self, tree: Tree) -> Result<(), TreeRepositoryError> {
        self.trees.lock().unwrap().push(tree);
        Ok(())
    }
}

impl OrchardUnitOfWork for InMemoryOrchardStorage {
    type Transaction = InMemoryOrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError> {
        if self.fail_to_begin {
            return Err(OrchardTransactionError::CouldNotBegin);
        }
        Ok(InMemoryOrchardTransaction {
            committed_trees: Arc::clone(&self.trees),
            staged_trees: Vec::new(),
            failing_legacy_feature_id: self.failing_legacy_feature_id,
            fail_when_checking_legacy_feature_ids: self.fail_when_checking_legacy_feature_ids,
            fail_on_commit: self.fail_on_commit,
        })
    }
}

pub struct InMemoryOrchardTransaction {
    committed_trees: Arc<Mutex<Vec<Tree>>>,
    staged_trees: Vec<Tree>,
    failing_legacy_feature_id: Option<u32>,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
}

impl OrchardTransaction for InMemoryOrchardTransaction {
    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError> {
        if self.fail_when_checking_legacy_feature_ids {
            return Err(OrchardTransactionError::CouldNotCheckExistingLegacyTree);
        }
        Ok(self
            .staged_trees
            .iter()
            .chain(self.committed_trees.lock().unwrap().iter())
            .any(|tree| tree.legacy_feature_id == Some(legacy_feature_id)))
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError> {
        if tree.legacy_feature_id == self.failing_legacy_feature_id {
            return Err(OrchardTransactionError::TreeCouldNotBeSaved);
        }
        self.staged_trees.push(tree);
        Ok(())
    }

    fn commit(self) -> Result<(), OrchardTransactionError> {
        if self.fail_on_commit {
            return Err(OrchardTransactionError::CouldNotCommit);
        }
        self.committed_trees
            .lock()
            .unwrap()
            .extend(self.staged_trees);
        Ok(())
    }

    fn rollback(self) {}
}

pub struct InMemoryOrchardObserver {
    trees: Arc<Mutex<Vec<Tree>>>,
}

impl InMemoryOrchardObserver {
    pub fn trees(&self) -> Vec<Tree> {
        self.trees.lock().unwrap().clone()
    }
}
