use crate::hexagon::models::{
    AnnualDate, HarvestDate, HarvestFruitState, HarvestObservation, HarvestObservationKind,
    HarvestScheduleOwner, HarvestedPart, OrchardId, TreeId, concrete_harvest_period,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[derive(Clone, Copy)]
pub struct ObservationHarvestWindow {
    pub start: AnnualDate,
    pub end: AnnualDate,
    pub start_year: i32,
}

pub struct StandaloneHarvestObservationRecorded {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub action_date: String,
    pub harvested_part: HarvestedPart,
    pub window: Option<ObservationHarvestWindow>,
    pub kind: HarvestObservationKind,
    pub fruit_state: Option<HarvestFruitState>,
}

#[derive(Debug, PartialEq)]
pub enum StandaloneHarvestObservationError {
    InvalidActionDate,
    InvalidFruitState,
    TreeNotFound,
    HarvestWindowChanged,
    ObservationCouldNotBeRecorded,
}

pub fn record_standalone_harvest_observation(
    event: StandaloneHarvestObservationRecorded,
    storage: &mut impl OrchardStorage,
) -> Result<u64, StandaloneHarvestObservationError> {
    use StandaloneHarvestObservationError::*;
    let recorded_on = HarvestDate::parse_iso(&event.action_date).ok_or(InvalidActionDate)?;
    let fruit_state = event
        .kind
        .fruit_state(event.fruit_state)
        .map_err(|_| InvalidFruitState)?;
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let trees = orchard.trees_in_orchard(event.orchard_id)?;
        let tree = trees
            .iter()
            .find(|tree| tree.id == event.tree_id)
            .ok_or(TreeNotFound)?;
        let period = event
            .window
            .map(|selected| {
                let window = tree
                    .harvest_windows
                    .iter()
                    .find(|window| {
                        window.start == selected.start
                            && window.end == selected.end
                            && window.harvested_part == event.harvested_part
                    })
                    .ok_or(HarvestWindowChanged)?;
                concrete_harvest_period(window, selected.start_year).ok_or(InvalidActionDate)
            })
            .transpose()?;
        let owner = tree.tree.cultivar_id.map_or(
            HarvestScheduleOwner::PlantIdentity(tree.tree.plant_identity_id),
            HarvestScheduleOwner::PlantCultivar,
        );
        orchard
            .save_harvest_observation(HarvestObservation {
                id: 0,
                orchard_id: event.orchard_id,
                harvest_run_id: None,
                tree_id: event.tree_id,
                owner,
                recorded_on,
                kind: event.kind,
                fruit_state,
                harvested_parts: vec![event.harvested_part],
                period,
                reviewed: false,
            })
            .map_err(Into::into)
    })
}

impl From<OrchardStorageError> for StandaloneHarvestObservationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::ObservationCouldNotBeRecorded
    }
}
