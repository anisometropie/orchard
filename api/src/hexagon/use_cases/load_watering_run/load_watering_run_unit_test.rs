#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod fixture;

use orchard_api::hexagon::models::{OrchardId, WateringRunId};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::load_watering_run::{WateringRunLoadError, load_watering_run};

#[test]
fn load_the_selected_run_including_paused_progress_without_crossing_orchard_boundary() {
    let (mut storage, _) = fixture::storage();
    storage
        .transaction(|orchard| orchard.set_watering_run_paused(WateringRunId(1), true))
        .unwrap();
    let progress = load_watering_run(OrchardId(7), WateringRunId(1), &mut storage).unwrap();
    assert!(progress.paused);
    assert_eq!(progress.total_tree_count, 2);
    assert_eq!(
        load_watering_run(OrchardId(8), WateringRunId(1), &mut storage),
        Err(WateringRunLoadError::WateringRunNotFound)
    );
}
