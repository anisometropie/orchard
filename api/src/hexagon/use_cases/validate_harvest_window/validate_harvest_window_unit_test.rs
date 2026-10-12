#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::assess_harvest_window::*;
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::*;
use orchard_api::hexagon::use_cases::validate_harvest_window::*;

fn owner() -> HarvestScheduleOwner {
    HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1))
}
fn selection() -> HarvestWindowSelection {
    HarvestWindowSelection {
        start: AnnualDate { month: 9, day: 1 },
        end: AnnualDate { month: 9, day: 30 },
        harvested_part: HarvestedPart::Fruit,
    }
}
#[test]
fn owner_can_explicitly_validate_without_a_hard_evidence_gate_and_other_schedules_stay_untouched() {
    let mut storage = fixture::storage_with_cultivar_window();
    let cultivar = HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1));
    let before = storage
        .orchard_harvest_windows(OrchardId(7), cultivar)
        .unwrap();
    let assessment = assess_harvest_window(
        HarvestWindowAssessmentRequested {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    assert!(!assessment.validated);
    assert_eq!(assessment.evidence.observation_count, 0);
    validate_harvest_window(
        HarvestWindowValidated {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    let saved = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert!(saved[0].validated);
    assert_eq!(saved[0].data_origin, HarvestDataOrigin::FieldObservation);
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), cultivar)
            .unwrap(),
        before
    );
    let unchanged = |day| OrchardHarvestWindowsReplaced {
        orchard_id: OrchardId(7),
        owner: owner(),
        reference_region: "Example Region, France".into(),
        windows: vec![AnnualHarvestWindowChanged {
            start_month: 9,
            start_day: 1,
            end_month: 9,
            end_day: day,
            harvested_part: HarvestedPart::Fruit,
        }],
    };
    replace_orchard_harvest_windows(unchanged(30), &mut storage).unwrap();
    assert!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
    replace_orchard_harvest_windows(unchanged(29), &mut storage).unwrap();
    assert!(
        !storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
}
#[test]
fn observations_and_review_do_not_validate_a_window_and_validation_does_not_review_observations() {
    use orchard_api::hexagon::use_cases::record_standalone_harvest_observation::*;
    use orchard_api::hexagon::use_cases::review_harvest_observations::*;
    let mut storage = fixture::storage_with_fruit_window();
    let id = record_standalone_harvest_observation(
        StandaloneHarvestObservationRecorded {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            action_date: "2026-09-15".into(),
            harvested_part: HarvestedPart::Fruit,
            window: Some(ObservationHarvestWindow {
                start: selection().start,
                end: selection().end,
                start_year: 2026,
            }),
            kind: HarvestObservationKind::HarvestedAll,
            fruit_state: None,
        },
        &mut storage,
    )
    .unwrap();
    assert!(
        !storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
    review_harvest_observations(
        HarvestObservationsReviewed {
            orchard_id: OrchardId(7),
            owner: owner(),
            observation_ids: vec![id],
            window_change: None,
        },
        &mut storage,
    )
    .unwrap();
    assert!(
        !storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
    validate_harvest_window(
        HarvestWindowValidated {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    assert!(storage.harvest_observations(OrchardId(7), owner()).unwrap()[0].reviewed);
    let report = assess_harvest_window(
        HarvestWindowAssessmentRequested {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    assert!(report.validated);
    assert_eq!(report.evidence.observation_count, 1);
    assert!(!report.evidence.start_supported && !report.evidence.end_supported);
}
#[test]
fn stale_window_wrong_orchard_and_failed_commit_never_validate() {
    let mut storage = fixture::storage_with_fruit_window();
    let mut stale = selection();
    stale.end.day = 29;
    for (orchard_id, window) in [(OrchardId(8), selection()), (OrchardId(7), stale)] {
        assert_eq!(
            validate_harvest_window(
                HarvestWindowValidated {
                    orchard_id,
                    owner: owner(),
                    window
                },
                &mut storage
            ),
            Err(HarvestWindowValidationError::WindowNotFound)
        );
    }
    storage = storage.with_commit_failure();
    assert_eq!(
        validate_harvest_window(
            HarvestWindowValidated {
                orchard_id: OrchardId(7),
                owner: owner(),
                window: selection()
            },
            &mut storage
        ),
        Err(HarvestWindowValidationError::CouldNotValidate)
    );
    assert!(
        !storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .validated
    );
}
