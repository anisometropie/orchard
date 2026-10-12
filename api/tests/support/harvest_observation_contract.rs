use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};
use orchard_api::hexagon::use_cases::record_harvest_observation::*;
use orchard_api::hexagon::use_cases::review_harvest_observations::*;
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::*;

pub fn assert_observations_are_scoped_ordered_and_atomic(storage: &mut impl OrchardStorage, run_id: HarvestRunId, orchard_id: OrchardId, species: HarvestScheduleOwner, cultivar: HarvestScheduleOwner) {
    for (tree_id, kind) in [(TreeId(1), HarvestObservationKind::TooSoon), (TreeId(2), HarvestObservationKind::TooLate)] {
        record_harvest_observation(HarvestObservationRecorded { orchard_id, harvest_run_id: run_id, tree_id,
            action_date: "2026-09-28".into(), kind }, storage).unwrap();
    }
    let species_points = storage.harvest_observations(orchard_id, species).unwrap();
    let cultivar_points = storage.harvest_observations(orchard_id, cultivar).unwrap();
    assert_eq!(species_points.len(), 1);
    assert_eq!(cultivar_points.len(), 1);
    assert_eq!(species_points[0].tree_id, TreeId(1));
    assert_eq!(cultivar_points[0].tree_id, TreeId(2));
    assert!(storage.harvest_observations(OrchardId(99), species).unwrap().is_empty());
    let first_id = species_points[0].id;
    let second_id = cultivar_points[0].id;
    assert!(first_id < second_id);
    assert!(storage.harvest_run(run_id).unwrap().unwrap().completed);
    let original = storage.orchard_harvest_windows(orchard_id, species).unwrap();
    assert_eq!(original[0].end.day, 30);
    assert_eq!(original[0].start.day, 1);
    assert_eq!(storage.harvest_run(run_id).unwrap().unwrap().ordered_trees[0].outcome,
        Some(HarvestTreeOutcome::TooSoon { recorded_on: HarvestDate::new(2026, 9, 28).unwrap(), retry_on: HarvestDate::new(2026, 10, 5).unwrap() }));

    let rolled_back: Result<(), OrchardStorageError> = storage.transaction(|storage| {
        storage.review_harvest_observations(orchard_id, species, &[first_id])?;
        assert!(storage.harvest_observations(orchard_id, species)?[0].reviewed);
        storage.replace_orchard_harvest_windows(orchard_id, species, vec![])?;
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    });
    assert!(rolled_back.is_err());
    assert_eq!(storage.harvest_observations(orchard_id, species).unwrap(), species_points);
    assert_eq!(storage.orchard_harvest_windows(orchard_id, species).unwrap(), original);
    let mixed: Result<(), OrchardStorageError> = storage.transaction(|storage|
        storage.review_harvest_observations(orchard_id, species, &[first_id, second_id]));
    assert!(mixed.is_err());
    assert!(!storage.harvest_observations(orchard_id, species).unwrap()[0].reviewed);

    review_harvest_observations(HarvestObservationsReviewed {
        orchard_id, owner: species, observation_ids: vec![first_id],
        window_change: Some(OrchardHarvestWindowsReplaced { orchard_id, owner: species,
            reference_region: "France".into(), windows: vec![AnnualHarvestWindowChanged {
                start_month: 9, start_day: 15, end_month: 9, end_day: 30, harvested_part: HarvestedPart::Fruit,
            }] }),
    }, storage).unwrap();
    assert!(storage.harvest_observations(orchard_id, species).unwrap()[0].reviewed);
    assert!(!storage.harvest_observations(orchard_id, cultivar).unwrap()[0].reviewed);
    assert_eq!(storage.orchard_harvest_windows(orchard_id, species).unwrap()[0].start.day, 15);
    assert!(storage.transaction(|storage| storage.delete_unreviewed_harvest_observation(first_id)).is_err());
    storage.transaction(|storage| storage.delete_unreviewed_harvest_observation(second_id)).unwrap();
    assert!(storage.harvest_observations(orchard_id, cultivar).unwrap().is_empty());
    assert_visit_facts_round_trip(storage, orchard_id, species);
    assert_standalone_cues_can_be_saved_and_reviewed_atomically(storage, orchard_id, species);
    assert_legacy_visits_classify_once_with_the_original_snapshot(storage, orchard_id);
    assert_review_edits_can_validate_atomically(storage, orchard_id, species, TreeId(1));
    assert_review_edits_can_validate_atomically(storage, orchard_id, cultivar, TreeId(2));
}

pub fn assert_visit_facts_round_trip(storage: &mut impl OrchardStorage, orchard_id: OrchardId, species: HarvestScheduleOwner) {
    let template = storage.harvest_observations(orchard_id, species).unwrap()[0].clone();
    let expected = [
        (HarvestObservationKind::HarvestedAll, HarvestFruitState { ready: true, ..Default::default() }),
        (HarvestObservationKind::EmptyOnArrival, HarvestFruitState::default()),
        (HarvestObservationKind::AllWasted, HarvestFruitState { wasted: true, ..Default::default() }),
        (HarvestObservationKind::FruitRemaining, HarvestFruitState { ready: true, unripe: true, wasted: true }),
    ];
    for (kind, fruit_state) in expected {
        storage.transaction(|storage| {
            let mut visit = template.clone(); visit.kind = kind; visit.fruit_state = Some(fruit_state); visit.reviewed = false;
            storage.save_harvest_observation(visit)?;
            Ok::<_, OrchardStorageError>(())
        }).unwrap();
    }
    let date = HarvestDate::new(2026, 9, 28).unwrap();
    for outcome in [
        HarvestTreeOutcome::EmptyOnArrival { recorded_on: date },
        HarvestTreeOutcome::FruitRemaining { recorded_on: date, retry_on: date.add_days(7).unwrap() },
    ] {
        let mut run = storage.harvest_run(template.harvest_run_id.unwrap()).unwrap().unwrap();
        run.id = HarvestRunId(0); run.completed = false;
        for tree in &mut run.ordered_trees { tree.outcome = None; }
        let run_id = storage.transaction(|storage| storage.create_harvest_run(run.orchard_id, run.target, &run.harvested_parts, run.started_on, &run.ordered_trees)).unwrap();
        storage.transaction(|storage| storage.record_harvest_tree_outcome(run_id, template.tree_id, outcome)).unwrap();
        assert_eq!(storage.harvest_run(run_id).unwrap().unwrap().ordered_trees[0].outcome, Some(outcome));
        storage.transaction(|storage| storage.complete_harvest_run(run_id)).unwrap();
    }
    let points = storage.harvest_observations_in_orchard(orchard_id).unwrap();
    assert!(points.windows(2).all(|pair| pair[0].id < pair[1].id));
    for (kind, fruit_state) in expected {
        let point = points.iter().find(|point| point.kind == kind).unwrap();
        assert_eq!(point.fruit_state, Some(fruit_state));
        assert_eq!(point.owner, species);
        assert_eq!(point.period, template.period);
        assert_eq!(point.harvested_parts, template.harvested_parts);
    }
    assert!(storage.harvest_observations_in_orchard(OrchardId(99)).unwrap().is_empty());
}

pub fn assert_standalone_cues_can_be_saved_and_reviewed_atomically(
    storage: &mut impl OrchardStorage,
    orchard_id: OrchardId,
    owner: HarvestScheduleOwner,
) {
    use orchard_api::hexagon::use_cases::record_standalone_harvest_observation::*;
    storage
        .transaction(|storage| storage.replace_orchard_harvest_windows(orchard_id, owner, vec![]))
        .unwrap();
    let mut ids = Vec::new();
    for kind in [
        HarvestObservationKind::FruitRemaining,
        HarvestObservationKind::EmptyOnArrival,
    ] {
        ids.push(
            record_standalone_harvest_observation(
                StandaloneHarvestObservationRecorded {
                    orchard_id,
                    tree_id: TreeId(1),
                    action_date: "2026-07-12".into(),
                    harvested_part: HarvestedPart::Flower,
                    window: None,
                    kind,
                    fruit_state: (kind == HarvestObservationKind::FruitRemaining).then_some(
                        HarvestFruitState {
                            ready: true,
                            ..Default::default()
                        },
                    ),
                },
                storage,
            )
            .unwrap(),
        );
    }
    let before = storage.harvest_observations(orchard_id, owner).unwrap();
    for id in &ids {
        let point = before.iter().find(|point| point.id == *id).unwrap();
        assert_eq!(point.harvest_run_id, None);
        assert_eq!(point.period, None);
        assert_eq!(point.harvested_parts, vec![HarvestedPart::Flower]);
        assert_eq!(point.recorded_on.to_string(), "2026-07-12");
        assert!(!point.reviewed);
    }
    let change = |day| OrchardHarvestWindowsReplaced {
        orchard_id,
        owner,
        reference_region: "France".into(),
        windows: vec![AnnualHarvestWindowChanged {
            start_month: 7,
            start_day: day,
            end_month: 7,
            end_day: 31,
            harvested_part: HarvestedPart::Flower,
        }],
    };
    let review = |day| HarvestObservationsReviewed {
        orchard_id,
        owner,
        observation_ids: ids.clone(),
        window_change: Some(change(day)),
    };
    assert!(review_harvest_observations(review(32), storage).is_err());
    assert_eq!(
        storage.harvest_observations(orchard_id, owner).unwrap(),
        before
    );
    assert!(
        storage
            .orchard_harvest_windows(orchard_id, owner)
            .unwrap()
            .is_empty()
    );
    let rolled_back: Result<(), OrchardStorageError> = storage.transaction(|storage| {
        storage.replace_orchard_harvest_windows(
            orchard_id,
            owner,
            vec![AnnualHarvestWindow {
                start: AnnualDate::new(7, 1).unwrap(),
                end: AnnualDate::new(7, 31).unwrap(),
                harvested_part: HarvestedPart::Flower,
                reference_region: Some("France".into()),
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
                validated: false,
            }],
        )?;
        storage.review_harvest_observations(orchard_id, owner, &ids)?;
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    });
    assert!(rolled_back.is_err());
    assert_eq!(
        storage.harvest_observations(orchard_id, owner).unwrap(),
        before
    );
    assert!(
        storage
            .orchard_harvest_windows(orchard_id, owner)
            .unwrap()
            .is_empty()
    );
    review_harvest_observations(review(1), storage).unwrap();
    let windows = storage.orchard_harvest_windows(orchard_id, owner).unwrap();
    assert_eq!(windows[0].harvested_part, HarvestedPart::Flower);
    assert_eq!(windows[0].data_origin, HarvestDataOrigin::FieldObservation);
    let after = storage.harvest_observations(orchard_id, owner).unwrap();
    for (original, point) in before.iter().zip(&after) {
        let mut expected = original.clone();
        if ids.contains(&point.id) {
            expected.reviewed = true;
        }
        assert_eq!(*point, expected);
    }
}

pub fn assert_legacy_visits_classify_once_with_the_original_snapshot(
    storage: &mut impl OrchardStorage,
    orchard_id: OrchardId,
) {
    use orchard_api::hexagon::use_cases::classify_legacy_harvest_observation::*;
    let date = HarvestDate::new(2026, 1, 5).unwrap();
    let period = HarvestPeriod {
        start: HarvestDate::new(2025, 12, 10).unwrap(),
        end: HarvestDate::new(2026, 1, 20).unwrap(),
    };
    let parts = vec![HarvestedPart::Fruit, HarvestedPart::Flower];
    let run_id = storage
        .transaction(|storage| {
            storage.create_harvest_run(
                orchard_id,
                HarvestRunTarget::All,
                &parts,
                date,
                &[HarvestRunTree {
                    tree_id: TreeId(1),
                    harvested_parts: parts.clone(),
                    period,
                    outcome: None,
                }],
            )
        })
        .unwrap();
    storage
        .transaction(|storage| {
            storage.record_harvest_tree_outcome(
                run_id,
                TreeId(1),
                HarvestTreeOutcome::Deferred {
                    deferred_on: date,
                    retry_on: date.add_days(7).unwrap(),
                },
            )?;
            storage.complete_harvest_run(run_id)
        })
        .unwrap();
    let before = storage.harvest_run(run_id).unwrap().unwrap();
    let event = || LegacyHarvestObservationClassified {
        orchard_id,
        harvest_run_id: run_id,
        tree_id: TreeId(1),
        kind: HarvestObservationKind::FruitRemaining,
        fruit_state: Some(HarvestFruitState {
            ready: true,
            unripe: true,
            wasted: false,
        }),
    };
    let id = classify_legacy_harvest_observation(event(), storage).unwrap();
    assert_eq!(
        classify_legacy_harvest_observation(event(), storage),
        Err(LegacyHarvestObservationError::AlreadyClassified)
    );
    let points = storage.harvest_observations_in_orchard(orchard_id).unwrap();
    let point = points.iter().find(|point| point.id == id).unwrap();
    assert_eq!(point.harvest_run_id, Some(run_id));
    assert_eq!(point.period, Some(period));
    assert_eq!(point.recorded_on, date);
    assert_eq!(
        point.harvested_parts,
        before.ordered_trees[0].harvested_parts
    );
    assert!(
        parts
            .iter()
            .all(|part| point.harvested_parts.contains(part))
    );
    assert_eq!(point.fruit_state, event().fruit_state);
    assert!(!point.reviewed);
    assert_eq!(
        points
            .iter()
            .filter(|point| point.harvest_run_id == Some(run_id))
            .count(),
        1
    );
    assert_eq!(storage.harvest_run(run_id).unwrap().unwrap(), before);
    assert_eq!(
        storage
            .completed_harvest_runs(orchard_id)
            .unwrap()
            .iter()
            .find(|run| run.id == run_id)
            .unwrap()
            .ordered_trees,
        before.ordered_trees
    );
}


fn assert_review_edits_can_validate_atomically(
    storage: &mut impl OrchardStorage,
    orchard_id: OrchardId,
    owner: HarvestScheduleOwner,
    tree_id: TreeId,
) {
    let original = storage.orchard_harvest_windows(orchard_id, owner).unwrap();
    let window = &original[0];
    let period = HarvestPeriod {
        start: HarvestDate::new(2026, window.start.month, window.start.day).unwrap(),
        end: HarvestDate::new(
            if (window.end.month, window.end.day) < (window.start.month, window.start.day) {
                2027
            } else {
                2026
            },
            window.end.month,
            window.end.day,
        )
        .unwrap(),
    };
    let id = storage
        .transaction(|storage| {
            storage.save_harvest_observation(HarvestObservation {
                id: 0,
                orchard_id,
                owner,
                tree_id,
                harvest_run_id: None,
                recorded_on: period.start.add_days(1).unwrap(),
                kind: HarvestObservationKind::HarvestedAll,
                fruit_state: Some(HarvestFruitState {
                    ready: true,
                    ..Default::default()
                }),
                harvested_parts: vec![window.harvested_part],
                period: Some(period),
                reviewed: false,
            })
        })
        .unwrap();
    let before = storage.harvest_observations(orchard_id, owner).unwrap();
    let key = AnnualHarvestWindowChanged {
        start_month: window.start.month,
        start_day: window.start.day,
        end_month: window.end.month,
        end_day: window.end.day,
        harvested_part: window.harvested_part,
    };
    let mut replacement = key;
    replacement.end_day -= 1;
    review_harvest_observations_and_validate_window(
        HarvestObservationsReviewed {
            orchard_id,
            owner,
            observation_ids: vec![id],
            window_change: Some(OrchardHarvestWindowsReplaced {
                orchard_id,
                owner,
                reference_region: window.reference_region.clone().unwrap(),
                windows: vec![replacement],
            }),
        },
        key,
        storage,
    )
    .unwrap();
    let saved = storage.orchard_harvest_windows(orchard_id, owner).unwrap();
    assert_eq!(saved[0].end.day, replacement.end_day);
    assert!(saved[0].validated);
    assert_eq!(saved[0].data_origin, HarvestDataOrigin::FieldObservation);
    assert_eq!(saved[1..], original[1..]);
    let expected = before
        .into_iter()
        .map(|mut point| {
            if point.id == id {
                point.reviewed = true;
            }
            point
        })
        .collect::<Vec<_>>();
    assert_eq!(
        storage.harvest_observations(orchard_id, owner).unwrap(),
        expected
    );
}
