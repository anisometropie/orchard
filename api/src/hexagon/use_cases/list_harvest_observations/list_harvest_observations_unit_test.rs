#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::use_cases::list_harvest_observations::*;
use orchard_api::hexagon::use_cases::record_harvest_observation::*;

#[test]
fn reading_points_does_not_mark_them_reviewed_or_mix_scopes() {
    let mut storage = fixture::storage_with_fruit_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    record_harvest_observation(
        HarvestObservationRecorded {
            orchard_id: OrchardId(7),
            harvest_run_id: run.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-28".into(),
            kind: HarvestObservationKind::TooSoon,
        },
        &mut storage,
    )
    .unwrap();
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    let points = list_harvest_observations(OrchardId(7), owner, &mut storage).unwrap();
    assert_eq!(points.len(), 1);
    assert!(!points[0].reviewed);
    assert_eq!(
        list_harvest_observations(OrchardId(7), owner, &mut storage).unwrap(),
        points
    );
    assert!(
        list_harvest_observations(OrchardId(8), owner, &mut storage)
            .unwrap()
            .is_empty()
    );
    assert!(
        list_harvest_observations(
            OrchardId(7),
            HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1)),
            &mut storage
        )
        .unwrap()
        .is_empty()
    );
}
