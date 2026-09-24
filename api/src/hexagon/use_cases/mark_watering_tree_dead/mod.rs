use crate::hexagon::models::{OrchardId, TreeId, WateringRunId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

pub struct WateringTreeMarkedDead {
    pub orchard_id: OrchardId,
    pub watering_run_id: WateringRunId,
    pub tree_id: TreeId,
}

#[derive(Debug, PartialEq)]
pub enum WateringTreeMarkedDeadError {
    WateringRunNotFound,
    WateringRunAlreadyCompleted,
    WateringRunIsPaused,
    TreeIsNotNext,
    TreeCouldNotBeMarkedDead,
}

pub fn mark_watering_tree_dead(
    event: WateringTreeMarkedDead,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringTreeMarkedDeadError> {
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let mut run_ids = orchard
            .unfinished_watering_runs(event.orchard_id)?
            .into_iter()
            .filter(|run| run.tree_is_pending(event.tree_id))
            .map(|run| run.id)
            .collect::<Vec<_>>();
        run_ids.push(event.watering_run_id);
        run_ids.sort_by_key(|id| id.0);
        run_ids.dedup();
        // Read locked snapshots in ID order: a worker may have advanced after the list read.
        let mut runs = Vec::new();
        for id in run_ids {
            if let Some(run) = orchard
                .watering_run(id)?
                .filter(|run| run.orchard_id == event.orchard_id)
            {
                runs.push(run);
            }
        }
        let selected = runs
            .iter()
            .find(|run| run.id == event.watering_run_id)
            .ok_or(WateringTreeMarkedDeadError::WateringRunNotFound)?;
        if selected.completed {
            return Err(WateringTreeMarkedDeadError::WateringRunAlreadyCompleted);
        }
        if selected.paused {
            return Err(WateringTreeMarkedDeadError::WateringRunIsPaused);
        }
        if selected.next_tree_id() != Some(event.tree_id)
            || !orchard.tree_belongs_to_orchard(event.tree_id, event.orchard_id)?
        {
            return Err(WateringTreeMarkedDeadError::TreeIsNotNext);
        }
        orchard.change_tree_danger(event.tree_id, false)?;
        orchard.change_tree_life_status(event.tree_id, false)?;
        for run in &mut runs {
            if run.completed || !run.tree_is_pending(event.tree_id) {
                continue;
            }
            orchard.mark_watering_tree_skipped(run.id, event.tree_id)?;
            run.skipped_tree_ids.push(event.tree_id);
            if run.is_finished() {
                orchard.complete_watering_run(run.id)?;
                run.completed = true;
                run.paused = false;
            }
        }
        let selected = runs
            .iter()
            .find(|run| run.id == event.watering_run_id)
            .expect("the selected watering run was validated before writing");
        let trees = orchard.trees_in_orchard(event.orchard_id)?;
        watering_progress(selected, &trees)
            .ok_or(WateringTreeMarkedDeadError::TreeCouldNotBeMarkedDead)
    })
}

impl From<OrchardStorageError> for WateringTreeMarkedDeadError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeMarkedDead
    }
}
