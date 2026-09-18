use crate::hexagon::models::{
    EligibleHarvestTree, HarvestDate, HarvestPeriod, HarvestRun, HarvestRunId, HarvestRunTarget,
    HarvestRunTree, HarvestTreeOutcome, HarvestedPart, OrchardId, OrchardTree, PlantIdentityId,
    TreeId, eligible_harvest_trees, normalized_harvested_parts,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct HarvestRunStartRequested {
    pub orchard_id: OrchardId,
    pub target: HarvestRunTarget,
    pub harvested_parts: Vec<HarvestedPart>,
    pub action_date: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarvestTree {
    pub id: TreeId,
    pub name: String,
    pub plant_identity_id: PlantIdentityId,
    pub harvested_parts: Vec<HarvestedPart>,
    pub longitude: f64,
    pub latitude: f64,
    pub route_rank: u32,
    pub period: HarvestPeriod,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarvestProgress {
    pub run_id: HarvestRunId,
    pub target: HarvestRunTarget,
    pub harvested_parts: Vec<HarvestedPart>,
    pub route: Vec<HarvestTree>,
    pub handled_tree_count: usize,
    pub harvested_tree_count: usize,
    pub deferred_tree_count: usize,
    pub total_tree_count: usize,
    pub current_tree: Option<HarvestTree>,
}

#[derive(Debug, PartialEq)]
pub enum HarvestRunStartError {
    InvalidActionDate,
    NoHarvestPartsSelected,
    NoTreesCurrentlyAvailable,
    AnotherHarvestRunIsActive,
    HarvestRunCouldNotBeStarted,
}

pub fn start_harvest_run(
    event: HarvestRunStartRequested,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, HarvestRunStartError> {
    let action_date = HarvestDate::parse_iso(&event.action_date)
        .ok_or(HarvestRunStartError::InvalidActionDate)?;
    let harvested_parts = normalized_harvested_parts(event.harvested_parts)
        .ok_or(HarvestRunStartError::NoHarvestPartsSelected)?;
    storage.transaction(|orchard| {
        if let Some(active_run) = orchard
            .active_harvest_run(event.orchard_id)
            .map_err(|_| HarvestRunStartError::HarvestRunCouldNotBeStarted)?
        {
            if active_run.target != event.target || active_run.harvested_parts != harvested_parts {
                return Err(HarvestRunStartError::AnotherHarvestRunIsActive);
            }
            let orchard_trees = orchard
                .trees_in_orchard(event.orchard_id)
                .map_err(|_| HarvestRunStartError::HarvestRunCouldNotBeStarted)?;
            return harvest_progress(&active_run, &orchard_trees, action_date)
                .ok_or(HarvestRunStartError::HarvestRunCouldNotBeStarted);
        }

        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| HarvestRunStartError::HarvestRunCouldNotBeStarted)?;
        let previous_outcomes = orchard
            .harvest_tree_outcomes(event.orchard_id)
            .map_err(|_| HarvestRunStartError::HarvestRunCouldNotBeStarted)?;
        let eligible_trees = eligible_harvest_trees(
            &orchard_trees,
            &previous_outcomes,
            action_date,
            &harvested_parts,
        )
        .into_iter()
        .filter(|candidate| target_includes(event.target, candidate.tree))
        .collect::<Vec<_>>();
        if eligible_trees.is_empty() {
            return Err(HarvestRunStartError::NoTreesCurrentlyAvailable);
        }

        let ordered_trees = nearest_neighbour_route(eligible_trees)
            .into_iter()
            .map(|candidate| HarvestRunTree {
                tree_id: candidate.tree.id,
                harvested_parts: candidate.harvested_parts,
                period: candidate.period,
                outcome: None,
            })
            .collect::<Vec<_>>();
        let run_id = orchard
            .create_harvest_run(
                event.orchard_id,
                event.target,
                &harvested_parts,
                action_date,
                &ordered_trees,
            )
            .map_err(|_| HarvestRunStartError::HarvestRunCouldNotBeStarted)?;
        let run = HarvestRun {
            id: run_id,
            orchard_id: event.orchard_id,
            target: event.target,
            harvested_parts,
            started_on: action_date,
            ordered_trees,
            completed: false,
        };
        harvest_progress(&run, &orchard_trees, action_date)
            .ok_or(HarvestRunStartError::HarvestRunCouldNotBeStarted)
    })
}

fn target_includes(target: HarvestRunTarget, tree: &OrchardTree) -> bool {
    match target {
        HarvestRunTarget::All => true,
        HarvestRunTarget::Species(plant_identity_id) => {
            tree.tree.plant_identity_id == plant_identity_id
        }
    }
}

fn nearest_neighbour_route<'a>(
    mut trees: Vec<EligibleHarvestTree<'a>>,
) -> Vec<EligibleHarvestTree<'a>> {
    trees.sort_by_key(|candidate| candidate.tree.id.0);
    let mut route = vec![trees.remove(0)];
    while !trees.is_empty() {
        let previous = route.last().expect("a harvest route has a first tree").tree;
        let next_index = trees
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                tree_distance(previous, left.tree)
                    .total_cmp(&tree_distance(previous, right.tree))
                    .then_with(|| left.tree.id.0.cmp(&right.tree.id.0))
            })
            .map(|(index, _)| index)
            .expect("a non-empty harvest remainder has a nearest tree");
        route.push(trees.remove(next_index));
    }
    route
}

fn tree_distance(from: &OrchardTree, to: &OrchardTree) -> f64 {
    let mean_latitude = ((from.tree.latitude + to.tree.latitude) / 2.0).to_radians();
    let longitude = (to.tree.longitude - from.tree.longitude).to_radians() * mean_latitude.cos();
    let latitude = (to.tree.latitude - from.tree.latitude).to_radians();
    longitude.hypot(latitude)
}

pub(crate) fn harvest_progress(
    run: &HarvestRun,
    orchard_trees: &[OrchardTree],
    action_date: HarvestDate,
) -> Option<HarvestProgress> {
    let route = run
        .ordered_trees
        .iter()
        .enumerate()
        .map(|(index, run_tree)| {
            let tree = orchard_trees
                .iter()
                .find(|tree| tree.id == run_tree.tree_id)?;
            Some(HarvestTree {
                id: tree.id,
                name: tree
                    .tree
                    .legacy_source
                    .as_ref()
                    .map(|source| source.name.clone())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| tree.plant_identity.common_name.clone()),
                plant_identity_id: tree.tree.plant_identity_id,
                harvested_parts: run_tree.harvested_parts.clone(),
                longitude: tree.tree.longitude,
                latitude: tree.tree.latitude,
                route_rank: u32::try_from(index + 1).ok()?,
                period: run_tree.period,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let current_tree =
        current_harvest_tree_index(run, action_date).and_then(|index| route.get(index).cloned());
    let harvested_tree_count = run
        .ordered_trees
        .iter()
        .filter(|tree| {
            matches!(
                tree.outcome,
                Some(HarvestTreeOutcome::HarvestedEverything { .. })
            )
        })
        .count();
    let deferred_tree_count = run
        .ordered_trees
        .iter()
        .filter(|tree| {
            matches!(
                tree.outcome,
                Some(HarvestTreeOutcome::Deferred { retry_on, .. }) if action_date < retry_on
            )
        })
        .count();
    Some(HarvestProgress {
        run_id: run.id,
        target: run.target,
        harvested_parts: run.harvested_parts.clone(),
        route,
        handled_tree_count: harvested_tree_count + deferred_tree_count,
        harvested_tree_count,
        deferred_tree_count,
        total_tree_count: run.ordered_trees.len(),
        current_tree,
    })
}

pub(crate) fn current_harvest_tree_index(
    run: &HarvestRun,
    action_date: HarvestDate,
) -> Option<usize> {
    if run.completed {
        return None;
    }
    run.ordered_trees
        .iter()
        .position(|tree| match tree.outcome {
            None => true,
            Some(HarvestTreeOutcome::Deferred { retry_on, .. }) => retry_on <= action_date,
            Some(HarvestTreeOutcome::HarvestedEverything { .. }) => false,
        })
}

impl From<OrchardStorageError> for HarvestRunStartError {
    fn from(_: OrchardStorageError) -> Self {
        Self::HarvestRunCouldNotBeStarted
    }
}
