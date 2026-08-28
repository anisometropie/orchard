use std::sync::{Arc, Mutex};

use crate::hexagon::models::{BotanicalTaxon, OrchardTree, PlantIdentity, PlantIdentityId, Tree};
use crate::hexagon::ports::{
    OrchardReadError, OrchardReader, OrchardTransaction, OrchardTransactionError, OrchardUnitOfWork,
};

/// In-memory transactional orchard storage for use-case and adapter tests.
pub struct InMemoryOrchardStorage {
    orchard: Arc<Mutex<InMemoryOrchard>>,
    failing_legacy_feature_id: Option<u32>,
    fail_when_saving_any_tree: bool,
    failing_plant_identity_genus: Option<String>,
    fail_to_begin: bool,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
    fail_when_reading_trees: bool,
}

#[derive(Default)]
struct InMemoryOrchard {
    plant_identities: Vec<PlantIdentity>,
    trees: Vec<Tree>,
    version: u64,
}

#[derive(Default)]
struct InMemoryOrchardConfiguration {
    plant_identities: Vec<PlantIdentity>,
    trees: Vec<Tree>,
    failing_legacy_feature_id: Option<u32>,
    fail_when_saving_any_tree: bool,
    failing_plant_identity_genus: Option<String>,
    fail_to_begin: bool,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
    fail_when_reading_trees: bool,
}

impl InMemoryOrchardStorage {
    pub fn new() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration::default())
    }

    pub fn failing_when_saving_tree_with_legacy_feature_id(
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            failing_legacy_feature_id: Some(legacy_feature_id),
            ..Default::default()
        })
    }

    pub fn failing_on_commit() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            fail_on_commit: true,
            ..Default::default()
        })
    }

    pub fn failing_to_begin() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            fail_to_begin: true,
            ..Default::default()
        })
    }

    pub fn failing_when_checking_legacy_feature_ids() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            fail_when_checking_legacy_feature_ids: true,
            ..Default::default()
        })
    }

    pub fn failing_when_saving_any_tree() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            fail_when_saving_any_tree: true,
            ..Default::default()
        })
    }

    pub fn failing_when_resolving_plant_identity_with_genus(
        genus: &str,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            failing_plant_identity_genus: Some(genus.into()),
            ..Default::default()
        })
    }

    pub fn failing_when_reading_trees() -> Self {
        Self::with_configuration(InMemoryOrchardConfiguration {
            fail_when_reading_trees: true,
            ..Default::default()
        })
        .0
    }

    pub fn with_existing_orchard_failing_when_saving_tree_with_legacy_feature_id(
        plant_identities: Vec<PlantIdentity>,
        trees: Vec<Tree>,
        legacy_feature_id: u32,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            plant_identities,
            trees,
            failing_legacy_feature_id: Some(legacy_feature_id),
            ..Default::default()
        })
    }

    pub fn with_existing_orchard(
        plant_identities: Vec<PlantIdentity>,
        trees: Vec<Tree>,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            plant_identities,
            trees,
            ..Default::default()
        })
    }

    fn with_configuration(
        configuration: InMemoryOrchardConfiguration,
    ) -> (Self, InMemoryOrchardObserver) {
        let orchard = Arc::new(Mutex::new(InMemoryOrchard {
            plant_identities: configuration.plant_identities,
            trees: configuration.trees,
            version: 0,
        }));
        (
            Self {
                orchard: Arc::clone(&orchard),
                failing_legacy_feature_id: configuration.failing_legacy_feature_id,
                fail_when_saving_any_tree: configuration.fail_when_saving_any_tree,
                failing_plant_identity_genus: configuration.failing_plant_identity_genus,
                fail_to_begin: configuration.fail_to_begin,
                fail_when_checking_legacy_feature_ids: configuration
                    .fail_when_checking_legacy_feature_ids,
                fail_on_commit: configuration.fail_on_commit,
                fail_when_reading_trees: configuration.fail_when_reading_trees,
            },
            InMemoryOrchardObserver { orchard },
        )
    }
}

impl OrchardUnitOfWork for InMemoryOrchardStorage {
    type Transaction = InMemoryOrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError> {
        if self.fail_to_begin {
            return Err(OrchardTransactionError::CouldNotBegin);
        }
        let expected_orchard_version = self.orchard.lock().unwrap().version;
        Ok(InMemoryOrchardTransaction {
            committed_orchard: Arc::clone(&self.orchard),
            expected_orchard_version,
            staged_plant_identities: Vec::new(),
            staged_trees: Vec::new(),
            failing_legacy_feature_id: self.failing_legacy_feature_id,
            fail_when_saving_any_tree: self.fail_when_saving_any_tree,
            failing_plant_identity_genus: self.failing_plant_identity_genus.clone(),
            fail_when_checking_legacy_feature_ids: self.fail_when_checking_legacy_feature_ids,
            fail_on_commit: self.fail_on_commit,
        })
    }
}

impl OrchardReader for InMemoryOrchardStorage {
    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardReadError> {
        if self.fail_when_reading_trees {
            return Err(OrchardReadError::TreesCouldNotBeRead);
        }
        let orchard = self.orchard.lock().unwrap();
        orchard
            .trees
            .iter()
            .map(|tree| {
                let identity_index = tree
                    .plant_identity_id
                    .0
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .ok_or(OrchardReadError::TreesCouldNotBeRead)?;
                let plant_identity = orchard
                    .plant_identities
                    .get(identity_index)
                    .cloned()
                    .ok_or(OrchardReadError::TreesCouldNotBeRead)?;
                Ok(OrchardTree {
                    tree: tree.clone(),
                    plant_identity,
                })
            })
            .collect()
    }
}

pub struct InMemoryOrchardTransaction {
    committed_orchard: Arc<Mutex<InMemoryOrchard>>,
    expected_orchard_version: u64,
    staged_plant_identities: Vec<PlantIdentity>,
    staged_trees: Vec<Tree>,
    failing_legacy_feature_id: Option<u32>,
    fail_when_saving_any_tree: bool,
    failing_plant_identity_genus: Option<String>,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
}

impl InMemoryOrchardTransaction {
    fn has_legacy_feature_id(&self, legacy_feature_id: u32) -> bool {
        self.staged_trees.iter().any(|tree| {
            tree.legacy_source
                .as_ref()
                .is_some_and(|source| source.feature_id == legacy_feature_id)
        }) || self
            .committed_orchard
            .lock()
            .unwrap()
            .trees
            .iter()
            .any(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == legacy_feature_id)
            })
    }
}

impl OrchardTransaction for InMemoryOrchardTransaction {
    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError> {
        if self.fail_when_checking_legacy_feature_ids {
            return Err(OrchardTransactionError::CouldNotCheckExistingLegacyTree);
        }
        Ok(self.has_legacy_feature_id(legacy_feature_id))
    }

    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardTransactionError> {
        if self
            .failing_plant_identity_genus
            .as_ref()
            .is_some_and(|failing_genus| {
                matches!(
                    &plant_identity.botanical_taxon,
                    BotanicalTaxon::Named(taxon) if &taxon.genus == failing_genus
                )
            })
        {
            return Err(OrchardTransactionError::PlantIdentityCouldNotBeResolved);
        }
        let committed_orchard = self.committed_orchard.lock().unwrap();
        if let Some(position) = committed_orchard
            .plant_identities
            .iter()
            .position(|existing| existing.has_same_catalog_identity_as(&plant_identity))
        {
            return Ok(PlantIdentityId((position + 1) as u64));
        }
        let committed_identity_count = committed_orchard.plant_identities.len();
        drop(committed_orchard);

        if let Some(position) = self
            .staged_plant_identities
            .iter()
            .position(|existing| existing.has_same_catalog_identity_as(&plant_identity))
        {
            return Ok(PlantIdentityId(
                (committed_identity_count + position + 1) as u64,
            ));
        }

        self.staged_plant_identities.push(plant_identity);
        Ok(PlantIdentityId(
            (committed_identity_count + self.staged_plant_identities.len()) as u64,
        ))
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError> {
        let available_identity_count = self
            .committed_orchard
            .lock()
            .unwrap()
            .plant_identities
            .len()
            + self.staged_plant_identities.len();
        if tree.plant_identity_id.0 == 0
            || tree.plant_identity_id.0 > available_identity_count as u64
        {
            return Err(OrchardTransactionError::TreeCouldNotBeSaved);
        }
        if tree
            .legacy_source
            .as_ref()
            .is_some_and(|source| self.has_legacy_feature_id(source.feature_id))
        {
            return Err(OrchardTransactionError::TreeCouldNotBeSaved);
        }
        if self.fail_when_saving_any_tree
            || tree
                .legacy_source
                .as_ref()
                .is_some_and(|source| Some(source.feature_id) == self.failing_legacy_feature_id)
        {
            return Err(OrchardTransactionError::TreeCouldNotBeSaved);
        }
        self.staged_trees.push(tree);
        Ok(())
    }

    fn commit(self) -> Result<(), OrchardTransactionError> {
        if self.fail_on_commit {
            return Err(OrchardTransactionError::CouldNotCommit);
        }
        let mut committed_orchard = self.committed_orchard.lock().unwrap();
        if committed_orchard.version != self.expected_orchard_version {
            return Err(OrchardTransactionError::CouldNotCommit);
        }
        committed_orchard
            .plant_identities
            .extend(self.staged_plant_identities);
        committed_orchard.trees.extend(self.staged_trees);
        committed_orchard.version += 1;
        Ok(())
    }

    fn rollback(self) {}
}

pub struct InMemoryOrchardObserver {
    orchard: Arc<Mutex<InMemoryOrchard>>,
}

impl InMemoryOrchardObserver {
    pub fn plant_identities(&self) -> Vec<PlantIdentity> {
        self.orchard.lock().unwrap().plant_identities.clone()
    }

    pub fn trees(&self) -> Vec<Tree> {
        self.orchard.lock().unwrap().trees.clone()
    }
}
