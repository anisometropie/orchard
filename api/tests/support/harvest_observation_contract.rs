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
        let mut run = storage.harvest_run(template.harvest_run_id).unwrap().unwrap();
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
