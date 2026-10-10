pub use super::record_harvest_visit::HarvestVisitRecordingError as HarvestObservationRecordingError;
use super::record_harvest_visit::{HarvestVisitRecorded, record_harvest_visit};
use super::start_harvest_run::HarvestProgress;
use crate::hexagon::models::{HarvestObservationKind, HarvestRunId, OrchardId, TreeId};
use crate::hexagon::ports::OrchardStorage;

pub struct HarvestObservationRecorded {
    pub orchard_id: OrchardId,
    pub harvest_run_id: HarvestRunId,
    pub tree_id: TreeId,
    pub action_date: String,
    pub kind: HarvestObservationKind,
}

pub fn record_harvest_observation(
    event: HarvestObservationRecorded,
    storage: &mut impl OrchardStorage,
) -> Result<HarvestProgress, HarvestObservationRecordingError> {
    record_harvest_visit(
        HarvestVisitRecorded {
            orchard_id: event.orchard_id,
            harvest_run_id: event.harvest_run_id,
            tree_id: event.tree_id,
            action_date: event.action_date,
            kind: event.kind,
            fruit_state: None,
        },
        storage,
    )
}
