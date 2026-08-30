use crate::hexagon::models::{
    LegacyTreeSource, PlantIdentification, PlantIdentityReference, ReproductiveRole, Tree,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct LegacyTreeSnapshot {
    pub legacy_source: LegacyTreeSource,
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identification: PlantIdentification,
    pub planted_on: Option<String>,
    pub row_name: String,
    pub is_pioneer: bool,
    pub is_alive: bool,
    pub reproductive_role: Option<ReproductiveRole>,
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
    U: OrchardStorage,
{
    orchard_storage.transaction(|orchard| {
        let mut imported_tree_count = 0;
        for legacy_tree in request.trees {
            let legacy_feature_id = legacy_tree.legacy_source.feature_id;
            match orchard.is_legacy_tree_already_imported(legacy_feature_id) {
                Ok(true) => {
                    return Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
                        legacy_feature_id,
                    });
                }
                Ok(false) => {}
                Err(_) => {
                    return Err(LegacyOrchardImportError::ExistingLegacyFeaturesCouldNotBeChecked);
                }
            }
            let identification_status = legacy_tree.plant_identification.identification_status;
            let plant_identity = orchard
                .resolve_plant_identification(legacy_tree.plant_identification.clone())
                .map_err(
                    |_| LegacyOrchardImportError::PlantIdentityCouldNotBeResolved {
                        legacy_feature_id,
                    },
                )?;
            let tree = map_legacy_tree(legacy_tree, plant_identity, identification_status);
            orchard
                .save_tree(tree)
                .map_err(|_| LegacyOrchardImportError::TreeCouldNotBeSaved { legacy_feature_id })?;
            imported_tree_count += 1;
        }
        Ok(imported_tree_count)
    })
}

impl From<OrchardStorageError> for LegacyOrchardImportError {
    fn from(error: OrchardStorageError) -> Self {
        match error {
            OrchardStorageError::AtomicOperationCouldNotBegin => Self::TransactionCouldNotBegin,
            OrchardStorageError::AtomicOperationCouldNotCommit => Self::TransactionCouldNotCommit,
            OrchardStorageError::ExistingLegacyTreeCouldNotBeChecked => {
                Self::ExistingLegacyFeaturesCouldNotBeChecked
            }
            _ => Self::ExistingLegacyFeaturesCouldNotBeChecked,
        }
    }
}

fn map_legacy_tree(
    legacy_tree: LegacyTreeSnapshot,
    plant_identity: PlantIdentityReference,
    identification_status: crate::hexagon::models::IdentificationStatus,
) -> Tree {
    Tree {
        legacy_source: Some(legacy_tree.legacy_source),
        plant_identity_id: plant_identity.plant_identity_id,
        cultivar_id: plant_identity.cultivar_id,
        identification_status,
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
        is_in_danger: false,
        reproductive_role: legacy_tree.reproductive_role,
        adult_height_meters: Some(legacy_tree.adult_height_meters),
        adult_width_meters: Some(legacy_tree.adult_width_meters),
    }
}
