use orchard_api::hexagon::models::{OrchardId, WateringRunId};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn assert_pause_preserves_progress_and_rolls_back(
    storage: &mut impl OrchardStorage,
    orchard_id: OrchardId,
    run_id: WateringRunId,
) {
    let before = storage.watering_run(run_id).unwrap().unwrap();
    storage.transaction(|orchard| orchard.set_watering_run_paused(run_id, true)).unwrap();
    let mut expected = before.clone();
    expected.paused = true;
    assert_eq!(storage.watering_run(run_id).unwrap(), Some(expected.clone()));
    assert_eq!(storage.active_watering_run(orchard_id).unwrap(), None);
    assert_eq!(storage.unfinished_watering_runs(orchard_id).unwrap(), vec![expected.clone()]);
    let failed: Result<(), OrchardStorageError> = storage.transaction(|orchard| {
        orchard.set_watering_run_paused(run_id, false)?;
        Err(OrchardStorageError::WateringRunCouldNotBeChanged)
    });
    assert!(failed.is_err());
    assert_eq!(storage.watering_run(run_id).unwrap(), Some(expected));
    storage.transaction(|orchard| orchard.set_watering_run_paused(run_id, false)).unwrap();
    assert_eq!(storage.watering_run(run_id).unwrap(), Some(before));
}
