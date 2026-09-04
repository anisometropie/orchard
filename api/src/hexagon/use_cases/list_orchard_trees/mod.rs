use crate::hexagon::models::{OrchardId, OrchardTree};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn list_orchard_trees(
    orchard_storage: &mut impl OrchardStorage,
) -> Result<Vec<OrchardTree>, OrchardStorageError> {
    orchard_storage.trees()
}

pub fn list_trees_for_orchard(
    orchard_id: OrchardId,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<Vec<OrchardTree>, OrchardStorageError> {
    orchard_storage.trees_in_orchard(orchard_id)
}
