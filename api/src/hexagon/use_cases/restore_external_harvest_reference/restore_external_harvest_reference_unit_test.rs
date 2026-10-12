#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::restore_external_harvest_reference::*;
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
fn event(orchard_id: OrchardId) -> ExternalHarvestReferenceRestored {
    ExternalHarvestReferenceRestored {
        orchard_id,
        owner: owner(),
        window: selection(),
    }
}

#[test]
fn return_only_the_selected_window_to_external_reference_and_preserve_its_link_and_observations() {
    use orchard_api::hexagon::use_cases::record_standalone_harvest_observation::*;
    let mut storage = fixture::storage_with_cultivar_window();
    let cultivar = HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1));
    let other = storage
        .orchard_harvest_windows(OrchardId(7), cultivar)
        .unwrap();
    let mut windows = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    windows[0].data_origin = HarvestDataOrigin::ExternalReference;
    windows[0].source_url = Some("https://example.com/apple".into());
    windows.push(AnnualHarvestWindow {
        start: AnnualDate { month: 6, day: 1 },
        end: AnnualDate { month: 6, day: 30 },
        ..windows[0].clone()
    });
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(OrchardId(7), owner(), windows.clone())
        })
        .unwrap();
    record_standalone_harvest_observation(
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
    let points = storage.harvest_observations(OrchardId(7), owner()).unwrap();
    validate_harvest_window(
        HarvestWindowValidated {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap()[0]
            .source_url,
        windows[0].source_url
    );
    restore_external_harvest_reference(event(OrchardId(7)), &mut storage).unwrap();
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        windows
    );
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), cultivar)
            .unwrap(),
        other
    );
    assert_eq!(
        storage.harvest_observations(OrchardId(7), owner()).unwrap(),
        points
    );
}

#[test]
fn return_an_unvalidated_field_window_without_inventing_a_reference_link() {
    let mut storage = fixture::storage_with_fruit_window();
    restore_external_harvest_reference(event(OrchardId(7)), &mut storage).unwrap();
    let saved = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert_eq!(saved[0].data_origin, HarvestDataOrigin::ExternalReference);
    assert!(!saved[0].validated);
    assert!(saved[0].source_url.is_none());
    restore_external_harvest_reference(event(OrchardId(7)), &mut storage).unwrap();
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        saved
    );
}

#[test]
fn stale_window_wrong_orchard_and_failed_commit_leave_the_window_untouched() {
    let mut storage = fixture::storage_with_fruit_window();
    validate_harvest_window(
        HarvestWindowValidated {
            orchard_id: OrchardId(7),
            owner: owner(),
            window: selection(),
        },
        &mut storage,
    )
    .unwrap();
    let before = storage
        .orchard_harvest_windows(OrchardId(7), owner())
        .unwrap();
    assert_eq!(
        restore_external_harvest_reference(event(OrchardId(8)), &mut storage),
        Err(ExternalHarvestReferenceRestorationError::WindowNotFound)
    );
    let mut stale = event(OrchardId(7));
    stale.window.end.day = 29;
    assert_eq!(
        restore_external_harvest_reference(stale, &mut storage),
        Err(ExternalHarvestReferenceRestorationError::WindowNotFound)
    );
    storage = storage.with_commit_failure();
    assert_eq!(
        restore_external_harvest_reference(event(OrchardId(7)), &mut storage),
        Err(ExternalHarvestReferenceRestorationError::CouldNotRestore)
    );
    assert_eq!(
        storage
            .orchard_harvest_windows(OrchardId(7), owner())
            .unwrap(),
        before
    );
}
