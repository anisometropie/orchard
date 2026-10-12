#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::assess_harvest_window::*;
use orchard_api::hexagon::use_cases::record_standalone_harvest_observation::*;

#[test]
fn assess_both_boundaries_from_saved_observations_without_changing_the_window_or_review_state() {
    let mut storage = fixture::storage_with_fruit_window();
    let window = HarvestWindowSelection {
        start: AnnualDate { month: 9, day: 1 },
        end: AnnualDate { month: 9, day: 30 },
        harvested_part: HarvestedPart::Fruit,
    };
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    for (date, kind) in [
        ("2026-08-30", HarvestObservationKind::FruitRemaining),
        ("2026-09-02", HarvestObservationKind::HarvestedAll),
        ("2026-09-28", HarvestObservationKind::HarvestedAll),
        ("2026-10-01", HarvestObservationKind::AllWasted),
    ] {
        record_standalone_harvest_observation(
            StandaloneHarvestObservationRecorded {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                action_date: date.into(),
                harvested_part: HarvestedPart::Fruit,
                window: Some(ObservationHarvestWindow {
                    start: window.start,
                    end: window.end,
                    start_year: 2026,
                }),
                kind,
                fruit_state: if kind == HarvestObservationKind::FruitRemaining {
                    Some(HarvestFruitState {
                        unripe: true,
                        ..Default::default()
                    })
                } else {
                    None
                },
            },
            &mut storage,
        )
        .unwrap();
    }
    let before = storage
        .orchard_harvest_windows(OrchardId(7), owner)
        .unwrap();
    let points = storage.harvest_observations(OrchardId(7), owner).unwrap();
    let report = assess_harvest_window(
        HarvestWindowAssessmentRequested {
            orchard_id: OrchardId(7),
            owner,
            window,
        },
        &mut storage,
    )
    .unwrap();
    assert!(!report.validated);
    assert!(report.evidence.start_supported && report.evidence.end_supported);
    assert_eq!(report.evidence.observation_count, 4);
    assert_eq!(report.evidence.conflicting_observations, 0);
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner)
            .unwrap(),
        before
    );
    assert_eq!(
        storage.harvest_observations(OrchardId(7), owner).unwrap(),
        points
    );
    assert!(matches!(
        assess_harvest_window(
            HarvestWindowAssessmentRequested {
                orchard_id: OrchardId(8),
                owner,
                window
            },
            &mut storage
        ),
        Err(HarvestWindowAssessmentError::WindowNotFound)
    ));
}
