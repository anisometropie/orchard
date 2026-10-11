use crate::hexagon::models::{
    HarvestDate, HarvestFruitState, HarvestObservation, HarvestObservationKind, HarvestRunId,
    HarvestScheduleOwner, HarvestTreeActionUndo, HarvestTreeOutcome, OrchardId, TreeId,
    current_harvest_parts_and_period,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_harvest_run::{HarvestProgress, current_harvest_tree_index, harvest_progress};

pub struct HarvestVisitRecorded {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub action_date: String,
    pub kind: HarvestObservationKind,
    pub fruit_state: Option<HarvestFruitState>,
}

#[derive(Debug, PartialEq)]
pub enum HarvestVisitRecordingError {
    InvalidActionDate,
    InvalidFruitState,
    HarvestRunNotFound,
    HarvestRunAlreadyCompleted,
    TreeIsNotCurrent,
    ActionDateOutsideRunPeriod,
    HarvestWindowChanged,
    TreeCouldNotBeRecorded,
}

pub fn record_harvest_visit(
    event: HarvestVisitRecorded,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, HarvestVisitRecordingError> {
    let recorded_on = HarvestDate::parse_iso(&event.action_date)
        .ok_or(HarvestVisitRecordingError::InvalidActionDate)?;
    let fruit_state = event
        .kind
        .fruit_state(event.fruit_state)
        .map_err(|_| HarvestVisitRecordingError::InvalidFruitState)?;
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let mut run = orchard
            .harvest_run(event.harvest_run_id)
            .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(HarvestVisitRecordingError::HarvestRunNotFound)?;
        if run.completed {
            return Err(HarvestVisitRecordingError::HarvestRunAlreadyCompleted);
        }
        let current_index = current_harvest_tree_index(&run, recorded_on);
        if current_index.and_then(|index| run.ordered_trees.get(index).map(|tree| tree.tree_id))
            != Some(event.tree_id)
        {
            return Err(HarvestVisitRecordingError::TreeIsNotCurrent);
        }
        let current_index = current_index.expect("the current harvest tree was checked");
        let snapshot_period = run.ordered_trees[current_index].period;
        let snapshot_parts = run.ordered_trees[current_index].harvested_parts.clone();
        let previous_outcome = run.ordered_trees[current_index].outcome;
        if recorded_on < run.started_on || recorded_on < snapshot_period.start {
            return Err(HarvestVisitRecordingError::ActionDateOutsideRunPeriod);
        }
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
        let tree = orchard_trees
            .iter()
            .find(|tree| tree.id == event.tree_id)
            .ok_or(HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
        let live_harvest =
            current_harvest_parts_and_period(&tree.harvest_windows, &snapshot_parts, recorded_on)
                .filter(|(parts, period)| {
                    *parts == snapshot_parts && period.start == snapshot_period.start
                });
        let Some((_, live_period)) = live_harvest else {
            return Err(if recorded_on > snapshot_period.end {
                HarvestVisitRecordingError::ActionDateOutsideRunPeriod
            } else {
                HarvestVisitRecordingError::HarvestWindowChanged
            });
        };
        if live_period.end > snapshot_period.end {
            orchard
                .extend_harvest_run_tree_period(
                    event.harvest_run_id,
                    event.tree_id,
                    live_period.end,
                    recorded_on,
                )
                .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
            run.ordered_trees[current_index].period.end = live_period.end;
        }

        let outcome = match event.kind {
            HarvestObservationKind::TooSoon => HarvestTreeOutcome::TooSoon {
                recorded_on,
                retry_on: recorded_on
                    .add_days(7)
                    .ok_or(HarvestVisitRecordingError::InvalidActionDate)?,
            },
            HarvestObservationKind::FruitRemaining => HarvestTreeOutcome::FruitRemaining {
                recorded_on,
                retry_on: recorded_on
                    .add_days(7)
                    .ok_or(HarvestVisitRecordingError::InvalidActionDate)?,
            },
            HarvestObservationKind::EmptyOnArrival => {
                HarvestTreeOutcome::EmptyOnArrival { recorded_on }
            }
            HarvestObservationKind::HarvestedAll => HarvestTreeOutcome::HarvestedEverything {
                harvested_on: recorded_on,
            },
            HarvestObservationKind::AllWasted | HarvestObservationKind::TooLate => {
                HarvestTreeOutcome::TooLate { recorded_on }
            }
        };
        let owner = tree.tree.cultivar_id.map_or(
            HarvestScheduleOwner::PlantIdentity(tree.tree.plant_identity_id),
            HarvestScheduleOwner::PlantCultivar,
        );
        let observation_id = orchard.save_harvest_observation(HarvestObservation {
            id: 0,
            orchard_id: event.orchard_id,
            harvest_run_id: Some(event.harvest_run_id),
            tree_id: event.tree_id,
            owner,
            recorded_on,
            kind: event.kind,
            fruit_state,
            harvested_parts: snapshot_parts,
            period: Some(run.ordered_trees[current_index].period),
            reviewed: false,
        })?;
        orchard
            .record_harvest_tree_outcome(event.harvest_run_id, event.tree_id, outcome)
            .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
        run.ordered_trees[current_index].outcome = Some(outcome);
        orchard
            .save_harvest_tree_action_undo(
                event.harvest_run_id,
                &HarvestTreeActionUndo {
                    observation_id: Some(observation_id),
                    tree_id: event.tree_id,
                    previous_outcome,
                    previous_period_end: snapshot_period.end,
                    recorded_outcome: outcome,
                    recorded_period_end: run.ordered_trees[current_index].period.end,
                    window_extensions: vec![],
                },
            )
            .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
        if current_harvest_tree_index(&run, recorded_on).is_none() {
            orchard
                .complete_harvest_run(event.harvest_run_id)
                .map_err(|_| HarvestVisitRecordingError::TreeCouldNotBeRecorded)?;
            run.completed = true;
        }
        harvest_progress(&run, &orchard_trees, recorded_on)
            .ok_or(HarvestVisitRecordingError::TreeCouldNotBeRecorded)
    })
}

impl From<OrchardStorageError> for HarvestVisitRecordingError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeRecorded
    }
}
