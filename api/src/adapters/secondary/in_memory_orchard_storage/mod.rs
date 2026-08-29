use std::sync::{Arc, Mutex};

use crate::hexagon::models::{BotanicalTaxon, OrchardTree, PlantIdentity, PlantIdentityId, Tree};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

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
    transaction: Option<InMemoryOrchardTransaction>,
}

#[derive(Default)]
struct InMemoryOrchard {
    plant_identities: Vec<PlantIdentity>,
    trees: Vec<Tree>,
}

#[derive(Default)]
struct InMemoryOrchardTransaction {
    staged_plant_identities: Vec<PlantIdentity>,
    staged_trees: Vec<Tree>,
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
                transaction: None,
            },
            InMemoryOrchardObserver { orchard },
        )
    }
}

impl OrchardStorage for InMemoryOrchardStorage {
    fn transaction<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<OrchardStorageError>,
    {
        if self.fail_to_begin || self.transaction.is_some() {
            return Err(E::from(OrchardStorageError::AtomicOperationCouldNotBegin));
        }
        self.transaction = Some(InMemoryOrchardTransaction::default());
        let result = operation(self);
        let transaction = self
            .transaction
            .take()
            .expect("an active transaction should own staged orchard changes");

        match result {
            Err(error) => Err(error),
            Ok(_) if self.fail_on_commit => {
                Err(E::from(OrchardStorageError::AtomicOperationCouldNotCommit))
            }
            Ok(value) => {
                let mut committed_orchard = self.orchard.lock().unwrap();
                committed_orchard
                    .plant_identities
                    .extend(transaction.staged_plant_identities);
                committed_orchard.trees.extend(transaction.staged_trees);
                Ok(value)
            }
        }
    }

    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardStorageError> {
        if self.fail_when_checking_legacy_feature_ids {
            return Err(OrchardStorageError::ExistingLegacyTreeCouldNotBeChecked);
        }
        let exists_in_committed_orchard =
            has_legacy_feature_id(&self.orchard.lock().unwrap(), legacy_feature_id);
        let exists_in_staged_trees = self.transaction.as_ref().is_some_and(|transaction| {
            transaction.staged_trees.iter().any(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == legacy_feature_id)
            })
        });
        Ok(exists_in_committed_orchard || exists_in_staged_trees)
    }

    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardStorageError> {
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
            return Err(OrchardStorageError::PlantIdentityCouldNotBeResolved);
        }
        let committed_orchard = self.orchard.lock().unwrap();
        if let Some(position) = committed_orchard
            .plant_identities
            .iter()
            .position(|existing| existing.has_same_catalog_identity_as(&plant_identity))
        {
            return Ok(PlantIdentityId((position + 1) as u64));
        }
        let committed_identity_count = committed_orchard.plant_identities.len();
        drop(committed_orchard);

        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        if let Some(position) = transaction
            .staged_plant_identities
            .iter()
            .position(|existing| existing.has_same_catalog_identity_as(&plant_identity))
        {
            return Ok(PlantIdentityId(
                (committed_identity_count + position + 1) as u64,
            ));
        }
        transaction.staged_plant_identities.push(plant_identity);
        Ok(PlantIdentityId(
            (committed_identity_count + transaction.staged_plant_identities.len()) as u64,
        ))
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError> {
        if self.fail_when_saving_any_tree
            || tree
                .legacy_source
                .as_ref()
                .is_some_and(|source| Some(source.feature_id) == self.failing_legacy_feature_id)
        {
            return Err(OrchardStorageError::TreeCouldNotBeSaved);
        }
        let committed_orchard = self.orchard.lock().unwrap();
        let committed_identity_count = committed_orchard.plant_identities.len();
        if has_tree_with_same_legacy_feature(&committed_orchard.trees, &tree) {
            return Err(OrchardStorageError::TreeCouldNotBeSaved);
        }
        drop(committed_orchard);

        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let available_identity_count =
            committed_identity_count + transaction.staged_plant_identities.len();
        if tree.plant_identity_id.0 == 0
            || tree.plant_identity_id.0 > available_identity_count as u64
            || has_tree_with_same_legacy_feature(&transaction.staged_trees, &tree)
        {
            return Err(OrchardStorageError::TreeCouldNotBeSaved);
        }
        transaction.staged_trees.push(tree);
        Ok(())
    }

    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardStorageError> {
        if self.fail_when_reading_trees {
            return Err(OrchardStorageError::TreesCouldNotBeRead);
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
                    .ok_or(OrchardStorageError::TreesCouldNotBeRead)?;
                let plant_identity = orchard
                    .plant_identities
                    .get(identity_index)
                    .cloned()
                    .ok_or(OrchardStorageError::TreesCouldNotBeRead)?;
                Ok(OrchardTree {
                    tree: tree.clone(),
                    plant_identity,
                })
            })
            .collect()
    }
}

fn has_legacy_feature_id(orchard: &InMemoryOrchard, legacy_feature_id: u32) -> bool {
    orchard.trees.iter().any(|tree| {
        tree.legacy_source
            .as_ref()
            .is_some_and(|source| source.feature_id == legacy_feature_id)
    })
}

fn has_tree_with_same_legacy_feature(trees: &[Tree], candidate: &Tree) -> bool {
    candidate.legacy_source.as_ref().is_some_and(|source| {
        trees.iter().any(|tree| {
            tree.legacy_source
                .as_ref()
                .is_some_and(|existing_source| existing_source.feature_id == source.feature_id)
        })
    })
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
