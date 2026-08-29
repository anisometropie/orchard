use crate::hexagon::models::OrchardTree;
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn list_orchard_trees(
    orchard_storage: &mut impl OrchardStorage,
) -> Result<Vec<OrchardTree>, OrchardStorageError> {
    orchard_storage.trees()
}
