#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod fixture;

use orchard_api::hexagon::models::{OrchardId, TreeId, WateringRunId, WateringRunTarget};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::mark_watering_tree_dead::{
    WateringTreeMarkedDead, WateringTreeMarkedDeadError, mark_watering_tree_dead,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{
    TreeWatered, TreeWateredError, record_tree_watered,
};

fn event(run: u64, tree: u64) -> WateringTreeMarkedDead {
    WateringTreeMarkedDead {
        orchard_id: OrchardId(7),
        watering_run_id: WateringRunId(run),
        tree_id: TreeId(tree),
    }
}

#[test]
fn mark_the_current_tree_dead_clear_danger_and_skip_it_without_recording_watering() {
    let (mut storage, observer) = fixture::storage();
    storage
        .transaction(|orchard| orchard.change_tree_danger(TreeId(1), true))
        .unwrap();
    let progress = mark_watering_tree_dead(event(1, 1), &mut storage).unwrap();
    assert_eq!(progress.watered_tree_count, 0);
    assert_eq!(progress.skipped_tree_count, 1);
    assert_eq!(progress.handled_tree_count, 1);
    assert_eq!(progress.skipped_tree_ids, vec![TreeId(1)]);
    assert!(progress.watered_tree_ids.is_empty());
    assert_eq!(progress.next_tree.unwrap().id, TreeId(2));
    assert!(!observer.trees()[0].is_alive);
    assert!(!observer.trees()[0].is_in_danger);
    let before = observer.watering_run(WateringRunId(1));
    assert_eq!(
        mark_watering_tree_dead(event(1, 1), &mut storage),
        Err(WateringTreeMarkedDeadError::TreeIsNotNext)
    );
    assert_eq!(
        record_tree_watered(
            TreeWatered {
                orchard_id: OrchardId(7),
                watering_run_id: WateringRunId(1),
                tree_id: TreeId(1)
            },
            &mut storage
        ),
        Err(TreeWateredError::TreeIsNotNext)
    );
    assert_eq!(observer.watering_run(WateringRunId(1)), before);
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
    let history = storage.completed_watering_runs(OrchardId(7)).unwrap();
    assert_eq!(history[0].trees[0].watered_at_unix_seconds, None);
    assert!(history[0].trees[0].skipped_at_unix_seconds.is_some());
    assert!(history[0].trees[1].watered_at_unix_seconds.is_some());
    assert_eq!(history[0].trees[1].skipped_at_unix_seconds, None);
}

#[test]
fn skipping_the_last_tree_completes_the_run_without_counting_it_as_watered() {
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
    let finished = mark_watering_tree_dead(event(1, 2), &mut storage).unwrap();
    assert_eq!(finished.next_tree, None);
    assert_eq!(finished.watered_tree_count, 1);
    assert_eq!(finished.skipped_tree_count, 1);
    assert_eq!(finished.handled_tree_count, 2);
    assert!(observer.watering_run(WateringRunId(1)).unwrap().completed);
}

#[test]
fn propagate_skips_to_later_route_positions_and_paused_runs_but_preserve_past_watering() {
    let (mut storage, observer) = fixture::storage();
    let other = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::DangerTrees,
                None,
                None,
                &[TreeId(2), TreeId(1)],
            )
        })
        .unwrap();
    let paused = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("Paused".into()),
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| orchard.set_watering_run_paused(paused, true))
        .unwrap();
    let previous = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("Previous".into()),
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| {
            orchard.mark_watering_tree_watered(previous, TreeId(1))?;
            orchard.complete_watering_run(previous)
        })
        .unwrap();
    let history = storage.completed_watering_runs(OrchardId(7)).unwrap();
    mark_watering_tree_dead(event(1, 1), &mut storage).unwrap();
    let other = observer.watering_run(other).unwrap();
    assert_eq!(other.skipped_tree_ids, vec![TreeId(1)]);
    assert_eq!(other.next_tree_id(), Some(TreeId(2)));
    assert!(!other.completed);
    let paused = observer.watering_run(paused).unwrap();
    assert!(paused.completed);
    assert!(!paused.paused);
    assert_eq!(paused.skipped_tree_ids, vec![TreeId(1)]);
    assert_eq!(
        storage
            .completed_watering_runs(OrchardId(7))
            .unwrap()
            .into_iter()
            .find(|run| run.id == previous)
            .unwrap(),
        history[0]
    );
}

#[test]
fn reject_other_orchards_non_current_paused_and_completed_runs_without_mutation() {
    let (mut storage, observer) = fixture::storage();
    let trees_before = observer.trees();
    let run_before = observer.watering_run(WateringRunId(1));
    let mut other_orchard = event(1, 1);
    other_orchard.orchard_id = OrchardId(8);
    assert_eq!(
        mark_watering_tree_dead(other_orchard, &mut storage),
        Err(WateringTreeMarkedDeadError::WateringRunNotFound)
    );
    assert_eq!(
        mark_watering_tree_dead(event(1, 2), &mut storage),
        Err(WateringTreeMarkedDeadError::TreeIsNotNext)
    );
    assert_eq!(observer.watering_run(WateringRunId(1)), run_before);
    storage
        .transaction(|orchard| orchard.set_watering_run_paused(WateringRunId(1), true))
        .unwrap();
    assert_eq!(
        mark_watering_tree_dead(event(1, 1), &mut storage),
        Err(WateringTreeMarkedDeadError::WateringRunIsPaused)
    );
    storage
        .transaction(|orchard| orchard.complete_watering_run(WateringRunId(1)))
        .unwrap();
    assert_eq!(
        mark_watering_tree_dead(event(1, 1), &mut storage),
        Err(WateringTreeMarkedDeadError::WateringRunAlreadyCompleted)
    );
    assert_eq!(observer.trees(), trees_before);
}

#[test]
fn rollback_restores_tree_condition_and_all_overlapping_progress_when_commit_fails() {
    let (mut storage, observer) = fixture::storage();
    storage
        .transaction(|orchard| orchard.change_tree_danger(TreeId(1), true))
        .unwrap();
    let other = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::DangerTrees,
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    let trees_before = observer.trees();
    let before = observer.watering_run(WateringRunId(1));
    let other_before = observer.watering_run(other);
    let mut storage = storage.with_commit_failure();
    assert_eq!(
        mark_watering_tree_dead(event(1, 1), &mut storage),
        Err(WateringTreeMarkedDeadError::TreeCouldNotBeMarkedDead)
    );
    assert_eq!(observer.trees(), trees_before);
    assert_eq!(observer.watering_run(WateringRunId(1)), before);
    assert_eq!(observer.watering_run(other), other_before);
}
