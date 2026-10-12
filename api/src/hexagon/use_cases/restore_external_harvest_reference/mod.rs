use crate::hexagon::models::{
    HarvestDataOrigin, HarvestScheduleOwner, HarvestWindowSelection, OrchardId,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct ExternalHarvestReferenceRestored {
    pub orchard_id: OrchardId,
    pub owner: HarvestScheduleOwner,
    pub window: HarvestWindowSelection,
}

#[derive(Debug, PartialEq)]
pub enum ExternalHarvestReferenceRestorationError {
    WindowNotFound,
    CouldNotRestore,
}

pub fn restore_external_harvest_reference(
    event: ExternalHarvestReferenceRestored,
    storage: &mut impl OrchardStorage,
) -> Result<(), ExternalHarvestReferenceRestorationError> {
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
            return Err(ExternalHarvestReferenceRestorationError::WindowNotFound);
        }
        let window = &mut windows[indexes[0]];
        window.data_origin = HarvestDataOrigin::ExternalReference;
        window.validated = false;
        if !orchard.replace_orchard_harvest_windows(event.orchard_id, event.owner, windows)? {
            return Err(ExternalHarvestReferenceRestorationError::WindowNotFound);
        }
        Ok(())
    })
}

impl From<OrchardStorageError> for ExternalHarvestReferenceRestorationError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotRestore
    }
}
