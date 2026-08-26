use crate::hexagon::models::Tree;
use crate::hexagon::ports::{OrchardImportTransaction, OrchardUnitOfWork};

pub struct LegacyTreeSnapshot {
    pub legacy_feature_id: u32,
    pub longitude: f64,
    pub latitude: f64,
    pub name: String,
    pub latin_name: String,
    pub planted_on: Option<String>,
    pub row_name: String,
    pub is_pioneer: bool,
    pub is_alive: bool,
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
    TreeCouldNotBeSaved { legacy_feature_id: u32 },
    LegacyFeatureAlreadyImported { legacy_feature_id: u32 },
    ExistingLegacyFeaturesCouldNotBeChecked,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn import_legacy_orchard<U>(
    request: LegacyOrchardImportRequested,
    orchard_unit_of_work: &mut U,
) -> Result<usize, LegacyOrchardImportError>
where
    U: OrchardUnitOfWork,
{
    let mut transaction = orchard_unit_of_work
        .begin()
        .map_err(|_| LegacyOrchardImportError::TransactionCouldNotBegin)?;
    let mut imported_tree_count = 0;
    for legacy_tree in request.trees {
        let legacy_feature_id = legacy_tree.legacy_feature_id;
        match transaction.has_legacy_feature_id(legacy_feature_id) {
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
        if transaction.save_tree(map_legacy_tree(legacy_tree)).is_err() {
            transaction.rollback();
            return Err(LegacyOrchardImportError::TreeCouldNotBeSaved { legacy_feature_id });
        }
        imported_tree_count += 1;
    }
    transaction
        .commit()
        .map_err(|_| LegacyOrchardImportError::TransactionCouldNotCommit)?;
    Ok(imported_tree_count)
}

fn map_legacy_tree(legacy_tree: LegacyTreeSnapshot) -> Tree {
    Tree {
        legacy_feature_id: Some(legacy_tree.legacy_feature_id),
        longitude: legacy_tree.longitude,
        latitude: legacy_tree.latitude,
        name: legacy_tree.name,
        latin_name: Some(legacy_tree.latin_name),
        planted_on: legacy_tree.planted_on,
        row_name: Some(legacy_tree.row_name),
        roles: if legacy_tree.is_pioneer {
            vec!["pioneer".into()]
        } else {
            vec![]
        },
        is_alive: legacy_tree.is_alive,
        harvest_start_day: legacy_tree.harvest_start_day,
        harvest_end_day: legacy_tree.harvest_end_day,
        adult_height_meters: Some(legacy_tree.adult_height_meters),
        adult_width_meters: Some(legacy_tree.adult_width_meters),
    }
}
