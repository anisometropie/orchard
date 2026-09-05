use crate::hexagon::models::OrchardId;
use crate::hexagon::ports::OrchardStorage;

use super::start_watering_run::{WateringProgress, watering_progress};

#[derive(Debug, PartialEq)]
pub enum ActiveWateringRunError {
    WateringRunCouldNotBeLoaded,
}

pub fn load_active_watering_run(
    orchard_id: OrchardId,
    storage: &mut impl OrchardStorage,
) -> Result<Option<WateringProgress>, ActiveWateringRunError> {
    let Some(run) = storage
        .active_watering_run(orchard_id)
        .map_err(|_| ActiveWateringRunError::WateringRunCouldNotBeLoaded)?
    else {
        return Ok(None);
    };
    let orchard_trees = storage
        .trees_in_orchard(orchard_id)
        .map_err(|_| ActiveWateringRunError::WateringRunCouldNotBeLoaded)?;
    watering_progress(&run, &orchard_trees)
        .map(Some)
        .ok_or(ActiveWateringRunError::WateringRunCouldNotBeLoaded)
}
