use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::cancel_harvest_run::{
    HarvestRunCancellationRequested, cancel_harvest_run,
};
use orchard_api::hexagon::use_cases::load_active_harvest_run::load_active_harvest_run;
use orchard_api::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, record_tree_harvested,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn delete_the_active_harvest_run_and_allow_a_fresh_one() {
    let mut storage = harvest_storage();
    let started = start(&mut storage);

    cancel_harvest_run(
        HarvestRunCancellationRequested {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        load_active_harvest_run(OrchardId(7), "2026-09-17", &mut storage),
        Ok(None)
    );
    assert_eq!(start(&mut storage).total_tree_count, 1);
}

#[test]
fn retain_recorded_harvests_when_abandoning_the_rest_of_a_run() {
    let mut storage = harvest_storage_with_tree_count(2);
    let started = start(&mut storage);
    record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: started.current_tree.unwrap().id,
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    cancel_harvest_run(
        HarvestRunCancellationRequested {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
        },
        &mut storage,
    )
    .unwrap();

    let restarted = start(&mut storage);
    assert_eq!(restarted.total_tree_count, 1);
    assert_eq!(restarted.current_tree.unwrap().id, TreeId(2));
}

fn start(
    storage: &mut InMemoryOrchardStorage,
) -> orchard_api::hexagon::use_cases::start_harvest_run::HarvestProgress {
    start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            action_date: "2026-09-17".into(),
        },
        storage,
    )
    .unwrap()
}

fn harvest_storage() -> InMemoryOrchardStorage {
    harvest_storage_with_tree_count(1)
}

fn harvest_storage_with_tree_count(tree_count: usize) -> InMemoryOrchardStorage {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: -73.5,
            latitude: 12.25,
            reference_region: "Example Region, France".into(),
        },
        vec![PlantIdentity {
            common_name: "Apple".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Malus".into(),
                species: Some("domestica".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
        }],
        (0..tree_count)
            .map(|index| Tree {
                legacy_source: None,
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: None,
                identification_status: IdentificationStatus::Confirmed,
                longitude: -73.5 + index as f64 / 10.0,
                latitude: 12.25,
                planted_on: None,
                row_name: None,
                roles: vec!["fruit".into()],
                is_alive: true,
                is_in_danger: false,
                reproductive_role: None,
                adult_height_meters: None,
                adult_width_meters: None,
            })
            .collect(),
    );
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 9,
                start_day: 1,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    storage
}
