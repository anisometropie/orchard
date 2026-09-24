#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod fixture;

use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{OrchardId, TreeId, WateringRunId, WateringRunTarget};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::load_active_watering_run::load_active_watering_run;
use orchard_api::hexagon::use_cases::pause_watering_run::{
    WateringRunPauseError, WateringRunPauseRequested, pause_watering_run,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{
    TreeWatered, TreeWateredError, record_tree_watered,
};

#[test]
fn pause_preserves_progress_and_rejects_further_watering() {
    let (mut storage, observer) = fixture::storage();
    let watered = record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();
    let paused = pause_watering_run(
        WateringRunPauseRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert!(paused.paused);
    assert_eq!(paused.route, watered.route);
    assert_eq!(paused.watered_tree_count, 1);
    assert_eq!(paused.next_tree, watered.next_tree);
    assert_eq!(
        load_active_watering_run(OrchardId(7), &mut storage),
        Ok(None)
    );
    let before = observer.watering_run(WateringRunId(1)).unwrap();
    assert_eq!(
        record_tree_watered(
            TreeWatered {
                orchard_id: OrchardId(7),
                watering_run_id: WateringRunId(1),
                tree_id: TreeId(2)
            },
            &mut storage
        ),
        Err(TreeWateredError::WateringRunIsPaused)
    );
    assert_eq!(observer.watering_run(WateringRunId(1)), Some(before));
}

#[test]
fn pause_cannot_change_a_run_in_another_orchard_or_a_completed_run() {
    let (mut storage, observer) = fixture::storage();
    assert_eq!(
        pause_watering_run(
            WateringRunPauseRequested {
                orchard_id: OrchardId(8),
                watering_run_id: WateringRunId(1)
            },
            &mut storage
        ),
        Err(WateringRunPauseError::WateringRunNotFound)
    );
    assert!(!observer.watering_run(WateringRunId(1)).unwrap().paused);
    storage
        .transaction(|orchard| orchard.complete_watering_run(WateringRunId(1)))
        .unwrap();
    assert_eq!(
        pause_watering_run(
            WateringRunPauseRequested {
                orchard_id: OrchardId(7),
                watering_run_id: WateringRunId(1)
            },
            &mut storage
        ),
        Err(WateringRunPauseError::WateringRunAlreadyCompleted)
    );
}

#[test]
fn failure_after_staging_pause_rolls_it_back() {
    let mut storage = InMemoryOrchardStorage::failing_when_reading_trees();
    let id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    let before = storage.watering_run(id).unwrap();
    assert_eq!(
        pause_watering_run(
            WateringRunPauseRequested {
                orchard_id: OrchardId(7),
                watering_run_id: id
            },
            &mut storage
        ),
        Err(WateringRunPauseError::WateringRunCouldNotBePaused)
    );
    assert_eq!(storage.watering_run(id).unwrap(), before);
}
