use crate::hexagon::models::{
    HarvestDate, HarvestRunId, HarvestTreeOutcome, OrchardId, TreeId,
    current_contiguous_fruit_period,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_harvest_run::{HarvestProgress, current_harvest_tree_index, harvest_progress};

pub struct TreeHarvestedEverything {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub action_date: String,
}

#[derive(Debug, PartialEq)]
pub enum TreeHarvestedEverythingError {
    InvalidActionDate,
    HarvestRunNotFound,
    HarvestRunAlreadyCompleted,
    TreeIsNotCurrent,
    ActionDateOutsideRunPeriod,
    HarvestWindowChanged,
    TreeCouldNotBeRecorded,
}

pub fn record_tree_harvested(
    event: TreeHarvestedEverything,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, TreeHarvestedEverythingError> {
    let harvested_on = HarvestDate::parse_iso(&event.action_date)
        .ok_or(TreeHarvestedEverythingError::InvalidActionDate)?;
    storage.transaction(|orchard| {
        let mut run = orchard
            .harvest_run(event.harvest_run_id)
            .map_err(|_| TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(TreeHarvestedEverythingError::HarvestRunNotFound)?;
        if run.completed {
            return Err(TreeHarvestedEverythingError::HarvestRunAlreadyCompleted);
        }
        let current_index = current_harvest_tree_index(&run, harvested_on);
        if current_index.and_then(|index| run.ordered_trees.get(index).map(|tree| tree.tree_id))
            != Some(event.tree_id)
        {
            return Err(TreeHarvestedEverythingError::TreeIsNotCurrent);
        }
        let current_index = current_index.expect("the current harvest tree was checked");
        let snapshot_period = run.ordered_trees[current_index].period;
        if harvested_on < run.started_on || harvested_on < snapshot_period.start {
            return Err(TreeHarvestedEverythingError::ActionDateOutsideRunPeriod);
        }
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?;
        let tree = orchard_trees
            .iter()
            .find(|tree| tree.id == event.tree_id)
            .ok_or(TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?;
        let live_period = current_contiguous_fruit_period(&tree.harvest_windows, harvested_on)
            .filter(|period| period.start == snapshot_period.start);
        let Some(live_period) = live_period else {
            return Err(if harvested_on > snapshot_period.end {
                TreeHarvestedEverythingError::ActionDateOutsideRunPeriod
            } else {
                TreeHarvestedEverythingError::HarvestWindowChanged
            });
        };
        if live_period.end > snapshot_period.end {
            orchard
                .extend_harvest_run_tree_period(
                    event.harvest_run_id,
                    event.tree_id,
                    live_period.end,
                    harvested_on,
                )
                .map_err(|_| TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?;
            run.ordered_trees[current_index].period.end = live_period.end;
        }

        let outcome = HarvestTreeOutcome::HarvestedEverything { harvested_on };
        orchard
            .record_harvest_tree_outcome(event.harvest_run_id, event.tree_id, outcome)
            .map_err(|_| TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?;
        run.ordered_trees[current_index].outcome = Some(outcome);
        if current_harvest_tree_index(&run, harvested_on).is_none() {
            orchard
                .complete_harvest_run(event.harvest_run_id)
                .map_err(|_| TreeHarvestedEverythingError::TreeCouldNotBeRecorded)?;
            run.completed = true;
        }
        harvest_progress(&run, &orchard_trees, harvested_on)
            .ok_or(TreeHarvestedEverythingError::TreeCouldNotBeRecorded)
    })
}

impl From<OrchardStorageError> for TreeHarvestedEverythingError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeRecorded
    }
}
