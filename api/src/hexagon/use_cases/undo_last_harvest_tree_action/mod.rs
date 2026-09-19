use crate::hexagon::models::{HarvestDate, HarvestRunId, OrchardId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_harvest_run::{HarvestProgress, harvest_progress};

pub struct LastHarvestTreeActionUndone {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub action_date: String,
}

#[derive(Debug, PartialEq)]
pub enum LastHarvestTreeActionUndoError {
    InvalidActionDate,
    HarvestRunNotFound,
    NoHarvestTreeActionToUndo,
    HarvestTreeActionCouldNotBeUndone,
}

pub fn undo_last_harvest_tree_action(
    event: LastHarvestTreeActionUndone,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, LastHarvestTreeActionUndoError> {
    let action_date = HarvestDate::parse_iso(&event.action_date)
        .ok_or(LastHarvestTreeActionUndoError::InvalidActionDate)?;
    storage.transaction(|orchard| {
        let mut run = orchard
            .harvest_run(event.harvest_run_id)
            .map_err(|_| LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(LastHarvestTreeActionUndoError::HarvestRunNotFound)?;
        if action_date < run.started_on {
            return Err(LastHarvestTreeActionUndoError::InvalidActionDate);
        }
        let undo = orchard
            .harvest_tree_action_undo(event.harvest_run_id)
            .map_err(|_| LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?
            .ok_or(LastHarvestTreeActionUndoError::NoHarvestTreeActionToUndo)?;

        for extension in undo.window_extensions.iter().rev() {
            let restored = orchard
                .restore_orchard_harvest_window(event.orchard_id, extension)
                .map_err(|_| LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?;
            if !restored {
                return Err(LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone);
            }
        }
        orchard
            .restore_harvest_tree_action(event.harvest_run_id, &undo)
            .map_err(|_| LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?;

        let run_tree = run
            .ordered_trees
            .iter_mut()
            .find(|tree| tree.tree_id == undo.tree_id)
            .ok_or(LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?;
        run_tree.outcome = undo.previous_outcome;
        run_tree.period.end = undo.previous_period_end;
        run.completed = false;
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)?;
        harvest_progress(&run, &orchard_trees, action_date)
            .ok_or(LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)
    })
}

impl From<OrchardStorageError> for LastHarvestTreeActionUndoError {
    fn from(_: OrchardStorageError) -> Self {
        Self::HarvestTreeActionCouldNotBeUndone
    }
}
