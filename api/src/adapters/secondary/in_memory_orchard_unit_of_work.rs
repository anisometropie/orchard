use std::sync::{Arc, Mutex};

use crate::hexagon::models::Tree;
use crate::hexagon::ports::{OrchardImportTransaction, OrchardTransactionError, OrchardUnitOfWork};

pub struct InMemoryOrchardUnitOfWork {
    trees: Arc<Mutex<Vec<Tree>>>,
    failing_legacy_feature_id: Option<u32>,
    fail_to_begin: bool,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
}

pub struct InMemoryOrchardObserver {
    trees: Arc<Mutex<Vec<Tree>>>,
}

pub struct InMemoryOrchardImportTransaction {
    trees: Arc<Mutex<Vec<Tree>>>,
    failing_legacy_feature_id: Option<u32>,
    fail_on_commit: bool,
    fail_when_checking_legacy_feature_ids: bool,
    staged_trees: Vec<Tree>,
}

impl InMemoryOrchardUnitOfWork {
    pub fn new() -> (Self, InMemoryOrchardObserver) {
        let trees = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                trees: Arc::clone(&trees),
                failing_legacy_feature_id: None,
                fail_to_begin: false,
                fail_when_checking_legacy_feature_ids: false,
                fail_on_commit: false,
            },
            InMemoryOrchardObserver { trees },
        )
    }

    pub fn failing_when_saving_tree_with_legacy_feature_id(
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        let (mut unit_of_work, observer) = Self::new();
        unit_of_work.failing_legacy_feature_id = Some(legacy_feature_id);
        (unit_of_work, observer)
    }

    pub fn failing_on_commit() -> (Self, InMemoryOrchardObserver) {
        let (mut unit_of_work, observer) = Self::new();
        unit_of_work.fail_on_commit = true;
        (unit_of_work, observer)
    }

    pub fn failing_to_begin() -> (Self, InMemoryOrchardObserver) {
        let (mut unit_of_work, observer) = Self::new();
        unit_of_work.fail_to_begin = true;
        (unit_of_work, observer)
    }

    pub fn failing_when_checking_legacy_feature_ids() -> (Self, InMemoryOrchardObserver) {
        let (mut unit_of_work, observer) = Self::new();
        unit_of_work.fail_when_checking_legacy_feature_ids = true;
        (unit_of_work, observer)
    }

    pub fn with_existing_trees_failing_when_saving_tree_with_legacy_feature_id(
        existing_trees: Vec<Tree>,
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        let trees = Arc::new(Mutex::new(existing_trees));
        (
            Self {
                trees: Arc::clone(&trees),
                failing_legacy_feature_id: Some(legacy_feature_id),
                fail_to_begin: false,
                fail_when_checking_legacy_feature_ids: false,
                fail_on_commit: false,
            },
            InMemoryOrchardObserver { trees },
        )
    }

    pub fn with_existing_trees(existing_trees: Vec<Tree>) -> (Self, InMemoryOrchardObserver) {
        let trees = Arc::new(Mutex::new(existing_trees));
        (
            Self {
                trees: Arc::clone(&trees),
                failing_legacy_feature_id: None,
                fail_to_begin: false,
                fail_when_checking_legacy_feature_ids: false,
                fail_on_commit: false,
            },
            InMemoryOrchardObserver { trees },
        )
    }
}

impl OrchardUnitOfWork for InMemoryOrchardUnitOfWork {
    type Transaction = InMemoryOrchardImportTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError> {
        if self.fail_to_begin {
            return Err(OrchardTransactionError::CouldNotBegin);
        }
        Ok(InMemoryOrchardImportTransaction {
            trees: Arc::clone(&self.trees),
            failing_legacy_feature_id: self.failing_legacy_feature_id,
            fail_on_commit: self.fail_on_commit,
            fail_when_checking_legacy_feature_ids: self.fail_when_checking_legacy_feature_ids,
            staged_trees: Vec::new(),
        })
    }
}

impl OrchardImportTransaction for InMemoryOrchardImportTransaction {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError> {
        if self.fail_when_checking_legacy_feature_ids {
            return Err(OrchardTransactionError::CouldNotCheckExistingLegacyFeature);
        }
        Ok(self
            .staged_trees
            .iter()
            .chain(self.trees.lock().unwrap().iter())
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
        self.trees.lock().unwrap().extend(self.staged_trees);
        Ok(())
    }

    fn rollback(self) {}
}

impl InMemoryOrchardObserver {
    pub fn trees(&self) -> Vec<Tree> {
        self.trees.lock().unwrap().clone()
    }
}
