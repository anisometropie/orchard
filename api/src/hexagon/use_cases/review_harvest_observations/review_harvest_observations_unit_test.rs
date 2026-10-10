#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::record_harvest_observation::*;
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::*;
use orchard_api::hexagon::use_cases::review_harvest_observations::*;
use orchard_api::hexagon::use_cases::undo_last_harvest_tree_action::*;

#[test]
fn keep_windows_reviews_only_selected_points_and_preserves_all_evidence() {
    let (mut storage, ids, _) = observations();
    let before = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    review_harvest_observations(review(vec![ids[0]], None), &mut storage).unwrap();
    let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    assert_eq!(points.len(), 2);
    assert!(points[0].reviewed);
    assert!(!points[1].reviewed);
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        before
    );
}

#[test]
fn update_windows_and_review_are_one_atomic_operation() {
    let (mut storage, ids, _) = observations();
    review_harvest_observations(review(ids, Some(change(15))), &mut storage).unwrap();
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner())
            .unwrap()
            .iter()
            .all(|p| p.reviewed)
    );
    let windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert_eq!(windows[0].start.day, 15);
    assert_eq!(windows[0].data_origin, HarvestDataOrigin::FieldObservation);
}

#[test]
fn invalid_selection_or_dates_leave_windows_and_review_status_untouched() {
    let (mut storage, ids, _) = observations();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    for selected in [vec![], vec![ids[0], ids[0]], vec![999], vec![ids[0], 999]] {
        assert!(
            review_harvest_observations(review(selected, Some(change(15))), &mut storage).is_err()
        );
    }
    assert!(
        review_harvest_observations(review(ids.clone(), Some(change(32))), &mut storage).is_err()
    );
    let mut other_orchard = review(ids.clone(), None);
    other_orchard.orchard_id = OrchardId(8);
    assert!(review_harvest_observations(other_orchard, &mut storage).is_err());
    let mut other_owner = review(ids.clone(), None);
    other_owner.owner = HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1));
    assert!(review_harvest_observations(other_owner, &mut storage).is_err());
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        original
    );
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner())
            .unwrap()
            .iter()
            .all(|p| !p.reviewed)
    );
    review_harvest_observations(review(vec![ids[0]], None), &mut storage).unwrap();
    assert_eq!(
        review_harvest_observations(review(ids, Some(change(15))), &mut storage),
        Err(HarvestObservationReviewError::InvalidObservationSelection)
    );
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        original
    );
}

#[test]
fn commit_failure_rolls_back_changed_windows_and_reviewed_points() {
    let (mut storage, ids, _) = observations();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    let mut storage = storage.with_commit_failure();
    assert!(review_harvest_observations(review(ids, Some(change(15))), &mut storage).is_err());
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        original
    );
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner())
            .unwrap()
            .iter()
            .all(|p| !p.reviewed)
    );
}

#[test]
fn reviewed_evidence_cannot_be_removed_by_undoing_the_visit() {
    let (mut storage, ids, run_id) = observations();
    review_harvest_observations(review(vec![ids[1]], None), &mut storage).unwrap();
    assert_eq!(
        undo_last_harvest_tree_action(
            LastHarvestTreeActionUndone {
                orchard_id: OrchardId(7),
                harvest_run_id: run_id,
                action_date: "2026-09-28".into(),
            },
            &mut storage
        ),
        Err(LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone)
    );
    assert_eq!(
        storage
            .harvest_observations(OrchardId(7), owner())
            .unwrap()
            .len(),
        2
    );
    assert!(storage.harvest_run(run_id).unwrap().unwrap().completed);
}

fn observations() -> (InMemoryOrchardStorage, Vec<u64>, HarvestRunId) {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    for (tree_id, kind) in [
        (TreeId(1), HarvestObservationKind::TooSoon),
        (TreeId(2), HarvestObservationKind::TooLate),
    ] {
        record_harvest_observation(
            HarvestObservationRecorded {
                orchard_id: OrchardId(7),
                harvest_run_id: run.run_id,
                tree_id,
                action_date: "2026-09-28".into(),
                kind,
            },
            &mut storage,
        )
        .unwrap();
    }
    let ids = storage
        .harvest_observations(OrchardId(7), owner())
        .unwrap()
        .iter()
        .map(|p| p.id)
        .collect();
    (storage, ids, run.run_id)
}
fn owner() -> HarvestScheduleOwner {
    HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
}
fn review(
    ids: Vec<u64>,
    window_change: Option<OrchardHarvestWindowsReplaced>,
) -> HarvestObservationsReviewed {
    HarvestObservationsReviewed {
        orchard_id: OrchardId(7),
        owner: owner(),
        observation_ids: ids,
        window_change,
    }
}
fn change(start_day: u8) -> OrchardHarvestWindowsReplaced {
    OrchardHarvestWindowsReplaced {
        orchard_id: OrchardId(7),
        owner: owner(),
        reference_region: "France".into(),
        windows: vec![AnnualHarvestWindowChanged {
            start_month: 9,
            start_day,
            end_month: 9,
            end_day: 30,
            harvested_part: HarvestedPart::Fruit,
        }],
    }
}
