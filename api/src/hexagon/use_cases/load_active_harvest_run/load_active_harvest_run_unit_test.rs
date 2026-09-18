use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree,
};
use orchard_api::hexagon::use_cases::load_active_harvest_run::load_active_harvest_run;
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn restore_the_active_harvest_route() {
    let mut storage = harvest_storage();
    assert_eq!(
        load_active_harvest_run(OrchardId(7), "2026-09-17", &mut storage),
        Ok(None)
    );

    let started = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    let restored = load_active_harvest_run(OrchardId(7), "2026-09-17", &mut storage)
        .unwrap()
        .unwrap();

    assert_eq!(restored.run_id, started.run_id);
    assert_eq!(restored.route, started.route);
    assert_eq!(restored.current_tree, started.current_tree);
}

fn harvest_storage() -> InMemoryOrchardStorage {
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
        vec![Tree {
            legacy_source: None,
            plant_identity_id: PlantIdentityId(1),
            cultivar_id: None,
            identification_status: IdentificationStatus::Confirmed,
            longitude: -73.5,
            latitude: 12.25,
            planted_on: None,
            row_name: None,
            roles: vec!["fruit".into()],
            is_alive: true,
            is_in_danger: false,
            reproductive_role: None,
            adult_height_meters: None,
            adult_width_meters: None,
        }],
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
