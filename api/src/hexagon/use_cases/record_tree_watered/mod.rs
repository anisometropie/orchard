use crate::hexagon::models::{OrchardId, TreeId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

pub struct TreeWatered {
    pub orchard_id: OrchardId,
    pub watering_run_id: WateringRunId,
    pub tree_id: TreeId,
}

#[derive(Debug, PartialEq)]
pub enum TreeWateredError {
    WateringRunNotFound,
    WateringRunAlreadyCompleted,
    WateringRunIsPaused,
    TreeIsNotNext,
    TreeCouldNotBeRecorded,
}

pub fn record_tree_watered(
    event: TreeWatered,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, TreeWateredError> {
    storage.transaction(|orchard| {
        let mut run = orchard
            .watering_run(event.watering_run_id)
            .map_err(|_| TreeWateredError::TreeCouldNotBeRecorded)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(TreeWateredError::WateringRunNotFound)?;
        if run.completed {
            return Err(TreeWateredError::WateringRunAlreadyCompleted);
        }
        if run.paused {
            return Err(TreeWateredError::WateringRunIsPaused);
        }
        let next_tree_id = run.next_tree_id();
        if next_tree_id != Some(event.tree_id) {
            return Err(TreeWateredError::TreeIsNotNext);
        }
        orchard
            .mark_watering_tree_watered(event.watering_run_id, event.tree_id)
            .map_err(|_| TreeWateredError::TreeCouldNotBeRecorded)?;
        run.watered_tree_ids.push(event.tree_id);
        if run.is_finished() {
            orchard
                .complete_watering_run(event.watering_run_id)
                .map_err(|_| TreeWateredError::TreeCouldNotBeRecorded)?;
            run.completed = true;
        }
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| TreeWateredError::TreeCouldNotBeRecorded)?;
        watering_progress(&run, &orchard_trees).ok_or(TreeWateredError::TreeCouldNotBeRecorded)
    })
}

impl From<OrchardStorageError> for TreeWateredError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeRecorded
    }
}
