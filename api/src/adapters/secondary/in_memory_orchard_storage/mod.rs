use std::sync::{Arc, Mutex};

use crate::hexagon::models::{
    AerialOverlayId, AerialOverlayImage, AnnualHarvestWindow, BotanicalTaxon, CompletedHarvestRun,
    CompletedWateringRun, CompletedWateringRunTree, GeoPoint, HarvestDataOrigin, HarvestDate,
    HarvestRun, HarvestRunId, HarvestRunTarget, HarvestRunTree, HarvestScheduleOwner,
    HarvestTreeActionUndo, HarvestTreeOutcome, HarvestTreeOutcomeRecord, HarvestWindowExtension,
    HarvestedPart, IssuedOrchardShareToken, MapConfiguration, Orchard, OrchardId,
    OrchardShareAccess, OrchardSharePermissions, OrchardShareTokenId, OrchardTree, PlantCultivar,
    PlantCultivarId, PlantIdentification, PlantIdentity, PlantIdentityId, PlantIdentityReference,
    Tree, TreeId, TreePhoto, TreePhotoVariant, User, UserId, WateringRun, WateringRunId,
    WateringRunTarget,
};
use crate::hexagon::ports::{
    AccessControl, AccessControlError, MapConfigurationStorage, MapConfigurationStorageError,
    OrchardStorage, OrchardStorageError, TreePhotoStorage, TreePhotoStorageError,
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
    share_tokens: Vec<InMemoryShareToken>,
    share_token_sequence: u64,
    orchards: Vec<(Orchard, UserId)>,
    plant_identities: Vec<PlantIdentity>,
    plant_cultivars: Vec<StoredCultivar>,
    harvest_schedules: Vec<(HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    orchard_harvest_schedules: Vec<(OrchardId, HarvestScheduleOwner, Vec<AnnualHarvestWindow>)>,
    trees: Vec<Tree>,
    tree_orchard_ids: Vec<Option<OrchardId>>,
    tree_row_ranks: Vec<Option<u32>>,
    tree_photos: Vec<(OrchardId, TreeId, TreePhoto)>,
    watering_runs: Vec<WateringRun>,
    harvest_runs: Vec<HarvestRun>,
    harvest_tree_action_undos: Vec<(HarvestRunId, HarvestTreeActionUndo)>,
}

struct InMemoryUser {
    id: u64,
    username: String,
    password: String,
}

struct InMemoryShareToken {
    id: OrchardShareTokenId,
    access: OrchardShareAccess,
    token: String,
    created_at_unix_seconds: i64,
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
    staged_tree_position_changes: Vec<(TreeId, GeoPoint)>,
    staged_row_orders: Vec<(OrchardId, String, Vec<TreeId>)>,
    staged_watering_runs: Vec<WateringRun>,
    staged_watered_trees: Vec<(WateringRunId, TreeId)>,
    staged_completed_watering_runs: Vec<WateringRunId>,
    staged_deleted_watering_runs: Vec<WateringRunId>,
    staged_harvest_runs: Vec<HarvestRun>,
    staged_harvest_tree_outcomes: Vec<(HarvestRunId, TreeId, HarvestTreeOutcome)>,
    staged_harvest_period_extensions: Vec<(HarvestRunId, TreeId, HarvestDate)>,
    staged_harvest_tree_action_undos: Vec<(HarvestRunId, HarvestTreeActionUndo)>,
    staged_restored_harvest_tree_actions: Vec<(HarvestRunId, HarvestTreeActionUndo)>,
    staged_completed_harvest_runs: Vec<HarvestRunId>,
    staged_deleted_harvest_runs: Vec<HarvestRunId>,
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
            tree_photos: Vec::new(),
            watering_runs: Vec::new(),
            harvest_runs: Vec::new(),
            harvest_tree_action_undos: Vec::new(),
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

    fn create_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        permissions: OrchardSharePermissions,
    ) -> Result<crate::hexagon::models::CreatedOrchardShareToken, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        if !orchard.orchards.iter().any(|(candidate, owner_user_id)| {
            candidate.id == orchard_id && *owner_user_id == user_id
        }) {
            return Err(AccessControlError::OrchardOwnershipCouldNotBeRead);
        }
        orchard.share_token_sequence += 1;
        let id = OrchardShareTokenId(orchard.share_token_sequence);
        let created_at_unix_seconds =
            i64::try_from(orchard.share_token_sequence).unwrap_or(i64::MAX);
        let token = format!(
            "in-memory-share-{}-{}",
            orchard_id.0, orchard.share_token_sequence
        );
        orchard.share_tokens.push(InMemoryShareToken {
            id,
            access: OrchardShareAccess {
                orchard_id,
                permissions,
            },
            token: token.clone(),
            created_at_unix_seconds,
        });
        Ok(crate::hexagon::models::CreatedOrchardShareToken { id, token })
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
            .find(|share| share.token == token)
            .map(|share| share.access))
    }

    fn issued_orchard_share_tokens(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<Vec<IssuedOrchardShareToken>, AccessControlError> {
        let orchard = self.orchard.lock().unwrap();
        if !orchard.orchards.iter().any(|(candidate, owner_user_id)| {
            candidate.id == orchard_id && *owner_user_id == user_id
        }) {
            return Err(AccessControlError::ShareTokensCouldNotBeListed);
        }
        Ok(orchard
            .share_tokens
            .iter()
            .filter(|share| share.access.orchard_id == orchard_id)
            .map(|share| IssuedOrchardShareToken {
                id: share.id,
                permissions: share.access.permissions,
                created_at_unix_seconds: share.created_at_unix_seconds,
            })
            .collect())
    }

    fn change_orchard_share_permissions(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        share_id: OrchardShareTokenId,
        permissions: OrchardSharePermissions,
    ) -> Result<bool, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        if !orchard.orchards.iter().any(|(candidate, owner_user_id)| {
            candidate.id == orchard_id && *owner_user_id == user_id
        }) {
            return Err(AccessControlError::ShareTokenCouldNotBeChanged);
        }
        let Some(share) = orchard
            .share_tokens
            .iter_mut()
            .find(|share| share.id == share_id && share.access.orchard_id == orchard_id)
        else {
            return Ok(false);
        };
        share.access.permissions = permissions;
        Ok(true)
    }

    fn revoke_orchard_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        share_id: OrchardShareTokenId,
    ) -> Result<bool, AccessControlError> {
        let mut orchard = self.orchard.lock().unwrap();
        if !orchard.orchards.iter().any(|(candidate, owner_user_id)| {
            candidate.id == orchard_id && *owner_user_id == user_id
        }) {
            return Err(AccessControlError::ShareTokenCouldNotBeRevoked);
        }
        let original_len = orchard.share_tokens.len();
        orchard
            .share_tokens
            .retain(|share| !(share.id == share_id && share.access.orchard_id == orchard_id));
        Ok(orchard.share_tokens.len() != original_len)
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

impl TreePhotoStorage for InMemoryOrchardStorage {
    fn save_tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        photo: TreePhoto,
    ) -> Result<bool, TreePhotoStorageError> {
        let mut orchard = self.orchard.lock().unwrap();
        let belongs_to_orchard = tree_index(tree_id)
            .and_then(|index| orchard.tree_orchard_ids.get(index))
            .is_some_and(|stored_orchard_id| *stored_orchard_id == Some(orchard_id));
        if !belongs_to_orchard {
            return Ok(false);
        }
        orchard.tree_photos.push((orchard_id, tree_id, photo));
        Ok(true)
    }

    fn latest_tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        variant: TreePhotoVariant,
    ) -> Result<Option<Vec<u8>>, TreePhotoStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .tree_photos
            .iter()
            .rev()
            .find(|(stored_orchard_id, stored_tree_id, _)| {
                *stored_orchard_id == orchard_id && *stored_tree_id == tree_id
            })
            .map(|(_, _, photo)| match variant {
                TreePhotoVariant::Full => photo.full_webp.clone(),
                TreePhotoVariant::Thumbnail => photo.thumbnail_webp.clone(),
            }))
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
                for (tree_id, position) in transaction.staged_tree_position_changes {
                    let index = tree_index(tree_id)
                        .expect("a staged position change should have a positive tree ID");
                    let tree = committed_orchard
                        .trees
                        .get_mut(index)
                        .expect("a staged position change should target an existing tree");
                    tree.longitude = position.longitude;
                    tree.latitude = position.latitude;
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
                committed_orchard
                    .watering_runs
                    .retain(|run| !transaction.staged_deleted_watering_runs.contains(&run.id));
                committed_orchard
                    .harvest_runs
                    .extend(transaction.staged_harvest_runs);
                for (run_id, tree_id, outcome) in transaction.staged_harvest_tree_outcomes {
                    if let Some(tree) = committed_orchard
                        .harvest_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                        .and_then(|run| {
                            run.ordered_trees
                                .iter_mut()
                                .find(|tree| tree.tree_id == tree_id)
                        })
                    {
                        tree.outcome = Some(outcome);
                    }
                }
                for (run_id, tree_id, new_end) in transaction.staged_harvest_period_extensions {
                    if let Some(tree) = committed_orchard
                        .harvest_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                        .and_then(|run| {
                            run.ordered_trees
                                .iter_mut()
                                .find(|tree| tree.tree_id == tree_id)
                        })
                    {
                        tree.period.end = new_end;
                    }
                }
                for run_id in transaction.staged_completed_harvest_runs {
                    if let Some(run) = committed_orchard
                        .harvest_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                    {
                        run.completed = true;
                    }
                }
                for (run_id, undo) in transaction.staged_harvest_tree_action_undos {
                    committed_orchard
                        .harvest_tree_action_undos
                        .retain(|(stored_run_id, _)| *stored_run_id != run_id);
                    committed_orchard
                        .harvest_tree_action_undos
                        .push((run_id, undo));
                }
                for (run_id, undo) in transaction.staged_restored_harvest_tree_actions {
                    if let Some(run) = committed_orchard
                        .harvest_runs
                        .iter_mut()
                        .find(|run| run.id == run_id)
                    {
                        run.completed = false;
                        if let Some(tree) = run
                            .ordered_trees
                            .iter_mut()
                            .find(|tree| tree.tree_id == undo.tree_id)
                        {
                            tree.outcome = undo.previous_outcome;
                            tree.period.end = undo.previous_period_end;
                        }
                    }
                    committed_orchard
                        .harvest_tree_action_undos
                        .retain(|(stored_run_id, _)| *stored_run_id != run_id);
                }
                committed_orchard
                    .harvest_runs
                    .retain(|run| !transaction.staged_deleted_harvest_runs.contains(&run.id));
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

    fn change_tree_position(
        &mut self,
        tree_id: TreeId,
        position: GeoPoint,
    ) -> Result<(), OrchardStorageError> {
        let tree_exists = tree_index(tree_id)
            .is_some_and(|index| self.orchard.lock().unwrap().trees.get(index).is_some());
        if !tree_exists {
            return Err(OrchardStorageError::TreePositionCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_tree_position_changes
            .push((tree_id, position));
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
                    has_photo: orchard.tree_photos.iter().any(
                        |(stored_orchard_id, stored_tree_id, _)| {
                            orchard.tree_orchard_ids.get(index) == Some(&Some(*stored_orchard_id))
                                && stored_tree_id.0 == (index + 1) as u64
                        },
                    ),
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

    fn completed_watering_runs(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<CompletedWateringRun>, OrchardStorageError> {
        let orchard = self.orchard.lock().unwrap();
        let mut runs = orchard
            .watering_runs
            .iter()
            .filter(|run| run.orchard_id == orchard_id && run.completed)
            .map(|run| {
                let started_at_unix_seconds = i64::try_from(run.id.0)
                    .ok()
                    .and_then(|id| id.checked_mul(100))
                    .ok_or(OrchardStorageError::RunHistoryCouldNotBeRead)?;
                let trees = run
                    .ordered_tree_ids
                    .iter()
                    .enumerate()
                    .map(|(index, tree_id)| CompletedWateringRunTree {
                        tree_id: *tree_id,
                        watered_at_unix_seconds: run.watered_tree_ids.contains(tree_id).then(
                            || {
                                started_at_unix_seconds
                                    + i64::try_from(index + 1).unwrap_or(i64::MAX)
                            },
                        ),
                    })
                    .collect::<Vec<_>>();
                let completed_at_unix_seconds = i64::try_from(trees.len() + 1)
                    .ok()
                    .and_then(|offset| started_at_unix_seconds.checked_add(offset))
                    .ok_or(OrchardStorageError::RunHistoryCouldNotBeRead)?;
                Ok(CompletedWateringRun {
                    id: run.id,
                    target: run.target.clone(),
                    carry_capacity: run.carry_capacity,
                    started_at_unix_seconds,
                    completed_at_unix_seconds,
                    trees,
                })
            })
            .collect::<Result<Vec<_>, OrchardStorageError>>()?;
        runs.sort_by_key(|run| std::cmp::Reverse(run.id.0));
        Ok(runs)
    }

    fn create_watering_run(
        &mut self,
        orchard_id: OrchardId,
        target: &WateringRunTarget,
        water_source: Option<GeoPoint>,
        carry_capacity: Option<u32>,
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
            carry_capacity,
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

    fn delete_watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<(), OrchardStorageError> {
        if !self
            .orchard
            .lock()
            .unwrap()
            .watering_runs
            .iter()
            .any(|run| run.id == watering_run_id)
        {
            return Err(OrchardStorageError::WateringRunCouldNotBeDeleted);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_deleted_watering_runs
            .push(watering_run_id);
        Ok(())
    }

    fn harvest_tree_outcomes(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<HarvestTreeOutcomeRecord>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .filter(|run| run.orchard_id == orchard_id)
            .flat_map(|run| {
                run.ordered_trees.iter().filter_map(|tree| {
                    tree.outcome.map(|outcome| HarvestTreeOutcomeRecord {
                        tree_id: tree.tree_id,
                        harvested_parts: tree.harvested_parts.clone(),
                        period: tree.period,
                        outcome,
                    })
                })
            })
            .collect())
    }

    fn active_harvest_run(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<HarvestRun>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .find(|run| run.orchard_id == orchard_id && !run.completed)
            .cloned())
    }

    fn harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<Option<HarvestRun>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .find(|run| run.id == harvest_run_id)
            .cloned())
    }

    fn completed_harvest_runs(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<CompletedHarvestRun>, OrchardStorageError> {
        let orchard = self.orchard.lock().unwrap();
        let mut runs = orchard
            .harvest_runs
            .iter()
            .filter(|run| run.orchard_id == orchard_id && run.completed)
            .map(|run| {
                let completed_at_unix_seconds = i64::try_from(run.id.0)
                    .ok()
                    .and_then(|id| id.checked_add(2_000_000))
                    .ok_or(OrchardStorageError::RunHistoryCouldNotBeRead)?;
                Ok(CompletedHarvestRun {
                    id: run.id,
                    target: run.target,
                    harvested_parts: run.harvested_parts.clone(),
                    started_on: run.started_on,
                    completed_at_unix_seconds,
                    ordered_trees: run.ordered_trees.clone(),
                })
            })
            .collect::<Result<Vec<_>, OrchardStorageError>>()?;
        runs.sort_by_key(|run| std::cmp::Reverse(run.id.0));
        Ok(runs)
    }

    fn create_harvest_run(
        &mut self,
        orchard_id: OrchardId,
        target: HarvestRunTarget,
        harvested_parts: &[HarvestedPart],
        started_on: HarvestDate,
        ordered_trees: &[HarvestRunTree],
    ) -> Result<HarvestRunId, OrchardStorageError> {
        let committed = self.orchard.lock().unwrap();
        if committed
            .harvest_runs
            .iter()
            .any(|run| run.orchard_id == orchard_id && !run.completed)
        {
            return Err(OrchardStorageError::HarvestRunCouldNotBeCreated);
        }
        let committed_max_run_id = committed
            .harvest_runs
            .iter()
            .map(|run| run.id.0)
            .max()
            .unwrap_or(0);
        drop(committed);
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        if transaction
            .staged_harvest_runs
            .iter()
            .any(|run| run.orchard_id == orchard_id && !run.completed)
        {
            return Err(OrchardStorageError::HarvestRunCouldNotBeCreated);
        }
        let max_run_id = transaction
            .staged_harvest_runs
            .iter()
            .map(|run| run.id.0)
            .max()
            .unwrap_or(committed_max_run_id)
            .max(committed_max_run_id);
        let run_id = HarvestRunId(
            max_run_id
                .checked_add(1)
                .ok_or(OrchardStorageError::HarvestRunCouldNotBeCreated)?,
        );
        transaction.staged_harvest_runs.push(HarvestRun {
            id: run_id,
            orchard_id,
            target,
            harvested_parts: harvested_parts.to_vec(),
            started_on,
            ordered_trees: ordered_trees.to_vec(),
            completed: false,
        });
        Ok(run_id)
    }

    fn record_harvest_tree_outcome(
        &mut self,
        harvest_run_id: HarvestRunId,
        tree_id: TreeId,
        outcome: HarvestTreeOutcome,
    ) -> Result<(), OrchardStorageError> {
        let stored_tree = self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .find(|run| run.id == harvest_run_id && !run.completed)
            .and_then(|run| {
                run.ordered_trees
                    .iter()
                    .find(|tree| tree.tree_id == tree_id)
            })
            .cloned();
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let can_resolve = stored_tree.is_some_and(|mut tree| {
            if let Some((_, _, staged_end)) = transaction
                .staged_harvest_period_extensions
                .iter()
                .rev()
                .find(|(run_id, staged_tree_id, _)| {
                    *run_id == harvest_run_id && *staged_tree_id == tree_id
                })
            {
                tree.period.end = *staged_end;
            }
            (tree.outcome.is_none()
                || tree
                    .outcome
                    .is_some_and(|stored_outcome| deferred_outcome_is_due(stored_outcome, outcome)))
                && !transaction.staged_harvest_tree_outcomes.iter().any(
                    |(run_id, staged_tree_id, _)| {
                        *run_id == harvest_run_id && *staged_tree_id == tree_id
                    },
                )
                && harvest_outcome_fits_period(outcome, tree.period)
        });
        if !can_resolve {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        }
        transaction
            .staged_harvest_tree_outcomes
            .push((harvest_run_id, tree_id, outcome));
        Ok(())
    }

    fn extend_harvest_run_tree_period(
        &mut self,
        harvest_run_id: HarvestRunId,
        tree_id: TreeId,
        new_end: HarvestDate,
        action_date: HarvestDate,
    ) -> Result<(), OrchardStorageError> {
        let stored_tree = self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .find(|run| run.id == harvest_run_id && !run.completed)
            .and_then(|run| {
                run.ordered_trees
                    .iter()
                    .find(|tree| tree.tree_id == tree_id)
            })
            .cloned();
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        let can_extend = stored_tree.is_some_and(|tree| {
            let effective_end = transaction
                .staged_harvest_period_extensions
                .iter()
                .rev()
                .find(|(run_id, staged_tree_id, _)| {
                    *run_id == harvest_run_id && *staged_tree_id == tree_id
                })
                .map(|(_, _, staged_end)| *staged_end)
                .unwrap_or(tree.period.end);
            (tree.outcome.is_none()
                || tree.outcome.is_some_and(|outcome| {
                    matches!(
                        outcome,
                        HarvestTreeOutcome::Deferred { retry_on, .. }
                            if retry_on <= action_date
                    )
                }))
                && !transaction.staged_harvest_tree_outcomes.iter().any(
                    |(run_id, staged_tree_id, _)| {
                        *run_id == harvest_run_id && *staged_tree_id == tree_id
                    },
                )
                && new_end >= effective_end
        });
        if !can_extend {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        }
        transaction
            .staged_harvest_period_extensions
            .push((harvest_run_id, tree_id, new_end));
        Ok(())
    }

    fn extend_orchard_harvest_window(
        &mut self,
        orchard_id: OrchardId,
        extension: &HarvestWindowExtension,
    ) -> Result<bool, OrchardStorageError> {
        let staged_windows = self.transaction.as_ref().and_then(|transaction| {
            transaction
                .staged_orchard_harvest_schedule_replacements
                .iter()
                .rev()
                .find(|(stored_orchard_id, owner, _)| {
                    *stored_orchard_id == orchard_id && *owner == extension.owner
                })
                .map(|(_, _, windows)| windows.clone())
        });
        let orchard = self.orchard.lock().unwrap();
        let schedule = staged_windows.as_ref().or_else(|| {
            orchard
                .orchard_harvest_schedules
                .iter()
                .find(|(stored_orchard_id, owner, _)| {
                    *stored_orchard_id == orchard_id && *owner == extension.owner
                })
                .map(|(_, _, windows)| windows)
                .or_else(|| {
                    orchard
                        .harvest_schedules
                        .iter()
                        .find(|(owner, _)| *owner == extension.owner)
                        .map(|(_, windows)| windows)
                })
        });
        let Some(mut windows) = schedule.cloned() else {
            return Ok(false);
        };
        let Some(window) = windows
            .iter_mut()
            .find(|window| **window == extension.current_window)
        else {
            return Ok(false);
        };
        window.end = extension.new_end;
        window.data_origin = HarvestDataOrigin::FieldObservation;
        window.source_url = None;
        drop(orchard);

        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        transaction
            .staged_orchard_harvest_schedule_replacements
            .retain(|(stored_orchard_id, owner, _)| {
                *stored_orchard_id != orchard_id || *owner != extension.owner
            });
        transaction
            .staged_orchard_harvest_schedule_replacements
            .push((orchard_id, extension.owner, windows));
        Ok(true)
    }

    fn restore_orchard_harvest_window(
        &mut self,
        orchard_id: OrchardId,
        extension: &HarvestWindowExtension,
    ) -> Result<bool, OrchardStorageError> {
        let staged_windows = self.transaction.as_ref().and_then(|transaction| {
            transaction
                .staged_orchard_harvest_schedule_replacements
                .iter()
                .rev()
                .find(|(stored_orchard_id, owner, _)| {
                    *stored_orchard_id == orchard_id && *owner == extension.owner
                })
                .map(|(_, _, windows)| windows.clone())
        });
        let orchard = self.orchard.lock().unwrap();
        let schedule = staged_windows.as_ref().or_else(|| {
            orchard
                .orchard_harvest_schedules
                .iter()
                .find(|(stored_orchard_id, owner, _)| {
                    *stored_orchard_id == orchard_id && *owner == extension.owner
                })
                .map(|(_, _, windows)| windows)
                .or_else(|| {
                    orchard
                        .harvest_schedules
                        .iter()
                        .find(|(owner, _)| *owner == extension.owner)
                        .map(|(_, windows)| windows)
                })
        });
        let Some(mut windows) = schedule.cloned() else {
            return Ok(false);
        };
        let mut extended_window = extension.current_window.clone();
        extended_window.end = extension.new_end;
        extended_window.data_origin = HarvestDataOrigin::FieldObservation;
        extended_window.source_url = None;
        let Some(window) = windows
            .iter_mut()
            .find(|window| **window == extended_window)
        else {
            return Ok(false);
        };
        *window = extension.current_window.clone();
        drop(orchard);

        let transaction = self
            .transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?;
        transaction
            .staged_orchard_harvest_schedule_replacements
            .retain(|(stored_orchard_id, owner, _)| {
                *stored_orchard_id != orchard_id || *owner != extension.owner
            });
        transaction
            .staged_orchard_harvest_schedule_replacements
            .push((orchard_id, extension.owner, windows));
        Ok(true)
    }

    fn save_harvest_tree_action_undo(
        &mut self,
        harvest_run_id: HarvestRunId,
        undo: &HarvestTreeActionUndo,
    ) -> Result<(), OrchardStorageError> {
        let can_save = self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .any(|run| run.id == harvest_run_id && !run.completed);
        if !can_save {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_harvest_tree_action_undos
            .push((harvest_run_id, undo.clone()));
        Ok(())
    }

    fn harvest_tree_action_undo(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<Option<HarvestTreeActionUndo>, OrchardStorageError> {
        Ok(self
            .orchard
            .lock()
            .unwrap()
            .harvest_tree_action_undos
            .iter()
            .find(|(stored_run_id, _)| *stored_run_id == harvest_run_id)
            .map(|(_, undo)| undo.clone()))
    }

    fn restore_harvest_tree_action(
        &mut self,
        harvest_run_id: HarvestRunId,
        undo: &HarvestTreeActionUndo,
    ) -> Result<(), OrchardStorageError> {
        let orchard = self.orchard.lock().unwrap();
        let Some(run) = orchard
            .harvest_runs
            .iter()
            .find(|run| run.id == harvest_run_id)
        else {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        };
        let action_is_latest =
            orchard
                .harvest_tree_action_undos
                .iter()
                .any(|(stored_run_id, stored_undo)| {
                    *stored_run_id == harvest_run_id && stored_undo == undo
                });
        let tree_matches = run.ordered_trees.iter().any(|tree| {
            tree.tree_id == undo.tree_id
                && tree.outcome == Some(undo.recorded_outcome)
                && tree.period.end == undo.recorded_period_end
        });
        let another_run_is_active = run.completed
            && orchard
                .harvest_runs
                .iter()
                .any(|other| other.orchard_id == run.orchard_id && !other.completed);
        drop(orchard);
        if !action_is_latest || !tree_matches || another_run_is_active {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_restored_harvest_tree_actions
            .push((harvest_run_id, undo.clone()));
        Ok(())
    }

    fn complete_harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<(), OrchardStorageError> {
        let can_complete = self
            .orchard
            .lock()
            .unwrap()
            .harvest_runs
            .iter()
            .any(|run| run.id == harvest_run_id && !run.completed);
        if !can_complete {
            return Err(OrchardStorageError::HarvestRunCouldNotBeChanged);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_completed_harvest_runs
            .push(harvest_run_id);
        Ok(())
    }

    fn delete_harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<(), OrchardStorageError> {
        let can_delete = self.orchard.lock().unwrap().harvest_runs.iter().any(|run| {
            run.id == harvest_run_id
                && !run.completed
                && run.ordered_trees.iter().all(|tree| tree.outcome.is_none())
        });
        if !can_delete {
            return Err(OrchardStorageError::HarvestRunCouldNotBeDeleted);
        }
        self.transaction
            .as_mut()
            .ok_or(OrchardStorageError::AtomicOperationCouldNotBegin)?
            .staged_deleted_harvest_runs
            .push(harvest_run_id);
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

fn harvest_outcome_fits_period(
    outcome: HarvestTreeOutcome,
    period: crate::hexagon::models::HarvestPeriod,
) -> bool {
    match outcome {
        HarvestTreeOutcome::HarvestedEverything { harvested_on } => {
            period.start <= harvested_on && harvested_on <= period.end
        }
        HarvestTreeOutcome::DoneForWindow { recorded_on } => {
            period.start <= recorded_on && recorded_on <= period.end
        }
        HarvestTreeOutcome::Deferred {
            deferred_on,
            retry_on,
        } => period.start <= deferred_on && deferred_on < retry_on && retry_on <= period.end,
    }
}

fn deferred_outcome_is_due(
    stored_outcome: HarvestTreeOutcome,
    new_outcome: HarvestTreeOutcome,
) -> bool {
    let action_date = match new_outcome {
        HarvestTreeOutcome::HarvestedEverything { harvested_on } => harvested_on,
        HarvestTreeOutcome::DoneForWindow { recorded_on } => recorded_on,
        HarvestTreeOutcome::Deferred { deferred_on, .. } => deferred_on,
    };
    matches!(
        stored_outcome,
        HarvestTreeOutcome::Deferred { retry_on, .. } if retry_on <= action_date
    )
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

    pub fn watering_run_exists(&self, watering_run_id: WateringRunId) -> bool {
        self.orchard
            .lock()
            .unwrap()
            .watering_runs
            .iter()
            .any(|run| run.id == watering_run_id)
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
