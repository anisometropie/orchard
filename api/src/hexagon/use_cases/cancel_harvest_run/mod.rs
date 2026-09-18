use crate::hexagon::models::{HarvestRunId, OrchardId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct HarvestRunCancellationRequested {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
}

#[derive(Debug, PartialEq)]
pub enum HarvestRunCancellationError {
    HarvestRunNotFound,
    HarvestRunAlreadyCompleted,
    HarvestRunCouldNotBeCancelled,
}

pub fn cancel_harvest_run(
    event: HarvestRunCancellationRequested,
    storage: &mut impl OrchardStorage,
) -> Result<(), HarvestRunCancellationError> {
    storage.transaction(|orchard| {
        let run = orchard
            .harvest_run(event.harvest_run_id)
            .map_err(|_| HarvestRunCancellationError::HarvestRunCouldNotBeCancelled)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(HarvestRunCancellationError::HarvestRunNotFound)?;
        if run.completed {
            return Err(HarvestRunCancellationError::HarvestRunAlreadyCompleted);
        }
        if run.ordered_trees.iter().any(|tree| tree.outcome.is_some()) {
            orchard
                .complete_harvest_run(event.harvest_run_id)
                .map_err(|_| HarvestRunCancellationError::HarvestRunCouldNotBeCancelled)
        } else {
            orchard
                .delete_harvest_run(event.harvest_run_id)
                .map_err(|_| HarvestRunCancellationError::HarvestRunCouldNotBeCancelled)
        }
    })
}

impl From<OrchardStorageError> for HarvestRunCancellationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::HarvestRunCouldNotBeCancelled
    }
}
