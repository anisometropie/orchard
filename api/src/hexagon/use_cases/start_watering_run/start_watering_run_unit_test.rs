use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree, TreeId, WateringRunId,
    WateringRunTarget,
};
use orchard_api::hexagon::ports::OrchardStorage;
use orchard_api::hexagon::use_cases::load_active_watering_run::load_active_watering_run;
use orchard_api::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{TreeWatered, record_tree_watered};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};
use orchard_api::hexagon::use_cases::start_watering_run::{
    WateringRunStartError, WateringRunStartRequested, start_watering_run,
};

#[test]
fn start_with_the_first_living_tree_in_the_rows_saved_order() {
    let trees = vec![
        apple_tree("North", -73.409, true),
        apple_tree("North", -73.227, true),
        apple_tree("North", -73.318, false),
    ];
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        trees,
    );
    order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(2), TreeId(3), TreeId(1)]),
        },
        &mut storage,
    )
    .unwrap();

    let progress = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.run_id, WateringRunId(1));
    assert_eq!(progress.target, WateringRunTarget::Row("North".into()));
    assert_eq!(progress.watered_tree_count, 0);
    assert_eq!(progress.total_tree_count, 2);
    let next_tree = progress.next_tree.unwrap();
    assert_eq!(next_tree.id, TreeId(2));
    assert_eq!(next_tree.longitude, -73.227);
    assert_eq!(next_tree.row_rank, 1);
    assert_eq!(
        observer.active_watering_run_tree_ids(OrchardId(7)),
        vec![TreeId(2), TreeId(1)]
    );
}

#[test]
fn refuse_to_start_an_unordered_row() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree("North", -73.409, true)],
    );

    let result = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    );

    assert_eq!(result, Err(WateringRunStartError::RowNotOrdered));
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn start_two_rows_independently_and_join_the_chosen_existing_row() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![
            apple_tree("North", -73.409, true),
            apple_tree("South", -73.227, true),
        ],
    );
    for (row, id) in [("North", TreeId(1)), ("South", TreeId(2))] {
        order_orchard_row(
            OrchardRowOrderRequested {
                orchard_id: OrchardId(7),
                row_name: row.into(),
                order: RowOrder::Manual(vec![id]),
            },
            &mut storage,
        )
        .unwrap();
    }
    let north = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    )
    .unwrap();
    let south = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "South".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_ne!(north.run_id, south.run_id);
    assert_eq!(
        start_watering_run(
            WateringRunStartRequested {
                orchard_id: OrchardId(7),
                row_name: "South".into()
            },
            &mut storage
        )
        .unwrap(),
        south
    );
    assert_eq!(
        storage
            .unfinished_watering_runs(OrchardId(7))
            .unwrap()
            .len(),
        2
    );
    record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: north.run_id,
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert!(observer.watering_run(north.run_id).unwrap().completed);
    assert!(
        observer
            .watering_run(south.run_id)
            .unwrap()
            .watered_tree_ids
            .is_empty()
    );
}

#[test]
fn danger_and_row_runs_can_coexist_and_a_second_danger_worker_joins_that_run() {
    use orchard_api::hexagon::models::GeoPoint;
    use orchard_api::hexagon::use_cases::start_danger_watering_run::{
        DangerWateringRunStartRequested, start_danger_watering_run,
    };
    let mut tree = apple_tree("North", -73.409, true);
    tree.is_in_danger = true;
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree],
    );
    order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(1)]),
        },
        &mut storage,
    )
    .unwrap();
    let row = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    )
    .unwrap();
    let start_danger = |storage: &mut InMemoryOrchardStorage| {
        start_danger_watering_run(
            DangerWateringRunStartRequested {
                orchard_id: OrchardId(7),
                water_source: GeoPoint {
                    longitude: -73.5,
                    latitude: 12.25,
                },
                carry_capacity: 2,
            },
            storage,
        )
        .unwrap()
    };
    let danger = start_danger(&mut storage);
    assert_ne!(row.run_id, danger.run_id);
    assert_eq!(start_danger(&mut storage), danger);
    assert_eq!(
        storage
            .unfinished_watering_runs(OrchardId(7))
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn restore_an_active_run_at_its_first_unwatered_tree() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![
            apple_tree("North", -73.409, true),
            apple_tree("North", -73.227, true),
        ],
    );
    order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(2), TreeId(1)]),
        },
        &mut storage,
    )
    .unwrap();
    let started = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    )
    .unwrap();
    record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
            tree_id: TreeId(2),
        },
        &mut storage,
    )
    .unwrap();

    let restored = load_active_watering_run(OrchardId(7), &mut storage)
        .unwrap()
        .unwrap();

    assert_eq!(restored.watered_tree_count, 1);
    assert_eq!(restored.next_tree.unwrap().id, TreeId(1));
}

#[test]
fn refuse_to_start_while_a_harvest_tour_is_active() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree("North", -73.409, true)],
    );
    order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(1)]),
        },
        &mut storage,
    )
    .unwrap();
    configure_fruit_harvest(&mut storage);
    start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-18".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        start_watering_run(
            WateringRunStartRequested {
                orchard_id: OrchardId(7),
                row_name: "North".into(),
            },
            &mut storage,
        ),
        Err(WateringRunStartError::HarvestRunIsActive)
    );
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

fn configure_fruit_harvest(storage: &mut InMemoryOrchardStorage) {
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 8,
                start_day: 1,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        storage,
    )
    .unwrap();
}

fn orchard() -> Orchard {
    Orchard {
        id: OrchardId(7),
        name: "My orchard".into(),
        longitude: -73.5,
        latitude: 12.25,
        reference_region: "Example Region, France".into(),
    }
}

fn apple_tree(row_name: &str, longitude: f64, is_alive: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 12.25,
        planted_on: None,
        row_name: Some(row_name.into()),
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}

fn apple_identity() -> PlantIdentity {
    PlantIdentity {
        common_name: "Apple".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Malus".into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
    }
}
