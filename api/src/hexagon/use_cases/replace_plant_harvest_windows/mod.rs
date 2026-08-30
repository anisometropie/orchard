use crate::hexagon::models::{AnnualDate, AnnualHarvestWindow, HarvestScheduleOwner};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[derive(Clone, Copy)]
pub struct AnnualHarvestWindowChanged {
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
}

pub struct PlantHarvestWindowsReplaced {
    pub owner: HarvestScheduleOwner,
    pub windows: Vec<AnnualHarvestWindowChanged>,
}

#[derive(Debug, PartialEq)]
pub enum PlantHarvestWindowsReplacementError {
    InvalidAnnualDate,
    OwnerNotFound,
    HarvestWindowsCouldNotBeReplaced,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn replace_plant_harvest_windows(
    event: PlantHarvestWindowsReplaced,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), PlantHarvestWindowsReplacementError> {
    let harvest_windows = event
        .windows
        .into_iter()
        .map(|window| {
            let start = AnnualDate::new(window.start_month, window.start_day)
                .ok_or(PlantHarvestWindowsReplacementError::InvalidAnnualDate)?;
            let end = AnnualDate::new(window.end_month, window.end_day)
                .ok_or(PlantHarvestWindowsReplacementError::InvalidAnnualDate)?;
            Ok::<AnnualHarvestWindow, PlantHarvestWindowsReplacementError>(AnnualHarvestWindow {
                start,
                end,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    orchard_storage.transaction(|orchard| {
        match orchard.replace_harvest_windows(event.owner, harvest_windows) {
            Ok(true) => Ok(()),
            Ok(false) => Err(PlantHarvestWindowsReplacementError::OwnerNotFound),
            Err(_) => Err(PlantHarvestWindowsReplacementError::HarvestWindowsCouldNotBeReplaced),
        }
    })
}

impl From<OrchardStorageError> for PlantHarvestWindowsReplacementError {
    fn from(error: OrchardStorageError) -> Self {
        match error {
            OrchardStorageError::AtomicOperationCouldNotBegin => Self::TransactionCouldNotBegin,
            OrchardStorageError::AtomicOperationCouldNotCommit => Self::TransactionCouldNotCommit,
            _ => Self::HarvestWindowsCouldNotBeReplaced,
        }
    }
}

#[cfg(test)]
mod replace_plant_harvest_windows_unit_test;
