#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::classify_legacy_harvest_observation::*;

fn date(value: &str) -> HarvestDate {
    HarvestDate::parse_iso(value).unwrap()
}

fn legacy_run(
    storage: &mut InMemoryOrchardStorage,
    tree_id: TreeId,
    outcome: HarvestTreeOutcome,
) -> HarvestRunId {
    let id = storage
        .transaction(|storage| {
            storage.create_harvest_run(
                OrchardId(7),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit, HarvestedPart::Flower],
                date("2025-12-20"),
                &[HarvestRunTree {
                    tree_id,
                    harvested_parts: vec![HarvestedPart::Fruit, HarvestedPart::Flower],
                    period: HarvestPeriod {
                        start: date("2025-12-10"),
                        end: date("2026-01-20"),
                    },
                    outcome: None,
                }],
            )
        })
        .unwrap();
    storage
        .transaction(|storage| {
            storage.record_harvest_tree_outcome(id, tree_id, outcome)?;
            storage.complete_harvest_run(id)?;
            Ok::<_, orchard_api::hexagon::ports::OrchardStorageError>(id)
        })
        .unwrap()
}

fn event(run_id: HarvestRunId) -> LegacyHarvestObservationClassified {
    LegacyHarvestObservationClassified {
        orchard_id: OrchardId(7),
        harvest_run_id: run_id,
        tree_id: TreeId(1),
        kind: HarvestObservationKind::FruitRemaining,
        fruit_state: Some(HarvestFruitState {
            ready: true,
            unripe: true,
            wasted: true,
        }),
    }
}

#[test]
fn classifies_each_legacy_kind_using_its_original_date_parts_and_period_without_changing_the_tour()
{
    let recorded_on = date("2026-01-05");
    for outcome in [
        HarvestTreeOutcome::HarvestedEverything {
            harvested_on: recorded_on,
        },
        HarvestTreeOutcome::Deferred {
            deferred_on: recorded_on,
            retry_on: date("2026-01-12"),
        },
        HarvestTreeOutcome::DoneForWindow { recorded_on },
    ] {
        let mut storage = fixture::storage_with_fruit_window();
        let run_id = legacy_run(&mut storage, TreeId(1), outcome);
        let original = storage.harvest_run(run_id).unwrap().unwrap();
        let windows = storage
            .orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            )
            .unwrap();
        let active = fixture::start(&mut storage, "2026-09-28").unwrap();
        let active_before = storage.harvest_run(active.run_id).unwrap().unwrap();
        let id = classify_legacy_harvest_observation(event(run_id), &mut storage).unwrap();
        let points = storage
            .harvest_observations_in_orchard(OrchardId(7))
            .unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].id, id);
        assert_eq!(points[0].harvest_run_id, Some(run_id));
        assert_eq!(points[0].recorded_on, recorded_on);
        assert_eq!(
            points[0].harvested_parts,
            original.ordered_trees[0].harvested_parts
        );
        assert_eq!(points[0].period, Some(original.ordered_trees[0].period));
        assert_eq!(points[0].fruit_state, event(run_id).fruit_state);
        assert!(!points[0].reviewed);
        assert_eq!(storage.harvest_run(run_id).unwrap().unwrap(), original);
        assert_eq!(
            storage.harvest_run(active.run_id).unwrap().unwrap(),
            active_before
        );
        assert_eq!(
            storage
                .orchard_harvest_windows(
                    OrchardId(7),
                    HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
                )
                .unwrap(),
            windows
        );
    }
}

#[test]
fn an_observation_already_attached_to_this_visit_cannot_be_duplicated_even_after_review() {
    let mut storage = fixture::storage_with_fruit_window();
    let run_id = legacy_run(
        &mut storage,
        TreeId(1),
        HarvestTreeOutcome::HarvestedEverything {
            harvested_on: date("2026-01-05"),
        },
    );
    let id = classify_legacy_harvest_observation(event(run_id), &mut storage).unwrap();
    assert_eq!(
        classify_legacy_harvest_observation(event(run_id), &mut storage),
        Err(LegacyHarvestObservationError::AlreadyClassified)
    );
    storage
        .transaction(|storage| {
            storage.review_harvest_observations(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
                &[id],
            )
        })
        .unwrap();
    assert_eq!(
        classify_legacy_harvest_observation(event(run_id), &mut storage),
        Err(LegacyHarvestObservationError::AlreadyClassified)
    );
    assert_eq!(
        storage
            .harvest_observations_in_orchard(OrchardId(7))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn rejects_other_orchards_missing_trees_active_runs_modern_outcomes_and_invalid_facts() {
    for case in 0..7 {
        let mut storage = fixture::storage_with_fruit_window();
        let outcome = if case == 4 {
            HarvestTreeOutcome::EmptyOnArrival {
                recorded_on: date("2026-01-05"),
            }
        } else {
            HarvestTreeOutcome::HarvestedEverything {
                harvested_on: date("2026-01-05"),
            }
        };
        let run_id = legacy_run(&mut storage, TreeId(1), outcome);
        let mut request = event(run_id);
        let expected = match case {
            0 => {
                request.orchard_id = OrchardId(8);
                LegacyHarvestObservationError::RunNotFound
            }
            1 => {
                request.harvest_run_id = HarvestRunId(999);
                LegacyHarvestObservationError::RunNotFound
            }
            2 => {
                request.tree_id = TreeId(2);
                LegacyHarvestObservationError::TreeNotInRun
            }
            3 => {
                request.harvest_run_id = fixture::start(&mut storage, "2026-09-28").unwrap().run_id;
                LegacyHarvestObservationError::RunNotCompleted
            }
            4 => LegacyHarvestObservationError::NotLegacyVisit,
            5 => {
                request.fruit_state = None;
                LegacyHarvestObservationError::InvalidFruitState
            }
            _ => {
                request.kind = HarvestObservationKind::TooSoon;
                request.fruit_state = None;
                LegacyHarvestObservationError::InvalidFruitState
            }
        };
        assert_eq!(
            classify_legacy_harvest_observation(request, &mut storage),
            Err(expected)
        );
        assert!(
            storage
                .harvest_observations_in_orchard(OrchardId(7))
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn can_classify_as_empty_and_rolls_back_on_commit_failure() {
    let mut storage = fixture::storage_with_cultivar_window();
    let run_id = legacy_run(
        &mut storage,
        TreeId(2),
        HarvestTreeOutcome::DoneForWindow {
            recorded_on: date("2026-01-05"),
        },
    );
    let original = storage.harvest_run(run_id).unwrap().unwrap();
    let mut request = event(run_id);
    request.tree_id = TreeId(2);
    request.kind = HarvestObservationKind::EmptyOnArrival;
    request.fruit_state = None;
    classify_legacy_harvest_observation(request, &mut storage).unwrap();
    let points = storage
        .harvest_observations(
            OrchardId(7),
            HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1)),
        )
        .unwrap();
    assert_eq!(points[0].kind, HarvestObservationKind::EmptyOnArrival);
    assert_eq!(points[0].fruit_state, Some(HarvestFruitState::default()));
    assert_eq!(storage.harvest_run(run_id).unwrap().unwrap(), original);
    let mut storage = fixture::storage_with_fruit_window();
    let run_id = legacy_run(
        &mut storage,
        TreeId(1),
        HarvestTreeOutcome::HarvestedEverything {
            harvested_on: date("2026-01-05"),
        },
    );
    let original = storage.harvest_run(run_id).unwrap().unwrap();
    let mut storage = storage.with_commit_failure();
    assert_eq!(
        classify_legacy_harvest_observation(event(run_id), &mut storage),
        Err(LegacyHarvestObservationError::CouldNotClassify)
    );
    assert!(
        storage
            .harvest_observations_in_orchard(OrchardId(7))
            .unwrap()
            .is_empty()
    );
    assert_eq!(storage.harvest_run(run_id).unwrap().unwrap(), original);
}

#[test]
fn trees_in_the_same_run_are_classified_independently_and_history_keeps_the_remaining_button() {
    use orchard_api::hexagon::ports::AccessControl;
    use orchard_api::hexagon::use_cases::list_orchard_run_history::{
        OrchardRunHistoryRequested, list_orchard_run_history,
    };
    let mut storage = fixture::storage_with_cultivar_window();
    let recorded_on = date("2026-01-05");
    let snapshots = [TreeId(1), TreeId(2)].map(|tree_id| HarvestRunTree {
        tree_id,
        harvested_parts: vec![HarvestedPart::Fruit],
        period: HarvestPeriod {
            start: date("2025-12-10"),
            end: date("2026-01-20"),
        },
        outcome: None,
    });
    let run_id = storage
        .transaction(|storage| {
            storage.create_harvest_run(
                OrchardId(7),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                date("2025-12-20"),
                &snapshots,
            )
        })
        .unwrap();
    storage
        .transaction(|storage| {
            storage.record_harvest_tree_outcome(
                run_id,
                TreeId(1),
                HarvestTreeOutcome::HarvestedEverything {
                    harvested_on: recorded_on,
                },
            )?;
            storage.record_harvest_tree_outcome(
                run_id,
                TreeId(2),
                HarvestTreeOutcome::DoneForWindow { recorded_on },
            )?;
            storage.complete_harvest_run(run_id)
        })
        .unwrap();
    let original = storage.harvest_run(run_id).unwrap().unwrap();
    classify_legacy_harvest_observation(event(run_id), &mut storage).unwrap();
    let owner = storage
        .verify_credentials("owner", "password")
        .unwrap()
        .unwrap();
    let session_token = storage.create_session(owner.id).unwrap();
    let history = list_orchard_run_history(
        OrchardRunHistoryRequested {
            orchard_id: OrchardId(7),
            session_token,
        },
        &mut storage,
    )
    .unwrap();
    assert!(!history.harvest_runs[0].trees[0].needs_legacy_classification);
    assert!(history.harvest_runs[0].trees[1].needs_legacy_classification);
    let mut second = event(run_id);
    second.tree_id = TreeId(2);
    classify_legacy_harvest_observation(second, &mut storage).unwrap();
    let points = storage
        .harvest_observations_in_orchard(OrchardId(7))
        .unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].tree_id, TreeId(1));
    assert_eq!(points[1].tree_id, TreeId(2));
    assert_eq!(
        points[0].owner,
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
    );
    assert_eq!(
        points[1].owner,
        HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1))
    );
    assert_eq!(storage.harvest_run(run_id).unwrap().unwrap(), original);
}
