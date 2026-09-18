use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartError, HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn start_with_every_living_tree_currently_in_a_fruit_period_and_order_nearest_neighbours() {
    let (mut storage, _) = harvest_storage(vec![
        tree(PlantIdentityId(1), 5.0, true),
        tree(PlantIdentityId(2), 5.3, true),
        tree(PlantIdentityId(3), 5.1, true),
        tree(PlantIdentityId(1), 5.05, false),
    ]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(2),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(3),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );

    let progress = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        progress
            .route
            .iter()
            .map(|tree| tree.id)
            .collect::<Vec<_>>(),
        vec![TreeId(1), TreeId(3), TreeId(2)]
    );
    assert_eq!(progress.current_tree.unwrap().id, TreeId(1));
    assert_eq!(progress.total_tree_count, 3);
    assert_eq!(progress.handled_tree_count, 0);
}

#[test]
fn select_one_species_and_only_fruit_windows() {
    let (mut storage, _) = harvest_storage(vec![
        tree(PlantIdentityId(1), 5.0, true),
        tree(PlantIdentityId(2), 5.1, true),
        tree(PlantIdentityId(3), 5.2, true),
    ]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(2),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(3),
        8,
        1,
        9,
        30,
        HarvestedPart::Nut,
    );

    let progress = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::Species(PlantIdentityId(2)),
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.route.len(), 1);
    assert_eq!(progress.route[0].plant_identity_id, PlantIdentityId(2));
}

#[test]
fn visit_a_tree_once_for_all_selected_parts_that_are_currently_available() {
    let (mut storage, _) = harvest_storage(vec![
        tree(PlantIdentityId(1), 5.0, true),
        tree(PlantIdentityId(2), 5.1, true),
    ]);
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![
                changed_window(9, 1, 9, 30, HarvestedPart::Cone),
                changed_window(9, 1, 9, 30, HarvestedPart::Flower),
                changed_window(9, 1, 9, 30, HarvestedPart::Fruit),
            ],
        },
        &mut storage,
    )
    .unwrap();
    configure_window(
        &mut storage,
        PlantIdentityId(2),
        9,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );

    let progress = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Flower, HarvestedPart::Cone],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.route.len(), 1);
    assert_eq!(progress.route[0].id, TreeId(1));
    assert_eq!(
        progress.route[0].harvested_parts,
        vec![HarvestedPart::Cone, HarvestedPart::Flower]
    );
    assert_eq!(
        progress.harvested_parts,
        vec![HarvestedPart::Cone, HarvestedPart::Flower]
    );
}

#[test]
fn merge_adjacent_current_fruit_windows_across_new_year() {
    let (mut storage, _) = harvest_storage(vec![tree(PlantIdentityId(1), 5.0, true)]);
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![
                changed_window(12, 20, 1, 10, HarvestedPart::Fruit),
                changed_window(1, 11, 1, 20, HarvestedPart::Fruit),
            ],
        },
        &mut storage,
    )
    .unwrap();

    let progress = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-01-05".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.route[0].period.start.to_string(), "2025-12-20");
    assert_eq!(progress.route[0].period.end.to_string(), "2026-01-20");
}

#[test]
fn reject_an_invalid_action_date_and_an_empty_current_harvest() {
    let (mut storage, _) = harvest_storage(vec![tree(PlantIdentityId(1), 5.0, true)]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        8,
        1,
        8,
        31,
        HarvestedPart::Fruit,
    );

    assert_eq!(
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id: OrchardId(7),
                target: HarvestRunTarget::All,
                harvested_parts: vec![],
                action_date: "2026-08-15".into(),
            },
            &mut storage,
        ),
        Err(HarvestRunStartError::NoHarvestPartsSelected)
    );
    assert_eq!(
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id: OrchardId(7),
                target: HarvestRunTarget::All,
                harvested_parts: vec![HarvestedPart::Fruit],
                action_date: "2026-02-29".into(),
            },
            &mut storage,
        ),
        Err(HarvestRunStartError::InvalidActionDate)
    );
    assert_eq!(
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id: OrchardId(7),
                target: HarvestRunTarget::All,
                harvested_parts: vec![HarvestedPart::Fruit],
                action_date: "2026-07-01".into(),
            },
            &mut storage,
        ),
        Err(HarvestRunStartError::NoTreesCurrentlyAvailable)
    );
}

#[test]
fn resume_the_same_target_but_refuse_a_different_target_while_active() {
    let (mut storage, _) = harvest_storage(vec![tree(PlantIdentityId(1), 5.0, true)]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        8,
        1,
        9,
        30,
        HarvestedPart::Fruit,
    );
    let first = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    let resumed = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-18".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(resumed.run_id, first.run_id);
    assert_eq!(
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id: OrchardId(7),
                target: HarvestRunTarget::All,
                harvested_parts: vec![HarvestedPart::Flower],
                action_date: "2026-09-18".into(),
            },
            &mut storage,
        ),
        Err(HarvestRunStartError::AnotherHarvestRunIsActive)
    );
    assert_eq!(
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id: OrchardId(7),
                target: HarvestRunTarget::Species(PlantIdentityId(1)),
                harvested_parts: vec![HarvestedPart::Fruit],
                action_date: "2026-09-18".into(),
            },
            &mut storage,
        ),
        Err(HarvestRunStartError::AnotherHarvestRunIsActive)
    );
}

fn harvest_storage(
    trees: Vec<Tree>,
) -> (
    InMemoryOrchardStorage,
    orchard_api::adapters::secondary::InMemoryOrchardObserver,
) {
    InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: -73.5,
            latitude: 12.25,
            reference_region: "Example Region, France".into(),
        },
        vec![identity("Apple"), identity("Pear"), identity("Plum")],
        trees,
    )
}

fn configure_window(
    storage: &mut InMemoryOrchardStorage,
    identity_id: PlantIdentityId,
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
    harvested_part: HarvestedPart,
) {
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(identity_id),
            reference_region: "Example Region, France".into(),
            windows: vec![changed_window(
                start_month,
                start_day,
                end_month,
                end_day,
                harvested_part,
            )],
        },
        storage,
    )
    .unwrap();
}

fn changed_window(
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
    harvested_part: HarvestedPart,
) -> AnnualHarvestWindowChanged {
    AnnualHarvestWindowChanged {
        start_month,
        start_day,
        end_month,
        end_day,
        harvested_part,
    }
}

fn tree(plant_identity_id: PlantIdentityId, longitude: f64, is_alive: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id,
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 12.25,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}

fn identity(common_name: &str) -> PlantIdentity {
    PlantIdentity {
        common_name: common_name.into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: common_name.into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
    }
}
