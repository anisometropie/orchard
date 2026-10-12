use crate::hexagon::models::{
    HarvestDataOrigin, HarvestScheduleOwner, HarvestWindowSelection, OrchardId,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct HarvestWindowValidated {
    pub orchard_id: OrchardId,
    pub owner: HarvestScheduleOwner,
    pub window: HarvestWindowSelection,
}

#[derive(Debug, PartialEq)]
pub enum HarvestWindowValidationError {
    WindowNotFound,
    CouldNotValidate,
}

pub fn validate_harvest_window(
    event: HarvestWindowValidated,
    storage: &mut impl OrchardStorage,
) -> Result<(), HarvestWindowValidationError> {
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let mut windows = orchard.orchard_harvest_windows(event.orchard_id, event.owner)?;
        let indexes = windows
            .iter()
            .enumerate()
            .filter(|(_, saved)| event.window.matches(saved))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if indexes.len() != 1 {
            return Err(HarvestWindowValidationError::WindowNotFound);
        }
        let window = &mut windows[indexes[0]];
        window.validated = true;
        window.data_origin = HarvestDataOrigin::FieldObservation;
        if !orchard.replace_orchard_harvest_windows(event.orchard_id, event.owner, windows)? {
            return Err(HarvestWindowValidationError::WindowNotFound);
        }
        Ok(())
    })
}

impl From<OrchardStorageError> for HarvestWindowValidationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotValidate
    }
}
