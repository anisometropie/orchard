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

#[test]
fn review_one_window_preserves_other_windows_and_the_same_trees_other_observations() {
    let (mut storage, ids) = observations_for_two_windows();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    review_harvest_observations_for_window(
        review(ids[..2].to_vec(), Some(change(15))),
        september_window(),
        &mut storage,
    )
    .unwrap();
    let windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert_eq!(windows.len(), original.len());
    assert_eq!(windows[0].start.day, 15);
    assert_eq!(windows[0].data_origin, HarvestDataOrigin::FieldObservation);
    assert_eq!(windows[0].source_url, None);
    assert_eq!(windows[1..], original[1..]);
    let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    assert_eq!(
        points
            .iter()
            .map(|point| point.reviewed)
            .collect::<Vec<_>>(),
        vec![true, true, false, false]
    );
    assert_eq!(points[0].tree_id, points[2].tree_id);
    let mut june = september_window();
    june.start_month = 6;
    june.end_month = 6;
    review_harvest_observations_for_window(review(vec![ids[2]], None), june, &mut storage).unwrap();
    assert!(storage.harvest_observations(OrchardId(7), owner()).unwrap()[2].reviewed);
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        windows
    );
}

#[test]
fn a_window_review_rejects_other_periods_parts_and_removed_windows_without_mutation() {
    let (mut storage, ids) = observations_for_two_windows();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    for selection in [vec![ids[0], ids[2]], vec![ids[3]]] {
        assert_eq!(
            review_harvest_observations_for_window(
                review(selection, Some(change(15))),
                september_window(),
                &mut storage,
            ),
            Err(HarvestObservationReviewError::InvalidObservationSelection)
        );
    }
    let mut removed = september_window();
    removed.start_day = 2;
    assert_eq!(
        review_harvest_observations_for_window(
            review(vec![ids[0]], Some(change(15))),
            removed,
            &mut storage,
        ),
        Err(HarvestObservationReviewError::InvalidObservationSelection)
    );
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
            .all(|point| !point.reviewed)
    );
}

#[test]
fn a_window_review_cannot_add_or_delete_windows() {
    let (mut storage, ids) = observations_for_two_windows();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    for count in [0, 2] {
        let mut replacement = change(15);
        replacement.windows = vec![september_window(); count];
        assert_eq!(
            review_harvest_observations_for_window(
                review(vec![ids[0]], Some(replacement)),
                september_window(),
                &mut storage,
            ),
            Err(HarvestObservationReviewError::InvalidObservationSelection)
        );
    }
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
            .all(|point| !point.reviewed)
    );
}

#[test]
fn failed_window_review_rolls_back_the_single_edit_and_review_status() {
    let (mut storage, ids) = observations_for_two_windows();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    let mut storage = storage.with_commit_failure();
    assert!(
        review_harvest_observations_for_window(
            review(ids[..2].to_vec(), Some(change(15))),
            september_window(),
            &mut storage,
        )
        .is_err()
    );
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
            .all(|point| !point.reviewed)
    );
}

fn september_window() -> AnnualHarvestWindowChanged {
    change(1).windows[0]
}

fn observations_for_two_windows() -> (InMemoryOrchardStorage, Vec<u64>) {
    let (mut storage, mut ids, _) = observations();
    let mut windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    windows[0].data_origin = HarvestDataOrigin::ExternalReference;
    windows[0].source_url = Some("https://example.com/september".into());
    let mut june = windows[0].clone();
    june.start.month = 6;
    june.end.month = 6;
    june.reference_region = Some("Lyon".into());
    june.source_url = Some("https://example.com/june".into());
    let mut flower = windows[0].clone();
    flower.harvested_part = HarvestedPart::Flower;
    windows.extend([june, flower]);
    let mut point = storage.harvest_observations(OrchardId(7), owner()).unwrap()[0].clone();
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(OrchardId(7), owner(), windows)?;
            point.period.as_mut().unwrap().start.month = 6;
            point.period.as_mut().unwrap().end.month = 6;
            point.recorded_on.month = 6;
            ids.push(storage.save_harvest_observation(point.clone())?);
            point.period.as_mut().unwrap().start.month = 9;
            point.period.as_mut().unwrap().end.month = 9;
            point.recorded_on.month = 9;
            point.harvested_parts = vec![HarvestedPart::Flower];
            ids.push(storage.save_harvest_observation(point)?);
            Ok::<_, orchard_api::hexagon::ports::OrchardStorageError>(())
        })
        .unwrap();
    (storage, ids)
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

#[test]
fn saving_an_edited_window_can_explicitly_validate_its_new_dates_and_review_only_its_points() {
    let (mut storage, ids) = observations_for_two_windows();
    let original = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    let before_points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    review_harvest_observations_and_validate_window(
        review(ids[..2].to_vec(), Some(change(15))),
        september_window(),
        &mut storage,
    )
    .unwrap();
    let windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert_eq!(windows[0].start.day, 15);
    assert!(windows[0].validated);
    assert_eq!(windows[0].data_origin, HarvestDataOrigin::FieldObservation);
    assert_eq!(windows[1..], original[1..]);
    let mut expected = before_points;
    expected[0].reviewed = true;
    expected[1].reviewed = true;
    assert_eq!(
        storage.harvest_observations(OrchardId(7), owner()).unwrap(),
        expected
    );
}

#[test]
fn edited_dates_remain_unvalidated_without_the_explicit_checkmark() {
    let (mut storage, ids) = observations_for_two_windows();
    review_harvest_observations_for_window(
        review(ids[..2].to_vec(), Some(change(15))),
        september_window(),
        &mut storage,
    )
    .unwrap();
    assert!(
        !storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
}

#[test]
fn an_edited_window_can_be_validated_using_already_reviewed_observations() {
    let (mut storage, ids) = observations_for_two_windows();
    review_harvest_observations_for_window(
        review(ids[..2].to_vec(), None),
        september_window(),
        &mut storage,
    )
    .unwrap();
    let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    review_harvest_observations_and_validate_window(
        review(vec![], Some(change(15))),
        september_window(),
        &mut storage,
    )
    .unwrap();
    assert!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
    assert_eq!(
        storage.harvest_observations(OrchardId(7), owner()).unwrap(),
        points
    );
    assert_eq!(
        review_harvest_observations_and_validate_window(
            review(vec![], None),
            september_window(),
            &mut storage
        ),
        Err(HarvestObservationReviewError::InvalidObservationSelection)
    );
}

#[test]
fn a_failed_combined_save_never_changes_the_dates_validation_or_reviewed_points() {
    let (storage, ids) = observations_for_two_windows();
    let mut storage = storage.with_commit_failure();
    let windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    assert_eq!(
        review_harvest_observations_and_validate_window(
            review(ids[..2].to_vec(), Some(change(15))),
            september_window(),
            &mut storage
        ),
        Err(HarvestObservationReviewError::CouldNotReview)
    );
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        windows
    );
    assert_eq!(
        storage.harvest_observations(OrchardId(7), owner()).unwrap(),
        points
    );
}
