use crate::hexagon::models::{OrchardId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

pub struct WateringRunPauseRequested {
    pub orchard_id: OrchardId,
    pub watering_run_id: WateringRunId,
}

#[derive(Debug, PartialEq)]
pub enum WateringRunPauseError {
    WateringRunNotFound,
    WateringRunAlreadyCompleted,
    WateringRunCouldNotBePaused,
}

pub fn pause_watering_run(
    event: WateringRunPauseRequested,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringRunPauseError> {
    storage.transaction(|orchard| {
        let mut run = orchard
            .watering_run(event.watering_run_id)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(WateringRunPauseError::WateringRunNotFound)?;
        if run.completed {
            return Err(WateringRunPauseError::WateringRunAlreadyCompleted);
        }
        orchard.set_watering_run_paused(run.id, true)?;
        run.paused = true;
        let trees = orchard.trees_in_orchard(event.orchard_id)?;
        watering_progress(&run, &trees).ok_or(WateringRunPauseError::WateringRunCouldNotBePaused)
    })
}

impl From<OrchardStorageError> for WateringRunPauseError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBePaused
    }
}
