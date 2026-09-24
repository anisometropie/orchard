use crate::hexagon::models::{OrchardId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

#[derive(Debug, PartialEq)]
pub enum WateringRunLoadError {
    WateringRunNotFound,
    WateringRunCouldNotBeLoaded,
}

pub fn load_watering_run(
    orchard_id: OrchardId,
    run_id: WateringRunId,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringRunLoadError> {
    let run = storage
        .watering_run(run_id)?
        .filter(|run| run.orchard_id == orchard_id)
        .ok_or(WateringRunLoadError::WateringRunNotFound)?;
    let trees = storage.trees_in_orchard(orchard_id)?;
    watering_progress(&run, &trees).ok_or(WateringRunLoadError::WateringRunCouldNotBeLoaded)
}

impl From<OrchardStorageError> for WateringRunLoadError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeLoaded
    }
}
