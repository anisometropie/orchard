#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod fixture;

use orchard_api::hexagon::models::{OrchardId, TreeId, WateringRunId, WateringRunTarget};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::pause_watering_run::{
    WateringRunPauseRequested, pause_watering_run,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{TreeWatered, record_tree_watered};
use orchard_api::hexagon::use_cases::resume_watering_run::{
    WateringRunResumeError, WateringRunResumeRequested, resume_watering_run,
};

#[test]
fn resume_continues_from_the_last_watered_tree_and_completes_the_same_run() {
    let (mut storage, observer) = fixture::storage();
    record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();
    pause_watering_run(
        WateringRunPauseRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    let resumed = resume_watering_run(
        WateringRunResumeRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert!(!resumed.paused);
    assert_eq!(resumed.watered_tree_count, 1);
    assert_eq!(resumed.next_tree.unwrap().id, TreeId(2));
    record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
            tree_id: TreeId(2),
        },
        &mut storage,
    )
    .unwrap();
    assert!(observer.watering_run(WateringRunId(1)).unwrap().completed);
    assert_eq!(
        resume_watering_run(
            WateringRunResumeRequested {
                orchard_id: OrchardId(7),
                watering_run_id: WateringRunId(1)
            },
            &mut storage
        ),
        Err(WateringRunResumeError::WateringRunAlreadyCompleted)
    );
}

#[test]
fn resume_rejects_another_orchard_and_keeps_paused_progress_when_the_same_target_is_active() {
    let (mut storage, observer) = fixture::storage();
    pause_watering_run(
        WateringRunPauseRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(
        resume_watering_run(
            WateringRunResumeRequested {
                orchard_id: OrchardId(8),
                watering_run_id: WateringRunId(1)
            },
            &mut storage
        ),
        Err(WateringRunResumeError::WateringRunNotFound)
    );
    storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(2)],
            )
        })
        .unwrap();
    let before = observer.watering_run(WateringRunId(1));
    assert_eq!(
        resume_watering_run(
            WateringRunResumeRequested {
                orchard_id: OrchardId(7),
                watering_run_id: WateringRunId(1)
            },
            &mut storage
        ),
        Err(WateringRunResumeError::AnotherWateringRunIsActive)
    );
    assert_eq!(observer.watering_run(WateringRunId(1)), before);
}

#[test]
fn resume_allows_an_unrelated_run_to_continue() {
    let (mut storage, observer) = fixture::storage();
    pause_watering_run(
        WateringRunPauseRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    let other_id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::DangerTrees,
                None,
                None,
                &[TreeId(2)],
            )
        })
        .unwrap();
    let before = observer.watering_run(other_id);
    let resumed = resume_watering_run(
        WateringRunResumeRequested {
            orchard_id: OrchardId(7),
            watering_run_id: WateringRunId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert!(!resumed.paused);
    assert_eq!(observer.watering_run(other_id), before);
    assert_eq!(
        storage
            .unfinished_watering_runs(OrchardId(7))
            .unwrap()
            .len(),
        2
    );
}
