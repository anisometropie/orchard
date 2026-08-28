use crate::hexagon::models::{
    LegacyTreeSource, PlantIdentity, PlantIdentityId, ReproductiveRole, Tree,
};
use crate::hexagon::ports::{OrchardTransaction, OrchardUnitOfWork};

pub struct LegacyTreeSnapshot {
    pub legacy_source: LegacyTreeSource,
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identity: PlantIdentity,
    pub planted_on: Option<String>,
    pub row_name: String,
    pub is_pioneer: bool,
    pub is_alive: bool,
    pub reproductive_role: Option<ReproductiveRole>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
    pub adult_height_meters: f64,
    pub adult_width_meters: f64,
}

pub struct LegacyOrchardImportRequested {
    pub trees: Vec<LegacyTreeSnapshot>,
}

#[derive(Debug, PartialEq)]
pub enum LegacyOrchardImportError {
    PlantIdentityCouldNotBeResolved { legacy_feature_id: u32 },
    TreeCouldNotBeSaved { legacy_feature_id: u32 },
    LegacyFeatureAlreadyImported { legacy_feature_id: u32 },
    ExistingLegacyFeaturesCouldNotBeChecked,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn import_legacy_orchard<U>(
    request: LegacyOrchardImportRequested,
    orchard_storage: &mut U,
) -> Result<usize, LegacyOrchardImportError>
where
    U: OrchardUnitOfWork,
{
    let mut transaction = orchard_storage
        .begin()
        .map_err(|_| LegacyOrchardImportError::TransactionCouldNotBegin)?;
    let mut imported_tree_count = 0;
    for legacy_tree in request.trees {
        let legacy_feature_id = legacy_tree.legacy_source.feature_id;
        match transaction.is_legacy_tree_already_imported(legacy_feature_id) {
            Ok(true) => {
                transaction.rollback();
                return Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
                    legacy_feature_id,
                });
            }
            Ok(false) => {}
            Err(_) => {
                transaction.rollback();
                return Err(LegacyOrchardImportError::ExistingLegacyFeaturesCouldNotBeChecked);
            }
        }
        let plant_identity_id =
            match transaction.find_or_create_plant_identity(legacy_tree.plant_identity.clone()) {
                Ok(plant_identity_id) => plant_identity_id,
                Err(_) => {
                    transaction.rollback();
                    return Err(LegacyOrchardImportError::PlantIdentityCouldNotBeResolved {
                        legacy_feature_id,
                    });
                }
            };
        let tree = map_legacy_tree(legacy_tree, plant_identity_id);
        match transaction.save_tree(tree) {
            Ok(()) => {
                imported_tree_count += 1;
            }
            Err(_) => {
                transaction.rollback();
                return Err(LegacyOrchardImportError::TreeCouldNotBeSaved { legacy_feature_id });
            }
        }
    }
    transaction
        .commit()
        .map_err(|_| LegacyOrchardImportError::TransactionCouldNotCommit)?;
    Ok(imported_tree_count)
}

fn map_legacy_tree(legacy_tree: LegacyTreeSnapshot, plant_identity_id: PlantIdentityId) -> Tree {
    Tree {
        legacy_source: Some(legacy_tree.legacy_source),
        plant_identity_id,
        longitude: legacy_tree.longitude,
        latitude: legacy_tree.latitude,
        planted_on: legacy_tree.planted_on,
        row_name: Some(legacy_tree.row_name),
        roles: if legacy_tree.is_pioneer {
            vec!["pioneer".into()]
        } else {
            vec![]
        },
        is_alive: legacy_tree.is_alive,
        reproductive_role: legacy_tree.reproductive_role,
        harvest_start_day: legacy_tree.harvest_start_day,
        harvest_end_day: legacy_tree.harvest_end_day,
        adult_height_meters: Some(legacy_tree.adult_height_meters),
        adult_width_meters: Some(legacy_tree.adult_width_meters),
    }
}
