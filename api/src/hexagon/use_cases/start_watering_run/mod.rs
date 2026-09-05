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
    pub target: WateringRunTarget,
    pub watered_tree_count: usize,
    pub total_tree_count: usize,
    pub next_tree: Option<WateringTree>,
}

#[derive(Debug, PartialEq)]
pub enum WateringRunStartError {
    RowNotFound,
    RowNotOrdered,
    AnotherWateringRunIsActive,
    WateringRunCouldNotBeStarted,
}

pub fn start_watering_run(
    event: WateringRunStartRequested,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, WateringRunStartError> {
    storage.transaction(|orchard| {
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?;
        if let Some(active_run) = orchard
            .active_watering_run(event.orchard_id)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?
        {
            if active_run.target != WateringRunTarget::Row(event.row_name.clone()) {
                return Err(WateringRunStartError::AnotherWateringRunIsActive);
            }
            return watering_progress(&active_run, &orchard_trees)
                .ok_or(WateringRunStartError::WateringRunCouldNotBeStarted);
        }

        let mut row_trees = orchard_trees
            .iter()
            .filter(|tree| {
                tree.tree.is_alive && tree.tree.row_name.as_deref() == Some(event.row_name.as_str())
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
            .create_watering_run(event.orchard_id, &target, &ordered_tree_ids)
            .map_err(|_| WateringRunStartError::WateringRunCouldNotBeStarted)?;
        let run = WateringRun {
            id: run_id,
            orchard_id: event.orchard_id,
            target,
            ordered_tree_ids,
            watered_tree_ids: vec![],
            completed: false,
        };
        watering_progress(&run, &orchard_trees)
            .ok_or(WateringRunStartError::WateringRunCouldNotBeStarted)
    })
}

pub(crate) fn watering_progress(
    run: &WateringRun,
    orchard_trees: &[OrchardTree],
) -> Option<WateringProgress> {
    let next_tree = match run
        .ordered_tree_ids
        .iter()
        .enumerate()
        .find(|(_, tree_id)| !run.watered_tree_ids.contains(tree_id))
    {
        None => None,
        Some((index, tree_id)) => {
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
        }
    };
    Some(WateringProgress {
        run_id: run.id,
        target: run.target.clone(),
        watered_tree_count: run.watered_tree_ids.len(),
        total_tree_count: run.ordered_tree_ids.len(),
        next_tree,
    })
}

impl From<OrchardStorageError> for WateringRunStartError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeStarted
    }
}
