use crate::hexagon::models::{OrchardId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct WateringRunCancellationRequested {
    pub orchard_id: OrchardId,
    pub watering_run_id: WateringRunId,
}

#[derive(Debug, PartialEq)]
pub enum WateringRunCancellationError {
    WateringRunNotFound,
    WateringRunAlreadyCompleted,
    WateringRunCouldNotBeCancelled,
}

pub fn cancel_watering_run(
    event: WateringRunCancellationRequested,
    storage: &mut impl OrchardStorage,
) -> Result<(), WateringRunCancellationError> {
    storage.transaction(|orchard| {
        let run = orchard
            .watering_run(event.watering_run_id)
            .map_err(|_| WateringRunCancellationError::WateringRunCouldNotBeCancelled)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(WateringRunCancellationError::WateringRunNotFound)?;
        if run.completed {
            return Err(WateringRunCancellationError::WateringRunAlreadyCompleted);
        }
        orchard
            .delete_watering_run(event.watering_run_id)
            .map_err(|_| WateringRunCancellationError::WateringRunCouldNotBeCancelled)
    })
}

impl From<OrchardStorageError> for WateringRunCancellationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeCancelled
    }
}
