use crate::hexagon::models::{
    HarvestDate, HarvestRunId, HarvestScheduleOwner, HarvestTreeOutcome, HarvestWindowExtension,
    HarvestedPart, OrchardId, OrchardTree, TreeId, concrete_harvest_period,
    current_contiguous_fruit_period,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_harvest_run::{HarvestProgress, current_harvest_tree_index, harvest_progress};

pub struct HarvestTreeDeferred {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub action_date: String,
    pub extend_window: bool,
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
        let live_period = current_contiguous_fruit_period(&tree.harvest_windows, deferred_on)
            .filter(|period| period.start == snapshot_period.start);
        let Some(live_period) = live_period else {
            return Err(if deferred_on > snapshot_period.end {
                HarvestTreeDeferralError::ActionDateOutsideRunPeriod
            } else {
                HarvestTreeDeferralError::HarvestWindowChanged
            });
        };
        let current_period_end = live_period.end;
        let proposed_end = current_period_end
            .add_days(7)
            .ok_or(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?
            .max(retry_on);

        if current_period_end <= retry_on {
            let extension = harvest_window_extension(tree, current_period_end, proposed_end)
                .ok_or(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?;
            let proposal = HarvestWindowExtensionProposal {
                current_end: current_period_end,
                proposed_end,
            };
            if !event.extend_window {
                return Err(HarvestTreeDeferralError::WindowExtensionRequired(proposal));
            }
            let extended = orchard
                .extend_orchard_harvest_window(event.orchard_id, &extension)
                .map_err(|_| HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?;
            if !extended {
                return Err(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended);
            }
            if proposed_end > snapshot_period.end {
                orchard
                    .extend_harvest_run_tree_period(
                        event.harvest_run_id,
                        event.tree_id,
                        proposed_end,
                        deferred_on,
                    )
                    .map_err(|_| HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)?;
                run.ordered_trees[current_index].period.end = proposed_end;
            }
        } else if current_period_end > snapshot_period.end {
            orchard
                .extend_harvest_run_tree_period(
                    event.harvest_run_id,
                    event.tree_id,
                    current_period_end,
                    deferred_on,
                )
                .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
            run.ordered_trees[current_index].period.end = current_period_end;
        }

        let outcome = HarvestTreeOutcome::Deferred {
            deferred_on,
            retry_on,
        };
        orchard
            .record_harvest_tree_outcome(event.harvest_run_id, event.tree_id, outcome)
            .map_err(|_| HarvestTreeDeferralError::TreeCouldNotBeDeferred)?;
        run.ordered_trees[current_index].outcome = Some(outcome);
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
    current_period_end: HarvestDate,
    proposed_end: HarvestDate,
) -> Option<HarvestWindowExtension> {
    let (current_window, start_year) = tree
        .harvest_windows
        .iter()
        .filter(|window| window.harvested_part == HarvestedPart::Fruit)
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
