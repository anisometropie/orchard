#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod fixture;

use orchard_api::hexagon::models::{OrchardId, WateringRunId};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::list_watering_runs::list_watering_runs;

#[test]
fn list_unfinished_progress_including_paused_runs_only_in_the_requested_orchard() {
    let (mut storage, _) = fixture::storage();
    let active = list_watering_runs(OrchardId(7), &mut storage).unwrap();
    assert_eq!(active.len(), 1);
    assert!(!active[0].paused);
    storage
        .transaction(|orchard| orchard.set_watering_run_paused(WateringRunId(1), true))
        .unwrap();
    assert!(list_watering_runs(OrchardId(7), &mut storage).unwrap()[0].paused);
    assert!(
        list_watering_runs(OrchardId(8), &mut storage)
            .unwrap()
            .is_empty()
    );
    storage
        .transaction(|orchard| orchard.complete_watering_run(WateringRunId(1)))
        .unwrap();
    assert!(
        list_watering_runs(OrchardId(7), &mut storage)
            .unwrap()
            .is_empty()
    );
}
