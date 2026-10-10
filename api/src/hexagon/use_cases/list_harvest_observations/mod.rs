use crate::hexagon::models::{HarvestObservation, HarvestScheduleOwner, OrchardId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn list_harvest_observations(
    orchard_id: OrchardId,
    owner: HarvestScheduleOwner,
    storage: &mut impl OrchardStorage,
) -> Result<Vec<HarvestObservation>, OrchardStorageError> {
    storage
        .harvest_observations(orchard_id, owner)
        .map(|points| {
            points
                .into_iter()
                .filter(HarvestObservation::has_timing_evidence)
                .collect()
        })
}
