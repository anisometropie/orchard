use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, WateringRunId,
};
use orchard_api::hexagon::use_cases::load_active_watering_run::load_active_watering_run;
use orchard_api::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{TreeWatered, record_tree_watered};
use orchard_api::hexagon::use_cases::start_watering_run::{
    WateringRunStartError, WateringRunStartRequested, start_watering_run,
};

#[test]
fn start_with_the_first_living_tree_in_the_rows_saved_order() {
    let trees = vec![
        apple_tree("North", 5.1, true),
        apple_tree("North", 5.3, true),
        apple_tree("North", 5.2, false),
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
    assert_eq!(progress.row_name, "North");
    assert_eq!(progress.watered_tree_count, 0);
    assert_eq!(progress.total_tree_count, 2);
    let next_tree = progress.next_tree.unwrap();
    assert_eq!(next_tree.id, TreeId(2));
    assert_eq!(next_tree.longitude, 5.3);
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
        vec![apple_tree("North", 5.1, true)],
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
fn restore_an_active_run_at_its_first_unwatered_tree() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![
            apple_tree("North", 5.1, true),
            apple_tree("North", 5.3, true),
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

fn orchard() -> Orchard {
    Orchard {
        id: OrchardId(7),
        name: "My orchard".into(),
        longitude: 5.0,
        latitude: 45.0,
        reference_region: "Drôme, France".into(),
    }
}

fn apple_tree(row_name: &str, longitude: f64, is_alive: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 45.0,
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
