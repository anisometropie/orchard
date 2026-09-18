use crate::hexagon::models::{HarvestDate, OrchardId};
use crate::hexagon::ports::OrchardStorage;

use super::start_harvest_run::{HarvestProgress, harvest_progress};

#[derive(Debug, PartialEq)]
pub enum ActiveHarvestRunError {
    InvalidActionDate,
    HarvestRunCouldNotBeLoaded,
}

pub fn load_active_harvest_run(
    orchard_id: OrchardId,
    action_date: &str,
    storage: &mut impl OrchardStorage,
) -> Result<Option<HarvestProgress>, ActiveHarvestRunError> {
    let action_date =
        HarvestDate::parse_iso(action_date).ok_or(ActiveHarvestRunError::InvalidActionDate)?;
    let Some(run) = storage
        .active_harvest_run(orchard_id)
        .map_err(|_| ActiveHarvestRunError::HarvestRunCouldNotBeLoaded)?
    else {
        return Ok(None);
    };
    let orchard_trees = storage
        .trees_in_orchard(orchard_id)
        .map_err(|_| ActiveHarvestRunError::HarvestRunCouldNotBeLoaded)?;
    harvest_progress(&run, &orchard_trees, action_date)
        .map(Some)
        .ok_or(ActiveHarvestRunError::HarvestRunCouldNotBeLoaded)
}
