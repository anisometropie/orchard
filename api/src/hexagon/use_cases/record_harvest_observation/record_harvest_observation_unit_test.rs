#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::record_harvest_observation::*;
use orchard_api::hexagon::use_cases::undo_last_harvest_tree_action::*;

#[test]
fn too_soon_near_window_end_saves_evidence_and_advances_without_extending_windows() {
    let mut storage = fixture::storage_with_fruit_window();
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    let windows = storage
        .orchard_harvest_windows(OrchardId(7), owner)
        .unwrap();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    let progress = record_harvest_observation(
        event(run.run_id, TreeId(1), HarvestObservationKind::TooSoon),
        &mut storage,
    )
    .unwrap();
    assert_eq!(progress.current_tree.unwrap().id, TreeId(2));
    assert_eq!(progress.deferred_tree_count, 1);
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner)
            .unwrap(),
        windows
    );
    let points = storage.harvest_observations(OrchardId(7), owner).unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].kind, HarvestObservationKind::TooSoon);
    assert_eq!(points[0].recorded_on.to_string(), "2026-09-28");
    assert_eq!(points[0].harvested_parts, vec![HarvestedPart::Fruit]);
    assert!(!points[0].reviewed);
    assert!(
        matches!(storage.harvest_run(run.run_id).unwrap().unwrap().ordered_trees[0].outcome,
        Some(HarvestTreeOutcome::TooSoon { retry_on, .. }) if retry_on.to_string() == "2026-10-05")
    );
}

#[test]
fn too_late_is_distinct_from_harvested_and_undo_removes_the_unreviewed_point() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    let progress = record_harvest_observation(
        event(run.run_id, TreeId(1), HarvestObservationKind::TooLate),
        &mut storage,
    )
    .unwrap();
    assert_eq!(progress.harvested_tree_count, 0);
    assert_eq!(progress.done_for_window_tree_count, 1);
    let restored = undo_last_harvest_tree_action(
        LastHarvestTreeActionUndone {
            orchard_id: OrchardId(7),
            harvest_run_id: run.run_id,
            action_date: "2026-09-28".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(restored.current_tree.unwrap().id, TreeId(1));
    assert!(
        storage
            .harvest_observations(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
            )
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reject_wrong_tree_orchard_date_and_duplicate_without_saving_points() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    let mut wrong_tree = event(run.run_id, TreeId(2), HarvestObservationKind::TooLate);
    assert_eq!(
        record_harvest_observation(wrong_tree, &mut storage),
        Err(HarvestObservationRecordingError::TreeIsNotCurrent)
    );
    wrong_tree = event(run.run_id, TreeId(1), HarvestObservationKind::TooLate);
    wrong_tree.orchard_id = OrchardId(8);
    assert_eq!(
        record_harvest_observation(wrong_tree, &mut storage),
        Err(HarvestObservationRecordingError::HarvestRunNotFound)
    );
    for date in ["not-a-date", "2026-09-27", "2026-10-01"] {
        let mut invalid = event(run.run_id, TreeId(1), HarvestObservationKind::TooLate);
        invalid.action_date = date.into();
        assert!(record_harvest_observation(invalid, &mut storage).is_err());
    }
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner)
            .unwrap()
            .is_empty()
    );
    record_harvest_observation(
        event(run.run_id, TreeId(1), HarvestObservationKind::TooLate),
        &mut storage,
    )
    .unwrap();
    assert!(
        record_harvest_observation(
            event(run.run_id, TreeId(1), HarvestObservationKind::TooSoon),
            &mut storage
        )
        .is_err()
    );
    assert_eq!(
        storage
            .harvest_observations(OrchardId(7), owner)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn failed_commit_rolls_back_both_point_and_tree_outcome() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    let mut storage = storage.with_commit_failure();
    assert!(
        record_harvest_observation(
            event(run.run_id, TreeId(1), HarvestObservationKind::TooLate),
            &mut storage
        )
        .is_err()
    );
    assert!(
        storage
            .harvest_observations(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
            )
            .unwrap()
            .is_empty()
    );
    assert!(
        storage
            .harvest_run(run.run_id)
            .unwrap()
            .unwrap()
            .ordered_trees[0]
            .outcome
            .is_none()
    );
}

fn event(
    run_id: HarvestRunId,
    tree_id: TreeId,
    kind: HarvestObservationKind,
) -> HarvestObservationRecorded {
    HarvestObservationRecorded {
        orchard_id: OrchardId(7),
        harvest_run_id: run_id,
        tree_id,
        action_date: "2026-09-28".into(),
        kind,
    }
}
