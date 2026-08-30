use crate::hexagon::models::{AnnualDate, AnnualHarvestWindow, PlantIdentityId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct PlantIdentityHarvestWindowChanged {
    pub plant_identity_id: PlantIdentityId,
    pub start_month: u8,
    pub start_day: u8,
    pub end_month: u8,
    pub end_day: u8,
}

#[derive(Debug, PartialEq)]
pub enum PlantIdentityHarvestWindowChangeError {
    InvalidAnnualDate,
    PlantIdentityNotFound,
    HarvestWindowCouldNotBeChanged,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn change_plant_identity_harvest_window(
    event: PlantIdentityHarvestWindowChanged,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), PlantIdentityHarvestWindowChangeError> {
    let start = AnnualDate::new(event.start_month, event.start_day)
        .ok_or(PlantIdentityHarvestWindowChangeError::InvalidAnnualDate)?;
    let end = AnnualDate::new(event.end_month, event.end_day)
        .ok_or(PlantIdentityHarvestWindowChangeError::InvalidAnnualDate)?;
    let harvest_window = AnnualHarvestWindow { start, end };

    orchard_storage.transaction(|orchard| {
        match orchard.change_plant_identity_harvest_window(event.plant_identity_id, harvest_window)
        {
            Ok(true) => Ok(()),
            Ok(false) => Err(PlantIdentityHarvestWindowChangeError::PlantIdentityNotFound),
            Err(_) => Err(PlantIdentityHarvestWindowChangeError::HarvestWindowCouldNotBeChanged),
        }
    })
}

impl From<OrchardStorageError> for PlantIdentityHarvestWindowChangeError {
    fn from(error: OrchardStorageError) -> Self {
        match error {
            OrchardStorageError::AtomicOperationCouldNotBegin => Self::TransactionCouldNotBegin,
            OrchardStorageError::AtomicOperationCouldNotCommit => Self::TransactionCouldNotCommit,
            _ => Self::HarvestWindowCouldNotBeChanged,
        }
    }
}

#[cfg(test)]
mod change_plant_identity_harvest_window_unit_test;
