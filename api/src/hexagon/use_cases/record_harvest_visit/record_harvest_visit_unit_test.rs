#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::list_harvest_observations::*;
use orchard_api::hexagon::use_cases::record_harvest_visit::*;
use orchard_api::hexagon::use_cases::undo_last_harvest_tree_action::*;

#[test]
fn mixed_fruit_records_the_facts_and_keeps_the_tree_available_without_editing_windows() {
    for fruit_state in [
        HarvestFruitState {
            ready: true,
            unripe: false,
            wasted: true,
        },
        HarvestFruitState {
            ready: false,
            unripe: true,
            wasted: true,
        },
        HarvestFruitState {
            ready: true,
            unripe: true,
            wasted: true,
        },
    ] {
        let mut storage = fixture::storage_with_fruit_window();
        let run = fixture::start(&mut storage, "2026-09-28").unwrap();
        let before = storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap();
        let result = record_harvest_visit(
            event(
                run.run_id,
                HarvestObservationKind::FruitRemaining,
                Some(fruit_state),
            ),
            &mut storage,
        )
        .unwrap();
        assert_eq!(result.current_tree.unwrap().id, TreeId(2));
        assert_eq!(result.deferred_tree_count, 1);
        assert_eq!(result.done_for_window_tree_count, 0);
        let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
        assert_eq!(points[0].fruit_state, Some(fruit_state));
        assert!(!points[0].reviewed);
        assert!(
            matches!(storage.harvest_run(run.run_id).unwrap().unwrap().ordered_trees[0].outcome,
            Some(HarvestTreeOutcome::FruitRemaining { retry_on, .. }) if retry_on.to_string() == "2026-10-05")
        );
        assert_eq!(
            storage
                .orchard_harvest_windows(OrchardId(7), owner())
                .unwrap(),
            before
        );
    }
}

#[test]
fn the_three_empty_tree_reasons_preserve_production_and_only_fruit_provides_window_evidence() {
    for (kind, ready, wasted, timing) in [
        (HarvestObservationKind::HarvestedAll, true, false, true),
        (HarvestObservationKind::EmptyOnArrival, false, false, false),
        (HarvestObservationKind::AllWasted, false, true, true),
    ] {
        let mut storage = fixture::storage_with_fruit_window();
        let run = fixture::start(&mut storage, "2026-09-28").unwrap();
        record_harvest_visit(event(run.run_id, kind, None), &mut storage).unwrap();
        let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
        assert_eq!(
            points[0].fruit_state,
            Some(HarvestFruitState {
                ready,
                unripe: false,
                wasted
            })
        );
        assert_eq!(points[0].kind, kind);
        assert_eq!(
            list_harvest_observations(OrchardId(7), owner(), &mut storage)
                .unwrap()
                .len(),
            usize::from(timing)
        );
        assert_eq!(
            storage
                .harvest_run(run.run_id)
                .unwrap()
                .unwrap()
                .ordered_trees[0]
                .outcome
                .is_some(),
            true
        );
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
                .harvest_observations(OrchardId(7), owner())
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn invalid_or_contradictory_fruit_answers_never_advance_or_save() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    for (kind, facts) in [
        (HarvestObservationKind::FruitRemaining, None),
        (
            HarvestObservationKind::FruitRemaining,
            Some(HarvestFruitState {
                ready: false,
                unripe: false,
                wasted: false,
            }),
        ),
        (
            HarvestObservationKind::FruitRemaining,
            Some(HarvestFruitState {
                ready: false,
                unripe: false,
                wasted: true,
            }),
        ),
        (
            HarvestObservationKind::EmptyOnArrival,
            Some(HarvestFruitState {
                ready: true,
                unripe: false,
                wasted: false,
            }),
        ),
    ] {
        assert_eq!(
            record_harvest_visit(event(run.run_id, kind, facts), &mut storage),
            Err(HarvestVisitRecordingError::InvalidFruitState)
        );
    }
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner())
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

#[test]
fn save_failure_leaves_production_record_and_tour_unchanged() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    let mut storage = storage.with_commit_failure();
    assert!(
        record_harvest_visit(
            event(run.run_id, HarvestObservationKind::EmptyOnArrival, None),
            &mut storage
        )
        .is_err()
    );
    assert!(
        storage
            .harvest_observations(OrchardId(7), owner())
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

fn owner() -> HarvestScheduleOwner {
    HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
}
fn event(
    run_id: HarvestRunId,
    kind: HarvestObservationKind,
    fruit_state: Option<HarvestFruitState>,
) -> HarvestVisitRecorded {
    HarvestVisitRecorded {
        orchard_id: OrchardId(7),
        harvest_run_id: run_id,
        tree_id: TreeId(1),
        action_date: "2026-09-28".into(),
        kind,
        fruit_state,
    }
}

#[test]
fn empty_reasons_suppress_the_current_window_but_allow_a_second_window_in_the_same_year() {
    use orchard_api::hexagon::use_cases::list_harvest_candidates::*;
    use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::*;
    for kind in [
        HarvestObservationKind::HarvestedAll,
        HarvestObservationKind::EmptyOnArrival,
        HarvestObservationKind::AllWasted,
    ] {
        let mut storage = fixture::storage_with_fruit_window();
        replace_orchard_harvest_windows(
            OrchardHarvestWindowsReplaced {
                orchard_id: OrchardId(7),
                owner: owner(),
                reference_region: "France".into(),
                windows: vec![
                    AnnualHarvestWindowChanged {
                        start_month: 9,
                        start_day: 1,
                        end_month: 9,
                        end_day: 30,
                        harvested_part: HarvestedPart::Fruit,
                    },
                    AnnualHarvestWindowChanged {
                        start_month: 11,
                        start_day: 1,
                        end_month: 11,
                        end_day: 30,
                        harvested_part: HarvestedPart::Fruit,
                    },
                ],
            },
            &mut storage,
        )
        .unwrap();
        let run = fixture::start(&mut storage, "2026-09-28").unwrap();
        record_harvest_visit(event(run.run_id, kind, None), &mut storage).unwrap();
        for (date, expected) in [
            ("2026-09-29", vec![TreeId(2)]),
            ("2026-11-01", vec![TreeId(1), TreeId(2)]),
        ] {
            let candidates = list_harvest_candidates(
                HarvestCandidatesRequested {
                    orchard_id: OrchardId(7),
                    harvested_parts: vec![HarvestedPart::Fruit],
                    action_date: date.into(),
                },
                &mut storage,
            )
            .unwrap();
            assert_eq!(
                candidates
                    .iter()
                    .map(|candidate| candidate.tree_id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn mixed_fruit_returns_for_a_later_visit_in_the_same_window() {
    use orchard_api::hexagon::use_cases::list_harvest_candidates::*;
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-01").unwrap();
    let mut visit = event(
        run.run_id,
        HarvestObservationKind::FruitRemaining,
        Some(HarvestFruitState {
            ready: false,
            unripe: true,
            wasted: true,
        }),
    );
    visit.action_date = "2026-09-01".into();
    record_harvest_visit(visit, &mut storage).unwrap();
    for (date, expected) in [
        ("2026-09-07", vec![TreeId(2)]),
        ("2026-09-08", vec![TreeId(1), TreeId(2)]),
    ] {
        let candidates = list_harvest_candidates(
            HarvestCandidatesRequested {
                orchard_id: OrchardId(7),
                harvested_parts: vec![HarvestedPart::Fruit],
                action_date: date.into(),
            },
            &mut storage,
        )
        .unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.tree_id)
                .collect::<Vec<_>>(),
            expected
        );
    }
}
