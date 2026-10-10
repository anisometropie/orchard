use super::replace_plant_harvest_windows::{
    OrchardHarvestWindowsReplaced, PlantHarvestWindowsReplacementError,
    replace_orchard_harvest_windows_in_transaction,
};
use crate::hexagon::models::{HarvestScheduleOwner, OrchardId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct HarvestObservationsReviewed {
    pub orchard_id: OrchardId,
    pub owner: HarvestScheduleOwner,
    pub observation_ids: Vec<u64>,
    pub window_change: Option<OrchardHarvestWindowsReplaced>,
}

#[derive(Debug, PartialEq)]
pub enum HarvestObservationReviewError {
    InvalidObservationSelection,
    InvalidWindowChange(PlantHarvestWindowsReplacementError),
    CouldNotReview,
}

pub fn review_harvest_observations(
    event: HarvestObservationsReviewed,
    storage: &mut impl OrchardStorage,
) -> Result<(), HarvestObservationReviewError> {
    if event.observation_ids.is_empty() {
        return Err(HarvestObservationReviewError::InvalidObservationSelection);
    }
    let mut ids = event.observation_ids.clone();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != event.observation_ids.len()
        || event.window_change.as_ref().is_some_and(|change| {
            change.orchard_id != event.orchard_id || change.owner != event.owner
        })
    {
        return Err(HarvestObservationReviewError::InvalidObservationSelection);
    }
    storage.transaction(|orchard| {
        orchard.lock_orchard_runs(event.orchard_id)?;
        let observations = orchard.harvest_observations(event.orchard_id, event.owner)?;
        if ids.iter().any(|id| {
            !observations
                .iter()
                .any(|point| point.id == *id && !point.reviewed && point.has_timing_evidence())
        }) {
            return Err(HarvestObservationReviewError::InvalidObservationSelection);
        }
        if let Some(change) = event.window_change {
            replace_orchard_harvest_windows_in_transaction(change, orchard)
                .map_err(HarvestObservationReviewError::InvalidWindowChange)?;
        }
        orchard.review_harvest_observations(event.orchard_id, event.owner, &ids)?;
        Ok(())
    })
}

impl From<OrchardStorageError> for HarvestObservationReviewError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotReview
    }
}
