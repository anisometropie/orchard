#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::record_standalone_harvest_observation::*;

fn event() -> StandaloneHarvestObservationRecorded {
    StandaloneHarvestObservationRecorded {
        orchard_id: OrchardId(7),
        tree_id: TreeId(1),
        action_date: "2026-08-15".into(),
        harvested_part: HarvestedPart::Fruit,
        window: Some(ObservationHarvestWindow {
            start: AnnualDate::new(9, 1).unwrap(),
            end: AnnualDate::new(9, 30).unwrap(),
            start_year: 2026,
        }),
        kind: HarvestObservationKind::FruitRemaining,
        fruit_state: Some(HarvestFruitState {
            ready: true,
            unripe: true,
            wasted: false,
        }),
    }
}

#[test]
fn saves_before_or_after_the_period_without_changing_an_active_tour_or_the_schedule() {
    for date in ["2026-08-15", "2026-10-15"] {
        let mut storage = fixture::storage_with_fruit_window();
        let run = fixture::start(&mut storage, "2026-09-28").unwrap();
        let before = storage.harvest_run(run.run_id).unwrap().unwrap();
        let windows = storage
            .orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            )
            .unwrap();
        let mut request = event();
        request.action_date = date.into();
        let id = record_standalone_harvest_observation(request, &mut storage).unwrap();
        let points = storage
            .harvest_observations_in_orchard(OrchardId(7))
            .unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].id, id);
        assert_eq!(points[0].harvest_run_id, None);
        assert_eq!(points[0].recorded_on.to_string(), date);
        assert_eq!(points[0].period.unwrap().start.to_string(), "2026-09-01");
        assert!(!points[0].reviewed);
        assert_eq!(storage.harvest_run(run.run_id).unwrap().unwrap(), before);
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
fn records_a_tree_with_no_period_and_preserves_history_without_inventing_timing_evidence() {
    let mut storage = fixture::storage_with_fruit_window();
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
                vec![],
            )
        })
        .unwrap();
    let mut request = event();
    request.window = None;
    request.harvested_part = HarvestedPart::Flower;
    record_standalone_harvest_observation(request, &mut storage).unwrap();
    let points = storage
        .harvest_observations_in_orchard(OrchardId(7))
        .unwrap();
    assert_eq!(points[0].period, None);
    assert_eq!(points[0].harvested_parts, vec![HarvestedPart::Flower]);
    assert!(harvest_window_evidence(&points[0], &points).is_empty());
}

#[test]
fn invalid_dates_states_trees_or_periods_save_nothing() {
    for case in 0..5 {
        let mut storage = fixture::storage_with_fruit_window();
        let mut request = event();
        let expected = match case {
            0 => {
                request.action_date = "2026-02-30".into();
                StandaloneHarvestObservationError::InvalidActionDate
            }
            1 => {
                request.fruit_state = None;
                StandaloneHarvestObservationError::InvalidFruitState
            }
            2 => {
                request.tree_id = TreeId(999);
                StandaloneHarvestObservationError::TreeNotFound
            }
            3 => {
                request.window.as_mut().unwrap().end.day = 29;
                StandaloneHarvestObservationError::HarvestWindowChanged
            }
            _ => {
                request.harvested_part = HarvestedPart::Flower;
                StandaloneHarvestObservationError::HarvestWindowChanged
            }
        };
        assert_eq!(
            record_standalone_harvest_observation(request, &mut storage),
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
fn a_failed_commit_does_not_leave_an_observation_behind() {
    let mut storage = fixture::storage_with_fruit_window().with_commit_failure();
    assert_eq!(
        record_standalone_harvest_observation(event(), &mut storage),
        Err(StandaloneHarvestObservationError::ObservationCouldNotBeRecorded)
    );
    assert!(
        storage
            .harvest_observations_in_orchard(OrchardId(7))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn assigns_the_trees_cultivar_and_makes_the_record_available_for_its_period_review() {
    let mut storage = fixture::storage_with_cultivar_window();
    let mut request = event();
    request.tree_id = TreeId(2);
    let id = record_standalone_harvest_observation(request, &mut storage).unwrap();
    let owner = HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1));
    let points = storage.harvest_observations(OrchardId(7), owner).unwrap();
    assert_eq!(points[0].id, id);
    assert_eq!(points[0].owner, owner);
    use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::AnnualHarvestWindowChanged;
    use orchard_api::hexagon::use_cases::review_harvest_observations::*;
    review_harvest_observations_for_window(
        HarvestObservationsReviewed {
            orchard_id: OrchardId(7),
            owner,
            observation_ids: vec![id],
            window_change: None,
        },
        AnnualHarvestWindowChanged {
            start_month: 9,
            start_day: 1,
            end_month: 9,
            end_day: 30,
            harvested_part: HarvestedPart::Fruit,
        },
        &mut storage,
    )
    .unwrap();
    assert!(storage.harvest_observations(OrchardId(7), owner).unwrap()[0].reviewed);
}
