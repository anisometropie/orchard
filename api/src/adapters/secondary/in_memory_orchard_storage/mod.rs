use std::sync::{Arc, Mutex};

use crate::hexagon::models::{
    AerialOverlayId, AerialOverlayImage, AnnualHarvestWindow, BotanicalTaxon, GeoPoint,
    HarvestScheduleOwner, MapConfiguration, Orchard, OrchardId, OrchardShareAccess,
    OrchardSharePermission, OrchardTree, PlantCultivar, PlantCultivarId, PlantIdentification,
    PlantIdentity, PlantIdentityId, PlantIdentityReference, Tree, TreeId, User, UserId,
    WateringRun, WateringRunId, WateringRunTarget,
};
use crate::hexagon::ports::{
    AccessControl, AccessControlError, MapConfigurationStorage, MapConfigurationStorageError,
    OrchardStorage, OrchardStorageError,
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
    map_configuration_orchard_id: Option<OrchardId>,
    aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
    transaction: Option<InMemoryOrchardTransaction>,
}

#[derive(Default)]
struct InMemoryOrchard {
    users: Vec<InMemoryUser>,
    sessions: Vec<(String, UserId)>,
    share_tokens: Vec<(OrchardShareAccess, String)>,
    share_token_sequence: u64,
    orchards: Vec<(Orchard, UserId)>,
    plant_identities: Vec<PlantIdentity>,
    plant_cultivars: Vec<StoredCultivar>,
    harvest_schedules: Vec<(HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    orchard_harvest_schedules: Vec<(OrchardId, HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    trees: Vec<Tree>,
    tree_orchard_ids: Vec<Option<OrchardId>>,
    tree_row_ranks: Vec<Option<u32>>,
    watering_runs: Vec<WateringRun>,
}

struct InMemoryUser {
    id: u64,
    username: String,
    password: String,
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
    staged_harvest_schedule_replacements: Vec<(HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    staged_orchard_harvest_schedule_replacements:
        Vec<(OrchardId, HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    staged_tree_danger_changes: Vec<(TreeId, bool)>,
    staged_tree_life_status_changes: Vec<(TreeId, bool)>,
    staged_row_orders: Vec<(OrchardId, String, Vec<TreeId>)>,
    staged_watering_runs: Vec<WateringRun>,
    staged_watered_trees: Vec<(WateringRunId, TreeId)>,
    staged_completed_watering_runs: Vec<WateringRunId>,
}

#[derive(Default)]
struct InMemoryOrchardConfiguration {
    users: Vec<InMemoryUser>,
    orchards: Vec<(Orchard, UserId)>,
    plant_identities: Vec<PlantIdentity>,
    trees: Vec<Tree>,
    tree_orchard_ids: Vec<Option<OrchardId>>,
    tree_row_ranks: Vec<Option<u32>>,
    failing_legacy_feature_id: Option<u32>,
    fail_when_saving_any_tree: bool,
    failing_plant_identity_genus: Option<String>,
    fail_to_begin: bool,
    fail_when_checking_legacy_feature_ids: bool,
    fail_on_commit: bool,
    fail_when_reading_trees: bool,
    map_configuration: Option<MapConfiguration>,
    map_configuration_orchard_id: Option<OrchardId>,
    aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
}

impl InMemoryOrchardStorage {
    pub fn new() -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration::default())
    }

    pub fn with_user_credentials(
        username: &str,
        password: &str,
    ) -> (Self, InMemoryOrchardObserver) {
        Self::with_configuration(InMemoryOrchardConfiguration {
            users: vec![InMemoryUser {
                id: 1,
                username: username.into(),
                password: password.into(),
            }],
            ..Default::default()
        })
    }

    pub fn with_user_owned_orchard(
        username: &str,
        password: &str,
        orchard: Orchard,
        plant_identities: Vec<PlantIdentity>,
        trees: Vec<Tree>,
    ) -> (Self, InMemoryOrchardObserver) {
        let orchard_id = orchard.id;
        let tree_count = trees.len();
        Self::with_configuration(InMemoryOrchardConfiguration {
            users: vec![InMemoryUser {
                id: 1,
                username: username.into(),
                password: password.into(),
            }],
            orchards: vec![(orchard, UserId(1))],
            plant_identities,
            trees,
            tree_orchard_ids: vec![Some(orchard_id); tree_count],
            map_configuration_orchard_id: Some(orchard_id),
            ..Default::default()
        })
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

    pub fn with_user_owned_orchard_and_map(
        username: &str,
        password: &str,
        orchard: Orchard,
        plant_identities: Vec<PlantIdentity>,
        trees: Vec<Tree>,
        map_configuration: MapConfiguration,
        aerial_overlay_images: Vec<(AerialOverlayId, AerialOverlayImage)>,
    ) -> Self {
        let orchard_id = orchard.id;
        let tree_count = trees.len();
        Self::with_configuration(InMemoryOrchardConfiguration {
            users: vec![InMemoryUser {
                id: 1,
                username: username.into(),
                password: password.into(),
            }],
            orchards: vec![(orchard, UserId(1))],
            plant_identities,
            trees,
            tree_orchard_ids: vec![Some(orchard_id); tree_count],
            map_configuration: Some(map_configuration),
            map_configuration_orchard_id: Some(orchard_id),
            aerial_overlay_images,
            ..Default::default()
        })
        .0
    }

    fn with_configuration(
        configuration: InMemoryOrchardConfiguration,
    ) -> (Self, InMemoryOrchardObserver) {
        let tree_row_ranks = if configuration.tree_row_ranks.is_empty() {
            vec![None; configuration.trees.len()]
        } else {
            configuration.tree_row_ranks
        };
        let orchard = Arc::new(Mutex::new(InMemoryOrchard {
            users: configuration.users,
            sessions: Vec::new(),
            share_tokens: Vec::new(),
            share_token_sequence: 0,
            orchards: configuration.orchards,
            plant_identities: configuration.plant_identities,
            plant_cultivars: Vec::new(),
            harvest_schedules: Vec::new(),
            orchard_harvest_schedules: Vec::new(),
            trees: configuration.trees,
            tree_orchard_ids: configuration.tree_orchard_ids,
            tree_row_ranks,
            watering_runs: Vec::new(),
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
                map_configuration_orchard_id: configuration.map_configuration_orchard_id,
                aerial_overlay_images: configuration.aerial_overlay_images,
                transaction: None,
            },
            InMemoryOrchardObserver { orchard },
        )
    }
}

impl AccessControl for InMemoryOrchardStorage {
    fn verify_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, AccessControlError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .users
            .iter()
            .find(|user| user.username == username && user.password == password)
            .map(|user| User {
                id: UserId(user.id),
                username: user.username.clone(),
            }))
    }

    fn create_session(&mut self, user_id: UserId) -> Result<String, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        let token = format!(
            "in-memory-session-{}-{}",
            user_id.0,
            orchard.sessions.len() + 1
        );
        orchard.sessions.push((token.clone(), user_id));
        Ok(token)
    }

    fn orchards_owned_by(&mut self, user_id: UserId) -> Result<Vec<Orchard>, AccessControlError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .orchards
            .iter()
            .filter(|(_, owner_user_id)| *owner_user_id == user_id)
            .map(|(orchard, _)| orchard.clone())
            .collect())
    }

    fn user_for_session(&mut self, token: &str) -> Result<Option<User>, AccessControlError> {
        let orchard = self.orchard.lock().unwrap();
        let user_id = orchard
            .sessions
            .iter()
            .find(|(session_token, _)| session_token == token)
            .map(|(_, user_id)| *user_id);
        Ok(user_id.and_then(|user_id| {
            orchard
                .users
                .iter()
                .find(|user| user.id == user_id.0)
                .map(|user| User {
                    id: user_id,
                    username: user.username.clone(),
                })
        }))
    }

    fn user_owns_orchard(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<bool, AccessControlError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .orchards
            .iter()
            .any(|(orchard, owner_user_id)| orchard.id == orchard_id && *owner_user_id == user_id))
    }

    fn replace_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        permission: OrchardSharePermission,
    ) -> Result<String, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        if !orchard.orchards.iter().any(|(candidate, owner_user_id)| {
            candidate.id == orchard_id && *owner_user_id == user_id
        }) {
            return Err(AccessControlError::OrchardOwnershipCouldNotBeRead);
        }
        orchard.share_token_sequence += 1;
        let token = format!(
            "in-memory-share-{}-{}",
            orchard_id.0, orchard.share_token_sequence
        );
        orchard.share_tokens.retain(|(access, _)| {
            access.orchard_id != orchard_id || access.permission != permission
        });
        orchard.share_tokens.push((
            OrchardShareAccess {
                orchard_id,
                permission,
            },
            token.clone(),
        ));
        Ok(token)
    }

    fn orchard_share_for_token(
        &mut self,
        token: &str,
    ) -> Result<Option<OrchardShareAccess>, AccessControlError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .share_tokens
            .iter()
            .find(|(_, share_token)| share_token == token)
            .map(|(access, _)| *access))
    }

    fn delete_session(&mut self, token: &str) -> Result<(), AccessControlError> {
        self.orchard
            .lock()
            .unwrap()
            .sessions
            .retain(|(session_token, _)| session_token != token);
        Ok(())
    }

    fn set_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<bool, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        let Some(user_index) = orchard
            .users
            .iter()
            .position(|user| user.username == username)
        else {
            return Ok(false);
        };
        let user_id = UserId(orchard.users[user_index].id);
        orchard.users[user_index].password = password.into();
        orchard
            .sessions
            .retain(|(_, session_user_id)| *session_user_id != user_id);
        Ok(true)
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

    fn map_configuration_for_orchard(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<MapConfiguration>, MapConfigurationStorageError> {
        let orchard = self
            .orchard
            .lock()
            .unwrap()
            .orchards
            .iter()
            .find(|(orchard, _)| orchard.id == orchard_id)
            .map(|(orchard, _)| orchard.clone());
        Ok(orchard.map(|orchard| MapConfiguration {
            default_center: crate::hexagon::models::GeoPoint {
                longitude: orchard.longitude,
                latitude: orchard.latitude,
            },
            aerial_overlays: self
                .map_configuration
                .as_ref()
                .filter(|_| self.map_configuration_orchard_id == Some(orchard_id))
                .map(|configuration| configuration.aerial_overlays.clone())
                .unwrap_or_default(),
        }))
    }

    fn aerial_overlay_image_for_orchard(
        &mut self,
        orchard_id: OrchardId,
        overlay_id: AerialOverlayId,
    ) -> Result<Option<AerialOverlayImage>, MapConfigurationStorageError> {
        if self.map_configuration_orchard_id != Some(orchard_id) {
            return Ok(None);
        }
        self.aerial_overlay_image(overlay_id)
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
                committed_orchard
                    .plant_cultivars
                    .extend(transaction.staged_plant_cultivars);
                committed_orchard.trees.extend(transaction.staged_trees);
                for (owner, harvest_windows) in transaction.staged_harvest_schedule_replacements {
                    committed_orchard
                        .harvest_schedules
                        .retain(|(existing_owner, _)| *existing_owner != owner);
                    if !harvest_windows.is_empty() {
                        committed_orchard
                            .harvest_schedules
                            .push((owner, harvest_windows));
                    }
                }
                for (orchard_id, owner, harvest_windows) in
                    transaction.staged_orchard_harvest_schedule_replacements
                {
                    committed_orchard.orchard_harvest_schedules.retain(
                        |(stored_orchard_id, stored_owner, _)| {
                            *stored_orchard_id != orchard_id || *stored_owner != owner
                        },
                    );
                    if !harvest_windows.is_empty() {
                        committed_orchard.orchard_harvest_schedules.push((
                            orchard_id,
                            owner,
                            harvest_windows,
                        ));
                    }
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
                for (orchard_id, row_name, ordered_tree_ids) in transaction.staged_row_orders {
                    for index in 0..committed_orchard.trees.len() {
                        let belongs_to_row = committed_orchard.tree_orchard_ids.get(index)
                            == Some(&Some(orchard_id))
                            && committed_orchard.trees[index].row_name.as_deref()
                                == Some(row_name.as_str());
                        if !belongs_to_row {
                            continue;
                        }
                        committed_orchard.tree_row_ranks[index] = ordered_tree_ids
                            .iter()
                            .position(|tree_id| tree_id.0 == (index + 1) as u64)
                            .map(|rank| (rank + 1) as u32);
                    }
                }
                committed_orchard
                    .watering_runs
                    .extend(transaction.staged_watering_runs);
                for (run_id, tree_id) in transaction.staged_watered_trees {
                    if let Some(run) = committed_orchard
                        .watering_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                    {
                        run.watered_tree_ids.push(tree_id);
                    }
                }
                for run_id in transaction.staged_completed_watering_runs {
                    if let Some(run) = committed_orchard
                        .watering_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                    {
                        run.completed = true;
                    }
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

    fn replace_harvest_windows(
        &mut self,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError> {
        let committed_orchard = self.orchard.lock().unwrap();
        let committed_identity_count = committed_orchard.plant_identities.len();
        let committed_cultivar_count = committed_orchard.plant_cultivars.len();
        let owner_exists_in_committed_orchard = match owner {
            HarvestScheduleOwner::PlantIdentity(id) => {
                id.0 > 0 && id.0 <= committed_identity_count as u64
            }
            HarvestScheduleOwner::PlantCultivar(id) => {
                id.0 > 0 && id.0 <= committed_cultivar_count as u64
            }
        };
        drop(committed_orchard);
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let owner_exists_in_staged_changes = match owner {
            HarvestScheduleOwner::PlantIdentity(id) => {
                id.0 > committed_identity_count as u64
                    && id.0
                        <= (committed_identity_count + transaction.staged_plant_identities.len())
                            as u64
            }
            HarvestScheduleOwner::PlantCultivar(id) => {
                id.0 > committed_cultivar_count as u64
                    && id.0
                        <= (committed_cultivar_count + transaction.staged_plant_cultivars.len())
                            as u64
            }
        };
        if !owner_exists_in_committed_orchard && !owner_exists_in_staged_changes {
            return Ok(false);
        }
        transaction
            .staged_harvest_schedule_replacements
            .retain(|(existing_owner, _)| *existing_owner != owner);
        transaction
            .staged_harvest_schedule_replacements
            .push((owner, harvest_windows));
        Ok(true)
    }

    fn replace_orchard_harvest_windows(
        &mut self,
        orchard_id: OrchardId,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError> {
        let orchard = self.orchard.lock().unwrap();
        let owner_exists = match owner {
            HarvestScheduleOwner::PlantIdentity(id) => {
                id.0 > 0 && id.0 <= orchard.plant_identities.len() as u64
            }
            HarvestScheduleOwner::PlantCultivar(id) => {
                id.0 > 0 && id.0 <= orchard.plant_cultivars.len() as u64
            }
        };
        drop(orchard);
        if !owner_exists {
            return Ok(false);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_orchard_harvest_schedule_replacements
            .push((orchard_id, owner, harvest_windows));
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
                let harvest_schedule_owner = tree.cultivar_id.map_or(
                    HarvestScheduleOwner::PlantIdentity(tree.plant_identity_id),
                    HarvestScheduleOwner::PlantCultivar,
                );
                let harvest_windows = orchard
                    .harvest_schedules
                    .iter()
                    .find(|(owner, _)| *owner == harvest_schedule_owner)
                    .map(|(_, windows)| windows.clone())
                    .unwrap_or_default();
                Ok(OrchardTree {
                    id: TreeId((index + 1) as u64),
                    row_rank: orchard.tree_row_ranks.get(index).copied().flatten(),
                    tree: tree.clone(),
                    plant_identity,
                    plant_cultivar,
                    harvest_windows,
                })
            })
            .collect()
    }

    fn trees_in_orchard(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<OrchardTree>, OrchardStorageError> {
        let trees = self.trees()?;
        let orchard = self.orchard.lock().unwrap();
        let mut trees = trees
            .into_iter()
            .filter(|tree| {
                tree_index(tree.id)
                    .and_then(|index| orchard.tree_orchard_ids.get(index))
                    .is_some_and(|stored_orchard_id| *stored_orchard_id == Some(orchard_id))
            })
            .collect::<Vec<_>>();
        for tree in &mut trees {
            let owner = tree.tree.cultivar_id.map_or(
                HarvestScheduleOwner::PlantIdentity(tree.tree.plant_identity_id),
                HarvestScheduleOwner::PlantCultivar,
            );
            if let Some(windows) = orchard
                .orchard_harvest_schedules
                .iter()
                .find(|(stored_orchard_id, stored_owner, _)| {
                    *stored_orchard_id == orchard_id && *stored_owner == owner
                })
                .map(|(_, _, windows)| windows.clone())
            {
                tree.harvest_windows = windows;
            }
        }
        Ok(trees)
    }

    fn tree_belongs_to_orchard(
        &mut self,
        tree_id: TreeId,
        orchard_id: OrchardId,
    ) -> Result<bool, OrchardStorageError> {
        Ok(tree_index(tree_id)
            .and_then(|index| {
                self.orchard
                    .lock()
                    .unwrap()
                    .tree_orchard_ids
                    .get(index)
                    .copied()
                    .flatten()
            })
            .is_some_and(|stored_orchard_id| stored_orchard_id == orchard_id))
    }

    fn replace_row_order(
        &mut self,
        orchard_id: OrchardId,
        row_name: &str,
        ordered_tree_ids: &[TreeId],
    ) -> Result<(), OrchardStorageError> {
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_row_orders
            .push((orchard_id, row_name.into(), ordered_tree_ids.to_vec()));
        Ok(())
    }

    fn active_watering_run(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<WateringRun>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .watering_runs
            .iter()
            .find(|run| run.orchard_id == orchard_id && !run.completed)
            .cloned())
    }

    fn watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<Option<WateringRun>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .watering_runs
            .iter()
            .find(|run| run.id == watering_run_id)
            .cloned())
    }

    fn create_watering_run(
        &mut self,
        orchard_id: OrchardId,
        target: &WateringRunTarget,
        water_source: Option<GeoPoint>,
        ordered_tree_ids: &[TreeId],
    ) -> Result<WateringRunId, OrchardStorageError> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let run_id = WateringRunId(
            (self.orchard.lock().unwrap().watering_runs.len()
                + transaction.staged_watering_runs.len()
                + 1) as u64,
        );
        transaction.staged_watering_runs.push(WateringRun {
            id: run_id,
            orchard_id,
            target: target.clone(),
            water_source,
            ordered_tree_ids: ordered_tree_ids.to_vec(),
            watered_tree_ids: vec![],
            completed: false,
        });
        Ok(run_id)
    }

    fn mark_watering_tree_watered(
        &mut self,
        watering_run_id: WateringRunId,
        tree_id: TreeId,
    ) -> Result<(), OrchardStorageError> {
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_watered_trees
            .push((watering_run_id, tree_id));
        Ok(())
    }

    fn complete_watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<(), OrchardStorageError> {
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_completed_watering_runs
            .push(watering_run_id);
        Ok(())
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

    pub fn row_order(&self, orchard_id: OrchardId, row_name: &str) -> Vec<TreeId> {
        let orchard = self.orchard.lock().unwrap();
        let mut ranked = orchard
            .trees
            .iter()
            .enumerate()
            .filter(|(index, tree)| {
                tree.row_name.as_deref() == Some(row_name)
                    && orchard.tree_orchard_ids.get(*index) == Some(&Some(orchard_id))
            })
            .filter_map(|(index, _)| {
                orchard.tree_row_ranks[index].map(|rank| (rank, TreeId((index + 1) as u64)))
            })
            .collect::<Vec<_>>();
        ranked.sort_by_key(|(rank, _)| *rank);
        ranked.into_iter().map(|(_, tree_id)| tree_id).collect()
    }

    pub fn active_watering_run_tree_ids(&self, orchard_id: OrchardId) -> Vec<TreeId> {
        self.orchard
            .lock()
            .unwrap()
            .watering_runs
            .iter()
            .find(|run| run.orchard_id == orchard_id && !run.completed)
            .map(|run| run.ordered_tree_ids.clone())
            .unwrap_or_default()
    }

    pub fn harvest_windows(&self, owner: HarvestScheduleOwner) -> Vec<AnnualHarvestWindow> {
        self.orchard
            .lock()
            .unwrap()
            .harvest_schedules
            .iter()
            .find(|(existing_owner, _)| *existing_owner == owner)
            .map(|(_, windows)| windows.clone())
            .unwrap_or_default()
    }

    pub fn orchard_harvest_windows(
        &self,
        orchard_id: OrchardId,
        owner: HarvestScheduleOwner,
    ) -> Vec<AnnualHarvestWindow> {
        self.orchard
            .lock()
            .unwrap()
            .orchard_harvest_schedules
            .iter()
            .find(|(stored_orchard_id, stored_owner, _)| {
                *stored_orchard_id == orchard_id && *stored_owner == owner
            })
            .map(|(_, _, windows)| windows.clone())
            .unwrap_or_default()
    }
}
