use crate::hexagon::models::OrchardId;
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

#[derive(Debug, PartialEq)]
pub enum WateringRunsListError {
    WateringRunsCouldNotBeLoaded,
}

pub fn list_watering_runs(
    orchard_id: OrchardId,
    storage: &mut impl OrchardStorage,
) -> Result<Vec<WateringProgress>, WateringRunsListError> {
    let runs = storage.unfinished_watering_runs(orchard_id)?;
    let trees = storage.trees_in_orchard(orchard_id)?;
    runs.iter()
        .map(|run| {
            watering_progress(run, &trees)
                .ok_or(WateringRunsListError::WateringRunsCouldNotBeLoaded)
        })
        .collect()
}

impl From<OrchardStorageError> for WateringRunsListError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunsCouldNotBeLoaded
    }
}
