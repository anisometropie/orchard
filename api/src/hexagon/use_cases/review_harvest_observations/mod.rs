use super::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, PlantHarvestWindowsReplacementError,
    replace_orchard_harvest_windows_in_transaction,
};
use crate::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, HarvestDataOrigin, HarvestObservation, HarvestScheduleOwner,
    OrchardId,
};
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
    review(event, None, storage)
}

pub fn review_harvest_observations_for_window(
    event: HarvestObservationsReviewed,
    window: AnnualHarvestWindowChanged,
    storage: &mut impl OrchardStorage,
) -> Result<(), HarvestObservationReviewError> {
    review(event, Some(window), storage)
}

fn review(
    event: HarvestObservationsReviewed,
    window: Option<AnnualHarvestWindowChanged>,
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
        let scoped_windows = if let Some(window) = window {
            let windows = orchard.orchard_harvest_windows(event.orchard_id, event.owner)?;
            let indexes = windows
                .iter()
                .enumerate()
                .filter(|(_, saved)| {
                    saved.start.month == window.start_month
                        && saved.start.day == window.start_day
                        && saved.end.month == window.end_month
                        && saved.end.day == window.end_day
                        && saved.harvested_part == window.harvested_part
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if indexes.len() != 1 {
                return Err(HarvestObservationReviewError::InvalidObservationSelection);
            }
            Some((windows, indexes[0]))
        } else {
            None
        };
        let observations = orchard.harvest_observations(event.orchard_id, event.owner)?;
        if ids.iter().any(|id| {
            !observations.iter().any(|point| {
                point.id == *id
                    && !point.reviewed
                    && point.has_timing_evidence()
                    && window.is_none_or(|window| observation_matches_window(point, window))
            })
        }) {
            return Err(HarvestObservationReviewError::InvalidObservationSelection);
        }
        if let Some(change) = event.window_change {
            if let Some((windows, index)) = scoped_windows {
                update_reviewed_window(change, windows, index, orchard)?;
            } else {
                replace_orchard_harvest_windows_in_transaction(change, orchard)
                    .map_err(HarvestObservationReviewError::InvalidWindowChange)?;
            }
        }
        orchard.review_harvest_observations(event.orchard_id, event.owner, &ids)?;
        Ok(())
    })
}

fn observation_matches_window(
    point: &HarvestObservation,
    window: AnnualHarvestWindowChanged,
) -> bool {
    point.period.start.month == window.start_month
        && point.period.start.day == window.start_day
        && point.period.end.month == window.end_month
        && point.period.end.day == window.end_day
        && point.harvested_parts.contains(&window.harvested_part)
}

fn update_reviewed_window(
    change: OrchardHarvestWindowsReplaced,
    mut windows: Vec<AnnualHarvestWindow>,
    index: usize,
    storage: &mut impl OrchardStorage,
) -> Result<(), HarvestObservationReviewError> {
    if change.windows.len() != 1 {
        return Err(HarvestObservationReviewError::InvalidObservationSelection);
    }
    let invalid = HarvestObservationReviewError::InvalidWindowChange;
    let reference_region = change.reference_region.trim();
    if reference_region.is_empty() {
        return Err(invalid(
            PlantHarvestWindowsReplacementError::MissingReferenceRegion,
        ));
    }
    let replacement = change.windows[0];
    let start = AnnualDate::new(replacement.start_month, replacement.start_day)
        .ok_or_else(|| invalid(PlantHarvestWindowsReplacementError::InvalidAnnualDate))?;
    let end = AnnualDate::new(replacement.end_month, replacement.end_day)
        .ok_or_else(|| invalid(PlantHarvestWindowsReplacementError::InvalidAnnualDate))?;
    let saved = &windows[index];
    if saved.start != start
        || saved.end != end
        || saved.harvested_part != replacement.harvested_part
        || saved.reference_region.as_deref() != Some(reference_region)
    {
        windows[index] = AnnualHarvestWindow {
            start,
            end,
            reference_region: Some(reference_region.into()),
            harvested_part: replacement.harvested_part,
            data_origin: HarvestDataOrigin::FieldObservation,
            source_url: None,
        };
    }
    if !storage.replace_orchard_harvest_windows(change.orchard_id, change.owner, windows)? {
        return Err(invalid(PlantHarvestWindowsReplacementError::OwnerNotFound));
    }
    Ok(())
}

impl From<OrchardStorageError> for HarvestObservationReviewError {
    fn from(_: OrchardStorageError) -> Self {
        Self::CouldNotReview
    }
}
