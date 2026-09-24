use crate::hexagon::models::{
    OrchardId, OrchardTree, TreeId, WateringRun, WateringRunId, WateringRunTarget,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct WateringRunStartRequested {
    pub orchard_id: OrchardId,
    pub row_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WateringTree {
    pub id: TreeId,
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
    pub row_rank: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WateringProgress {
    pub run_id: WateringRunId,
    pub paused: bool,
    pub target: WateringRunTarget,
    pub water_source: Option<crate::hexagon::models::GeoPoint>,
    pub carry_capacity: Option<u32>,
    pub route: Vec<WateringTree>,
    pub watered_tree_count: usize,
    pub skipped_tree_count: usize,
    pub handled_tree_count: usize,
    pub watered_tree_ids: Vec<TreeId>,
    pub skipped_tree_ids: Vec<TreeId>,
    pub total_tree_count: usize,
    pub next_tree: Option<WateringTree>,
}

#[derive(Debug, PartialEq)]
pub enum WateringRunStartError {
    RowNotFound,
    RowNotOrdered,
    HarvestRunIsActive,
    WateringRunCouldNotBeStarted,
}

pub fn start_watering_run(
    event: WateringRunStartRequested,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringRunStartError> {
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        if orchard
            .active_harvest_run(event.orchard_id)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?
            .is_some()
        {
            return Err(WateringRunStartError::HarvestRunIsActive);
        }
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?;
        if let Some(active_run) = orchard
            .unfinished_watering_runs(event.orchard_id)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?
            .into_iter()
            .find(|run| !run.paused && run.target == WateringRunTarget::Row(event.row_name.clone()))
        {
            return watering_progress(&active_run, &orchard_trees)
                .ok_or(WateringRunStartError::WateringRunCouldNotBeStarted);
        }

        let mut row_trees = orchard_trees
            .iter()
            .filter(|tree| {
                tree.tree.is_alive
                    && !tree.tree.is_excluded_from_watering
                    && tree.tree.row_name.as_deref() == Some(event.row_name.as_str())
            })
            .collect::<Vec<_>>();
        if row_trees.is_empty() {
            return Err(WateringRunStartError::RowNotFound);
        }
        if row_trees.iter().any(|tree| tree.row_rank.is_none()) {
            return Err(WateringRunStartError::RowNotOrdered);
        }
        row_trees.sort_by_key(|tree| tree.row_rank);
        let ordered_tree_ids = row_trees.iter().map(|tree| tree.id).collect::<Vec<_>>();
        let target = WateringRunTarget::Row(event.row_name);
        let run_id = orchard
            .create_watering_run(event.orchard_id, &target, None, None, &ordered_tree_ids)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?;
        let run = WateringRun {
            id: run_id,
            orchard_id: event.orchard_id,
            target,
            water_source: None,
            carry_capacity: None,
            ordered_tree_ids,
            watered_tree_ids: vec![],
            skipped_tree_ids: vec![],
            completed: false,
            paused: false,
        };
        watering_progress(&run, &orchard_trees)
            .ok_or(WateringRunStartError::WateringRunCouldNotBeStarted)
    })
}

pub(crate) fn watering_progress(
    run: &WateringRun,
    orchard_trees: &[OrchardTree],
) -> Option<WateringProgress> {
    let route = run
        .ordered_tree_ids
        .iter()
        .enumerate()
        .map(|(index, tree_id)| {
            let tree = orchard_trees.iter().find(|tree| tree.id == *tree_id)?;
            Some(WateringTree {
                id: tree.id,
                name: tree
                    .tree
                    .legacy_source
                    .as_ref()
                    .map(|source| source.name.clone())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| tree.plant_identity.common_name.clone()),
                longitude: tree.tree.longitude,
                latitude: tree.tree.latitude,
                row_rank: u32::try_from(index + 1).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let next_tree_id = run.next_tree_id();
    let next_tree = route
        .iter()
        .find(|tree| Some(tree.id) == next_tree_id)
        .cloned();
    Some(WateringProgress {
        run_id: run.id,
        paused: run.paused,
        target: run.target.clone(),
        water_source: run.water_source,
        carry_capacity: run.carry_capacity,
        route,
        watered_tree_count: run.watered_tree_ids.len(),
        skipped_tree_count: run.skipped_tree_ids.len(),
        handled_tree_count: run.watered_tree_ids.len() + run.skipped_tree_ids.len(),
        watered_tree_ids: run.watered_tree_ids.clone(),
        skipped_tree_ids: run.skipped_tree_ids.clone(),
        total_tree_count: run.ordered_tree_ids.len(),
        next_tree,
    })
}

impl From<OrchardStorageError> for WateringRunStartError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeStarted
    }
}
