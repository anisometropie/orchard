use crate::hexagon::models::{
    HarvestFruitState, HarvestObservation, HarvestObservationKind, HarvestRunId,
    HarvestScheduleOwner, OrchardId, TreeId,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct LegacyHarvestObservationClassified {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub kind: HarvestObservationKind,
    pub fruit_state: Option<HarvestFruitState>,
}

#[derive(Debug, PartialEq)]
pub enum LegacyHarvestObservationError {
    InvalidFruitState,
    RunNotFound,
    RunNotCompleted,
    TreeNotInRun,
    NotLegacyVisit,
    AlreadyClassified,
    CouldNotClassify,
}

pub fn classify_legacy_harvest_observation(
    event: LegacyHarvestObservationClassified,
    storage: &mut impl OrchardStorage,
) -> Result<u64, LegacyHarvestObservationError> {
    use LegacyHarvestObservationError::*;
    if !matches!(
        event.kind,
        HarvestObservationKind::HarvestedAll
            | HarvestObservationKind::EmptyOnArrival
            | HarvestObservationKind::AllWasted
            | HarvestObservationKind::FruitRemaining
    ) {
        return Err(InvalidFruitState);
    }
    let fruit_state = event
        .kind
        .fruit_state(event.fruit_state)
        .map_err(|_| InvalidFruitState)?;
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let run = orchard
            .harvest_run(event.harvest_run_id)?
            .filter(|run| run.orchard_id == event.orchard_id)
            .ok_or(RunNotFound)?;
        if !run.completed {
            return Err(RunNotCompleted);
        }
        let visit = run
            .ordered_trees
            .iter()
            .find(|tree| tree.tree_id == event.tree_id)
            .ok_or(TreeNotInRun)?;
        let recorded_on = visit
            .outcome
            .and_then(|outcome| outcome.legacy_observation_date())
            .ok_or(NotLegacyVisit)?;
        if orchard
            .harvest_observations_in_orchard(event.orchard_id)?
            .iter()
            .any(|point| point.matches_run_visit(event.harvest_run_id, event.tree_id, recorded_on))
        {
            return Err(AlreadyClassified);
        }
        let trees = orchard.trees_in_orchard(event.orchard_id)?;
        let tree = trees
            .iter()
            .find(|tree| tree.id == event.tree_id)
            .ok_or(TreeNotInRun)?;
        let owner = tree.tree.cultivar_id.map_or(
            HarvestScheduleOwner::PlantIdentity(tree.tree.plant_identity_id),
            HarvestScheduleOwner::PlantCultivar,
        );
        orchard
            .save_harvest_observation(HarvestObservation {
                id: 0,
                orchard_id: event.orchard_id,
                harvest_run_id: Some(event.harvest_run_id),
                tree_id: event.tree_id,
                owner,
                recorded_on,
                kind: event.kind,
                fruit_state,
                harvested_parts: visit.harvested_parts.clone(),
                period: Some(visit.period),
                reviewed: false,
            })
            .map_err(Into::into)
    })
}

impl From<OrchardStorageError> for LegacyHarvestObservationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotClassify
    }
}
