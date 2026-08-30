use std::sync::{Arc, Mutex};

use crate::hexagon::models::{
    AerialOverlayId, AerialOverlayImage, AnnualHarvestWindow, BotanicalTaxon, MapConfiguration,
    OrchardTree, PlantCultivar, PlantCultivarId, PlantIdentification, PlantIdentity,
    PlantIdentityId, PlantIdentityReference, Tree, TreeId,
};
use crate::hexagon::ports::{
    MapConfigurationStorage, MapConfigurationStorageError, OrchardStorage, OrchardStorageError,
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
    map_configuration: Option<MapConfiguration>,
    aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
    transaction: Option<InMemoryOrchardTransaction>,
}

#[derive(Default)]
struct InMemoryOrchard {
    plant_identities: Vec<PlantIdentity>,
    plant_cultivars: Vec<StoredCultivar>,
    harvest_windows: Vec<Option<AnnualHarvestWindow>>,
    trees: Vec<Tree>,
}

#[derive(Clone)]
struct StoredCultivar {
    plant_identity_id: PlantIdentityId,
    cultivar: String,
    trade_name: Option<String>,
}

#[derive(Default)]
struct InMemoryOrchardTransaction {
    staged_plant_identities: Vec<PlantIdentity>,
    staged_plant_cultivars: Vec<StoredCultivar>,
    staged_trees: Vec<Tree>,
    staged_harvest_window_changes: Vec<(PlantIdentityId, AnnualHarvestWindow)>,
    staged_tree_danger_changes: Vec<(TreeId, bool)>,
    staged_tree_life_status_changes: Vec<(TreeId, bool)>,
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
    map_configuration: Option<MapConfiguration>,
    aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
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

    pub fn with_map_configuration(
        map_configuration: MapConfiguration,
        aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
    ) -> Self {
        Self::with_configuration(InMemoryOrchardConfiguration {
            map_configuration: Some(map_configuration),
            aerial_overlay_images,
            ..Default::default()
        })
        .0
    }

    fn with_configuration(
        configuration: InMemoryOrchardConfiguration,
    ) -> (Self, InMemoryOrchardObserver) {
        let harvest_windows = vec![None; configuration.plant_identities.len()];
        let orchard = Arc::new(Mutex::new(InMemoryOrchard {
            plant_identities: configuration.plant_identities,
            plant_cultivars: Vec::new(),
            harvest_windows,
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
                map_configuration: configuration.map_configuration,
                aerial_overlay_images: configuration.aerial_overlay_images,
                transaction: None,
            },
            InMemoryOrchardObserver { orchard },
        )
    }
}

impl MapConfigurationStorage for InMemoryOrchardStorage {
    fn map_configuration(
        &mut self,
    ) -> Result<Option<MapConfiguration>, MapConfigurationStorageError> {
        Ok(self.map_configuration.clone())
    }

    fn aerial_overlay_image(
        &mut self,
        overlay_id: AerialOverlayId,
    ) -> Result<Option<AerialOverlayImage>, MapConfigurationStorageError> {
        Ok(self
            .aerial_overlay_images
            .iter()
            .find(|(id, _)| *id == overlay_id)
            .map(|(_, image)| image.clone()))
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
                    .harvest_windows
                    .extend(std::iter::repeat_n(
                        None,
                        transaction.staged_plant_identities.len(),
                    ));
                committed_orchard
                    .plant_identities
                    .extend(transaction.staged_plant_identities);
                committed_orchard
                    .plant_cultivars
                    .extend(transaction.staged_plant_cultivars);
                committed_orchard.trees.extend(transaction.staged_trees);
                for (plant_identity_id, harvest_window) in transaction.staged_harvest_window_changes
                {
                    let index = plant_identity_index(plant_identity_id)
                        .expect("a harvest-window change should have a positive identity ID");
                    *committed_orchard
                        .harvest_windows
                        .get_mut(index)
                        .expect("a harvest-window change should target an existing identity") =
                        Some(harvest_window);
                }
                for (tree_id, is_in_danger) in transaction.staged_tree_danger_changes {
                    let index = tree_index(tree_id)
                        .expect("a staged danger change should have a positive tree ID");
                    committed_orchard
                        .trees
                        .get_mut(index)
                        .expect("a staged danger change should target an existing tree")
                        .is_in_danger = is_in_danger;
                }
                for (tree_id, is_alive) in transaction.staged_tree_life_status_changes {
                    let index = tree_index(tree_id)
                        .expect("a staged life-status change should have a positive tree ID");
                    committed_orchard
                        .trees
                        .get_mut(index)
                        .expect("a staged life-status change should target an existing tree")
                        .is_alive = is_alive;
                }
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

    fn resolve_plant_identification(
        &mut self,
        plant_identification: PlantIdentification,
    ) -> Result<PlantIdentityReference, OrchardStorageError> {
        let PlantIdentification {
            plant_identity,
            plant_cultivar,
            ..
        } = plant_identification;
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
        let committed_identity_position = committed_orchard
            .plant_identities
            .iter()
            .position(|existing| existing.has_same_taxon_as(&plant_identity));
        let committed_identity_count = committed_orchard.plant_identities.len();
        let committed_cultivar_count = committed_orchard.plant_cultivars.len();
        let committed_cultivar_id = committed_identity_position.and_then(|position| {
            plant_cultivar.as_ref().and_then(|plant_cultivar| {
                let plant_identity_id = PlantIdentityId((position + 1) as u64);
                committed_orchard
                    .plant_cultivars
                    .iter()
                    .position(|stored| {
                        stored.plant_identity_id == plant_identity_id
                            && stored.cultivar == plant_cultivar.cultivar
                    })
                    .map(|position| PlantCultivarId((position + 1) as u64))
            })
        });
        drop(committed_orchard);

        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let plant_identity_id = committed_identity_position.map_or_else(
            || {
                if let Some(position) = transaction
                    .staged_plant_identities
                    .iter()
                    .position(|existing| existing.has_same_taxon_as(&plant_identity))
                {
                    return PlantIdentityId((committed_identity_count + position + 1) as u64);
                }
                transaction.staged_plant_identities.push(plant_identity);
                PlantIdentityId(
                    (committed_identity_count + transaction.staged_plant_identities.len()) as u64,
                )
            },
            |position| PlantIdentityId((position + 1) as u64),
        );
        let cultivar_id = match plant_cultivar {
            None => None,
            Some(_) if committed_cultivar_id.is_some() => committed_cultivar_id,
            Some(PlantCultivar {
                cultivar,
                trade_name,
            }) => {
                let position = transaction
                    .staged_plant_cultivars
                    .iter()
                    .position(|stored| {
                        stored.plant_identity_id == plant_identity_id && stored.cultivar == cultivar
                    });
                let position = position.unwrap_or_else(|| {
                    transaction.staged_plant_cultivars.push(StoredCultivar {
                        plant_identity_id,
                        cultivar,
                        trade_name,
                    });
                    transaction.staged_plant_cultivars.len() - 1
                });
                Some(PlantCultivarId(
                    (committed_cultivar_count + position + 1) as u64,
                ))
            }
        };
        Ok(PlantIdentityReference {
            plant_identity_id,
            cultivar_id,
        })
    }

    fn change_plant_identity_harvest_window(
        &mut self,
        plant_identity_id: PlantIdentityId,
        harvest_window: AnnualHarvestWindow,
    ) -> Result<bool, OrchardStorageError> {
        let committed_identity_count = self.orchard.lock().unwrap().plant_identities.len();
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let available_identity_count =
            committed_identity_count + transaction.staged_plant_identities.len();
        if plant_identity_id.0 == 0 || plant_identity_id.0 > available_identity_count as u64 {
            return Ok(false);
        }
        transaction
            .staged_harvest_window_changes
            .push((plant_identity_id, harvest_window));
        Ok(true)
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
        let committed_cultivar_count = committed_orchard.plant_cultivars.len();
        let cultivar_is_valid = tree.cultivar_id.is_none_or(|cultivar_id| {
            cultivar_belongs_to_identity(
                &committed_orchard.plant_cultivars,
                cultivar_id,
                tree.plant_identity_id,
            )
        });
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
        let staged_cultivar_is_valid = tree.cultivar_id.is_none_or(|cultivar_id| {
            if cultivar_id.0 <= committed_cultivar_count as u64 {
                cultivar_is_valid
            } else {
                let staged_index = cultivar_id.0 as usize - committed_cultivar_count - 1;
                transaction
                    .staged_plant_cultivars
                    .get(staged_index)
                    .is_some_and(|cultivar| cultivar.plant_identity_id == tree.plant_identity_id)
            }
        });
        if tree.plant_identity_id.0 == 0
            || tree.plant_identity_id.0 > available_identity_count as u64
            || !staged_cultivar_is_valid
            || has_tree_with_same_legacy_feature(&transaction.staged_trees, &tree)
        {
            return Err(OrchardStorageError::TreeCouldNotBeSaved);
        }
        transaction.staged_trees.push(tree);
        Ok(())
    }

    fn tree_is_alive(&mut self, tree_id: TreeId) -> Result<Option<bool>, OrchardStorageError> {
        Ok(tree_index(tree_id).and_then(|index| {
            self.orchard
                .lock()
                .unwrap()
                .trees
                .get(index)
                .map(|tree| tree.is_alive)
        }))
    }

    fn change_tree_danger(
        &mut self,
        tree_id: TreeId,
        is_in_danger: bool,
    ) -> Result<(), OrchardStorageError> {
        let tree_exists = tree_index(tree_id)
            .is_some_and(|index| self.orchard.lock().unwrap().trees.get(index).is_some());
        if !tree_exists {
            return Err(OrchardStorageError::TreeDangerCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_tree_danger_changes
            .push((tree_id, is_in_danger));
        Ok(())
    }

    fn change_tree_life_status(
        &mut self,
        tree_id: TreeId,
        is_alive: bool,
    ) -> Result<(), OrchardStorageError> {
        let tree_exists = tree_index(tree_id)
            .is_some_and(|index| self.orchard.lock().unwrap().trees.get(index).is_some());
        if !tree_exists {
            return Err(OrchardStorageError::TreeLifeStatusCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_tree_life_status_changes
            .push((tree_id, is_alive));
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
            .enumerate()
            .map(|(index, tree)| {
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
                let plant_cultivar = if let Some(cultivar_id) = tree.cultivar_id {
                    let cultivar_index = cultivar_id
                        .0
                        .checked_sub(1)
                        .and_then(|index| usize::try_from(index).ok())
                        .ok_or(OrchardStorageError::TreesCouldNotBeRead)?;
                    let cultivar = orchard
                        .plant_cultivars
                        .get(cultivar_index)
                        .filter(|cultivar| cultivar.plant_identity_id == tree.plant_identity_id)
                        .ok_or(OrchardStorageError::TreesCouldNotBeRead)?;
                    Some(PlantCultivar {
                        cultivar: cultivar.cultivar.clone(),
                        trade_name: cultivar.trade_name.clone(),
                    })
                } else {
                    None
                };
                Ok(OrchardTree {
                    id: TreeId((index + 1) as u64),
                    tree: tree.clone(),
                    plant_identity,
                    plant_cultivar,
                    harvest_window: orchard.harvest_windows[identity_index],
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

fn tree_index(tree_id: TreeId) -> Option<usize> {
    tree_id
        .0
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
}

fn plant_identity_index(plant_identity_id: PlantIdentityId) -> Option<usize> {
    plant_identity_id
        .0
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
}

fn cultivar_belongs_to_identity(
    cultivars: &[StoredCultivar],
    cultivar_id: PlantCultivarId,
    plant_identity_id: PlantIdentityId,
) -> bool {
    cultivar_id
        .0
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| cultivars.get(index))
        .is_some_and(|cultivar| cultivar.plant_identity_id == plant_identity_id)
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

    pub fn harvest_window(
        &self,
        plant_identity_id: PlantIdentityId,
    ) -> Option<AnnualHarvestWindow> {
        plant_identity_index(plant_identity_id)
            .and_then(|index| self.orchard.lock().unwrap().harvest_windows[index])
    }
}
