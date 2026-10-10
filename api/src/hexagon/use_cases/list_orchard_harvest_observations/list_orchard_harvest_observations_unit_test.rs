#[path = "../../../../tests/support/harvest_observation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use orchard_api::hexagon::models::*;
use orchard_api::hexagon::use_cases::list_orchard_harvest_observations::*;
use orchard_api::hexagon::use_cases::record_harvest_visit::*;

#[test]
fn production_history_includes_empty_visits_in_the_exact_orchard() {
    let mut storage = fixture::storage_with_cultivar_window();
    let run = fixture::start(&mut storage, "2026-09-28").unwrap();
    for (tree_id, kind) in [
        (TreeId(1), HarvestObservationKind::EmptyOnArrival),
        (TreeId(2), HarvestObservationKind::HarvestedAll),
    ] {
        record_harvest_visit(
            HarvestVisitRecorded {
                orchard_id: OrchardId(7),
                harvest_run_id: run.run_id,
                tree_id,
                action_date: "2026-09-28".into(),
                kind,
                fruit_state: None,
            },
            &mut storage,
        )
        .unwrap();
    }
    let records = list_orchard_harvest_observations(OrchardId(7), &mut storage).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].kind, HarvestObservationKind::EmptyOnArrival);
    assert_eq!(
        records[1].owner,
        HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1))
    );
    assert!(
        list_orchard_harvest_observations(OrchardId(8), &mut storage)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        list_orchard_harvest_observations(OrchardId(7), &mut storage).unwrap(),
        records
    );
}
