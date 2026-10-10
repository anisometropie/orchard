use crate::hexagon::models::{HarvestObservation, OrchardId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};
pub fn list_orchard_harvest_observations(
    orchard_id: OrchardId,
    storage: &mut impl OrchardStorage,
) -> Result<Vec<HarvestObservation>, OrchardStorageError> {
    storage.harvest_observations_in_orchard(orchard_id)
}
