use crate::hexagon::models::{OrchardId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

pub struct WateringRunResumeRequested {
    pub orchard_id: OrchardId,
    pub watering_run_id: WateringRunId,
}

#[derive(Debug, PartialEq)]
pub enum WateringRunResumeError {
    WateringRunNotFound,
    WateringRunAlreadyCompleted,
    AnotherWateringRunIsActive,
    HarvestRunIsActive,
    WateringRunCouldNotBeResumed,
}

pub fn resume_watering_run(
    event: WateringRunResumeRequested,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringRunResumeError> {
    storage.transaction(|orchard| {
        let mut run = orchard
            .watering_run(event.watering_run_id)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(WateringRunResumeError::WateringRunNotFound)?;
        if run.completed {
            return Err(WateringRunResumeError::WateringRunAlreadyCompleted);
        }
        if orchard.active_harvest_run(event.orchard_id)?.is_some() {
            return Err(WateringRunResumeError::HarvestRunIsActive);
        }
        if orchard
            .active_watering_run(event.orchard_id)?
            .is_some_and(|active| active.id != run.id)
        {
            return Err(WateringRunResumeError::AnotherWateringRunIsActive);
        }
        orchard.set_watering_run_paused(run.id, false)?;
        run.paused = false;
        let trees = orchard.trees_in_orchard(event.orchard_id)?;
        watering_progress(&run, &trees).ok_or(WateringRunResumeError::WateringRunCouldNotBeResumed)
    })
}

impl From<OrchardStorageError> for WateringRunResumeError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeResumed
    }
}
