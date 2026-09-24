use orchard_api::hexagon::models::{TreeId, WateringRunId};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn assert_skips_are_atomic_and_distinct_from_watering(
    storage: &mut impl OrchardStorage,
    run_id: WateringRunId,
    watered: TreeId,
    pending: TreeId,
) {
    let before = storage.watering_run(run_id).unwrap();
    assert!(storage.transaction(|orchard| orchard.mark_watering_tree_skipped(run_id, watered)).is_err());
    let result: Result<(), OrchardStorageError> = storage.transaction(|orchard| {
        orchard.mark_watering_tree_skipped(run_id, pending)?;
        Err(OrchardStorageError::WateringRunCouldNotBeChanged)
    });
    assert!(result.is_err());
    assert_eq!(storage.watering_run(run_id).unwrap(), before);
    storage.transaction(|orchard| orchard.mark_watering_tree_skipped(run_id, pending)).unwrap();
    let skipped = storage.watering_run(run_id).unwrap().unwrap();
    assert_eq!(skipped.watered_tree_ids, vec![watered]);
    assert_eq!(skipped.skipped_tree_ids, vec![pending]);
    assert!(storage.transaction(|orchard| orchard.mark_watering_tree_skipped(run_id, pending)).is_err());
    assert!(storage.transaction(|orchard| orchard.mark_watering_tree_watered(run_id, pending)).is_err());
    assert_eq!(storage.watering_run(run_id).unwrap(), Some(skipped));
}
