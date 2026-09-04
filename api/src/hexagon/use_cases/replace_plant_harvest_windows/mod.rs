use crate::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, HarvestDataOrigin, HarvestScheduleOwner, HarvestedPart,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[derive(Clone, Copy)]
pub struct AnnualHarvestWindowChanged {
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
    pub harvested_part: HarvestedPart,
}

pub struct PlantHarvestWindowsReplaced {
    pub owner: HarvestScheduleOwner,
    pub reference_region: String,
    pub windows: Vec<AnnualHarvestWindowChanged>,
}

pub struct OrchardHarvestWindowsReplaced {
    pub orchard_id: crate::hexagon::models::OrchardId,
    pub owner: HarvestScheduleOwner,
    pub reference_region: String,
    pub windows: Vec<AnnualHarvestWindowChanged>,
}

#[derive(Debug, PartialEq)]
pub enum PlantHarvestWindowsReplacementError {
    InvalidAnnualDate,
    MissingReferenceRegion,
    OwnerNotFound,
    HarvestWindowsCouldNotBeReplaced,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn replace_plant_harvest_windows(
    event: PlantHarvestWindowsReplaced,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), PlantHarvestWindowsReplacementError> {
    let reference_region = event.reference_region.trim();
    if !event.windows.is_empty() && reference_region.is_empty() {
        return Err(PlantHarvestWindowsReplacementError::MissingReferenceRegion);
    }
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
                reference_region: Some(reference_region.into()),
                harvested_part: window.harvested_part,
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
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

pub fn replace_orchard_harvest_windows(
    event: OrchardHarvestWindowsReplaced,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), PlantHarvestWindowsReplacementError> {
    let reference_region = event.reference_region.trim();
    if !event.windows.is_empty() && reference_region.is_empty() {
        return Err(PlantHarvestWindowsReplacementError::MissingReferenceRegion);
    }
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
                reference_region: Some(reference_region.into()),
                harvested_part: window.harvested_part,
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    orchard_storage.transaction(|orchard| {
        match orchard.replace_orchard_harvest_windows(
            event.orchard_id,
            event.owner,
            harvest_windows,
        ) {
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
