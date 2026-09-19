use crate::hexagon::models::{
    HarvestDate, HarvestRunId, HarvestScheduleOwner, HarvestTreeActionUndo, HarvestTreeOutcome,
    HarvestWindowExtension, HarvestedPart, OrchardId, OrchardTree, TreeId, concrete_harvest_period,
    current_harvest_part_periods, current_harvest_parts_and_period,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_harvest_run::{HarvestProgress, current_harvest_tree_index, harvest_progress};

pub struct HarvestTreeDeferred {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub action_date: String,
    pub extend_window: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HarvestWindowExtensionProposal {
    pub current_end: HarvestDate,
    pub proposed_end: HarvestDate,
}

#[derive(Debug, PartialEq)]
pub enum HarvestTreeDeferralError {
    InvalidActionDate,
    HarvestRunNotFound,
    HarvestRunAlreadyCompleted,
    TreeIsNotCurrent,
    ActionDateOutsideRunPeriod,
    HarvestWindowChanged,
    WindowExtensionRequired(HarvestWindowExtensionProposal),
    HarvestWindowCouldNotBeExtended,
    TreeCouldNotBeDeferred,
}

pub fn defer_harvest_tree(
    event: HarvestTreeDeferred,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, HarvestTreeDeferralError> {
    let deferred_on = HarvestDate::parse_iso(&event.action_date)
        .ok_or(HarvestTreeDeferralError::InvalidActionDate)?;
    let retry_on = deferred_on
        .add_days(7)
        .ok_or(HarvestTreeDeferralError::InvalidActionDate)?;
    storage.transaction(|orchard| {
        let mut run = orchard
            .harvest_run(event.harvest_run_id)
            .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(HarvestTreeDeferralError::HarvestRunNotFound)?;
        if run.completed {
            return Err(HarvestTreeDeferralError::HarvestRunAlreadyCompleted);
        }
        let current_index = current_harvest_tree_index(&run, deferred_on);
        if current_index.and_then(|index| run.ordered_trees.get(index).map(|tree| tree.tree_id))
            != Some(event.tree_id)
        {
            return Err(HarvestTreeDeferralError::TreeIsNotCurrent);
        }
        let current_index = current_index.expect("the current harvest tree was checked");
        let snapshot_period = run.ordered_trees[current_index].period;
        let snapshot_parts = run.ordered_trees[current_index].harvested_parts.clone();
        let previous_outcome = run.ordered_trees[current_index].outcome;
        if deferred_on < run.started_on || deferred_on < snapshot_period.start {
            return Err(HarvestTreeDeferralError::ActionDateOutsideRunPeriod);
        }
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
        let tree = orchard_trees
            .iter()
            .find(|tree| tree.id == event.tree_id)
            .ok_or(HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
        let live_harvest =
            current_harvest_parts_and_period(&tree.harvest_windows, &snapshot_parts, deferred_on)
                .filter(|(parts, period)| {
                    *parts == snapshot_parts && period.start == snapshot_period.start
                });
        let Some((_, live_period)) = live_harvest else {
            return Err(if deferred_on > snapshot_period.end {
                HarvestTreeDeferralError::ActionDateOutsideRunPeriod
            } else {
                HarvestTreeDeferralError::HarvestWindowChanged
            });
        };
        let part_periods =
            current_harvest_part_periods(&tree.harvest_windows, &snapshot_parts, deferred_on);
        let mut required_extensions = part_periods
            .iter()
            .filter(|(_, period)| period.end <= retry_on)
            .map(|(harvested_part, period)| {
                period
                    .end
                    .add_days(7)
                    .map(|proposed_end| (*harvested_part, period.end, proposed_end.max(retry_on)))
                    .ok_or(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)
            })
            .collect::<Result<Vec<_>, _>>()?;
        required_extensions
            .sort_by_key(|(harvested_part, current_end, _)| (*current_end, *harvested_part));
        if let Some((_, current_end, proposed_end)) = required_extensions.first() {
            let proposal = HarvestWindowExtensionProposal {
                current_end: *current_end,
                proposed_end: *proposed_end,
            };
            if event.extend_window.is_none() {
                return Err(HarvestTreeDeferralError::WindowExtensionRequired(proposal));
            }
        }
        let planned_extensions = if event.extend_window == Some(true) {
            required_extensions
                .iter()
                .map(|(harvested_part, current_end, proposed_end)| {
                    harvest_window_extension(tree, *harvested_part, *current_end, *proposed_end)
                        .map(|extension| (extension, *current_end, *proposed_end))
                        .ok_or(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };

        let declined_extension =
            !required_extensions.is_empty() && event.extend_window == Some(false);
        if !required_extensions.is_empty() && event.extend_window == Some(true) {
            for (extension, _, _) in &planned_extensions {
                let extended = orchard
                    .extend_orchard_harvest_window(event.orchard_id, extension)
                    .map_err(|_| HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?;
                if !extended {
                    return Err(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended);
                }
            }
            let extended_period_end = required_extensions
                .iter()
                .map(|(_, _, proposed_end)| *proposed_end)
                .chain(std::iter::once(live_period.end))
                .max()
                .expect("a live harvest has an end");
            if extended_period_end > snapshot_period.end {
                orchard
                    .extend_harvest_run_tree_period(
                        event.harvest_run_id,
                        event.tree_id,
                        extended_period_end,
                        deferred_on,
                    )
                    .map_err(|_| HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?;
                run.ordered_trees[current_index].period.end = extended_period_end;
            }
        } else if live_period.end > snapshot_period.end {
            orchard
                .extend_harvest_run_tree_period(
                    event.harvest_run_id,
                    event.tree_id,
                    live_period.end,
                    deferred_on,
                )
                .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
            run.ordered_trees[current_index].period.end = live_period.end;
        }

        let outcome = if declined_extension {
            HarvestTreeOutcome::DoneForWindow {
                recorded_on: deferred_on,
            }
        } else {
            HarvestTreeOutcome::Deferred {
                deferred_on,
                retry_on,
            }
        };
        orchard
            .record_harvest_tree_outcome(event.harvest_run_id, event.tree_id, outcome)
            .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
        run.ordered_trees[current_index].outcome = Some(outcome);
        orchard
            .save_harvest_tree_action_undo(
                event.harvest_run_id,
                &HarvestTreeActionUndo {
                    tree_id: event.tree_id,
                    previous_outcome,
                    previous_period_end: snapshot_period.end,
                    recorded_outcome: outcome,
                    recorded_period_end: run.ordered_trees[current_index].period.end,
                    window_extensions: planned_extensions
                        .into_iter()
                        .map(|(extension, _, _)| extension)
                        .collect(),
                },
            )
            .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
        if current_harvest_tree_index(&run, deferred_on).is_none() {
            orchard
                .complete_harvest_run(event.harvest_run_id)
                .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
            run.completed = true;
        }
        harvest_progress(&run, &orchard_trees, deferred_on)
            .ok_or(HarvestTreeDeferralError::TreeCouldNotBeDeferred)
    })
}

fn harvest_window_extension(
    tree: &OrchardTree,
    harvested_part: HarvestedPart,
    current_period_end: HarvestDate,
    proposed_end: HarvestDate,
) -> Option<HarvestWindowExtension> {
    let (current_window, start_year) = tree
        .harvest_windows
        .iter()
        .filter(|window| window.harvested_part == harvested_part)
        .find_map(|window| {
            [current_period_end.year - 1, current_period_end.year]
                .into_iter()
                .find(|year| {
                    concrete_harvest_period(window, *year)
                        .is_some_and(|period| period.end == current_period_end)
                })
                .map(|year| (window.clone(), year))
        })?;
    let mut extended_window = current_window.clone();
    extended_window.end = proposed_end.annual_date();
    if concrete_harvest_period(&extended_window, start_year)?.end != proposed_end {
        return None;
    }
    let owner = tree.tree.cultivar_id.map_or(
        HarvestScheduleOwner::PlantIdentity(tree.tree.plant_identity_id),
        HarvestScheduleOwner::PlantCultivar,
    );
    Some(HarvestWindowExtension {
        owner,
        current_window,
        new_end: proposed_end.annual_date(),
    })
}

impl From<OrchardStorageError> for HarvestTreeDeferralError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeDeferred
    }
}
