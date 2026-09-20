use crate::hexagon::models::{
    HarvestDate, HarvestPeriod, HarvestRunId, HarvestRunTarget, HarvestTreeOutcome, HarvestedPart,
    OrchardId, OrchardTree, TreeId, WateringRunId,
};
use crate::hexagon::ports::{AccessControl, OrchardStorage};
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub struct OrchardRunHistoryRequested {
    pub orchard_id: OrchardId,
    pub session_token: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WateringRunHistoryTree {
    pub tree_id: TreeId,
    pub name: String,
    pub watered_at_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WateringRunHistory {
    pub run_id: WateringRunId,
    pub target_label: String,
    pub carry_capacity: Option<u32>,
    pub started_at_unix_seconds: i64,
    pub completed_at_unix_seconds: i64,
    pub trees: Vec<WateringRunHistoryTree>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarvestRunHistoryTree {
    pub tree_id: TreeId,
    pub name: String,
    pub harvested_parts: Vec<HarvestedPart>,
    pub period: HarvestPeriod,
    pub outcome: HarvestTreeOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarvestRunHistory {
    pub run_id: HarvestRunId,
    pub target_label: String,
    pub harvested_parts: Vec<HarvestedPart>,
    pub started_on: HarvestDate,
    pub completed_at_unix_seconds: i64,
    pub total_tree_count: usize,
    pub trees: Vec<HarvestRunHistoryTree>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrchardRunHistory {
    pub watering_runs: Vec<WateringRunHistory>,
    pub harvest_runs: Vec<HarvestRunHistory>,
}

#[derive(Debug, PartialEq)]
pub enum OrchardRunHistoryError {
    SessionNotFound,
    OrchardNotOwned,
    HistoryCouldNotBeRead,
}

pub fn list_orchard_run_history<T>(
    event: OrchardRunHistoryRequested,
    storage: &mut T,
) -> Result<OrchardRunHistory, OrchardRunHistoryError>
where
    T: AccessControl + OrchardStorage,
{
    authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id: event.orchard_id,
            session_token: event.session_token,
        },
        storage,
    )
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => OrchardRunHistoryError::SessionNotFound,
        OrchardOwnerAccessError::OrchardNotOwned => OrchardRunHistoryError::OrchardNotOwned,
        OrchardOwnerAccessError::AccessCouldNotBeChecked => {
            OrchardRunHistoryError::HistoryCouldNotBeRead
        }
    })?;

    let trees = storage
        .trees_in_orchard(event.orchard_id)
        .map_err(|_| OrchardRunHistoryError::HistoryCouldNotBeRead)?;
    let watering_runs = storage
        .completed_watering_runs(event.orchard_id)
        .map_err(|_| OrchardRunHistoryError::HistoryCouldNotBeRead)?
        .into_iter()
        .map(|run| {
            let trees = run
                .trees
                .into_iter()
                .map(|run_tree| {
                    Ok(WateringRunHistoryTree {
                        tree_id: run_tree.tree_id,
                        name: tree_name(&trees, run_tree.tree_id)?,
                        watered_at_unix_seconds: run_tree.watered_at_unix_seconds,
                    })
                })
                .collect::<Result<Vec<_>, OrchardRunHistoryError>>()?;
            Ok(WateringRunHistory {
                run_id: run.id,
                target_label: run.target.label().to_owned(),
                carry_capacity: run.carry_capacity,
                started_at_unix_seconds: run.started_at_unix_seconds,
                completed_at_unix_seconds: run.completed_at_unix_seconds,
                trees,
            })
        })
        .collect::<Result<Vec<_>, OrchardRunHistoryError>>()?;
    let harvest_runs = storage
        .completed_harvest_runs(event.orchard_id)
        .map_err(|_| OrchardRunHistoryError::HistoryCouldNotBeRead)?
        .into_iter()
        .map(|run| {
            let target_label = match run.target {
                HarvestRunTarget::All => "All currently available".into(),
                HarvestRunTarget::Species(plant_identity_id) => trees
                    .iter()
                    .find(|tree| tree.tree.plant_identity_id == plant_identity_id)
                    .map(|tree| tree.plant_identity.common_name.clone())
                    .ok_or(OrchardRunHistoryError::HistoryCouldNotBeRead)?,
            };
            let total_tree_count = run.ordered_trees.len();
            let trees = run
                .ordered_trees
                .into_iter()
                .filter_map(|run_tree| run_tree.outcome.map(|outcome| (run_tree, outcome)))
                .map(|(run_tree, outcome)| {
                    Ok(HarvestRunHistoryTree {
                        tree_id: run_tree.tree_id,
                        name: tree_name(&trees, run_tree.tree_id)?,
                        harvested_parts: run_tree.harvested_parts,
                        period: run_tree.period,
                        outcome,
                    })
                })
                .collect::<Result<Vec<_>, OrchardRunHistoryError>>()?;
            Ok(HarvestRunHistory {
                run_id: run.id,
                target_label,
                harvested_parts: run.harvested_parts,
                started_on: run.started_on,
                completed_at_unix_seconds: run.completed_at_unix_seconds,
                total_tree_count,
                trees,
            })
        })
        .collect::<Result<Vec<_>, OrchardRunHistoryError>>()?;

    Ok(OrchardRunHistory {
        watering_runs,
        harvest_runs,
    })
}

fn tree_name(trees: &[OrchardTree], tree_id: TreeId) -> Result<String, OrchardRunHistoryError> {
    let tree = trees
        .iter()
        .find(|tree| tree.id == tree_id)
        .ok_or(OrchardRunHistoryError::HistoryCouldNotBeRead)?;
    Ok(tree
        .tree
        .legacy_source
        .as_ref()
        .map(|source| source.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| tree.plant_identity.common_name.clone()))
}
