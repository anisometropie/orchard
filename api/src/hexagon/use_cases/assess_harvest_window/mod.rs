use crate::hexagon::models::{
    HarvestDataOrigin, HarvestScheduleOwner, HarvestValidationEvidence, HarvestWindowSelection,
    OrchardId, harvest_validation_evidence,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};
use serde::Serialize;

pub struct HarvestWindowAssessmentRequested {
    pub orchard_id: OrchardId,
    pub owner: HarvestScheduleOwner,
    pub window: HarvestWindowSelection,
}

#[derive(Serialize)]
pub struct HarvestWindowAssessment {
    pub validated: bool,
    pub data_origin: HarvestDataOrigin,
    pub evidence: HarvestValidationEvidence,
}

#[derive(Debug, PartialEq)]
pub enum HarvestWindowAssessmentError {
    WindowNotFound,
    CouldNotAssess,
}

pub fn assess_harvest_window(
    event: HarvestWindowAssessmentRequested,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestWindowAssessment, HarvestWindowAssessmentError> {
    let windows = storage.orchard_harvest_windows(event.orchard_id, event.owner)?;
    let matches = windows
        .iter()
        .filter(|saved| event.window.matches(saved))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(HarvestWindowAssessmentError::WindowNotFound);
    }
    let window = matches[0];
    let points = storage.harvest_observations(event.orchard_id, event.owner)?;
    Ok(HarvestWindowAssessment {
        validated: window.validated,
        data_origin: window.data_origin,
        evidence: harvest_validation_evidence(window, &points),
    })
}

impl From<OrchardStorageError> for HarvestWindowAssessmentError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotAssess
    }
}
