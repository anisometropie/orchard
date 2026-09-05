use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{
    TreeWatered, TreeWateredError, record_tree_watered,
};
use orchard_api::hexagon::use_cases::start_watering_run::{
    WateringRunStartRequested, start_watering_run,
};

#[test]
fn record_each_tree_and_advance_until_the_run_is_complete() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree(5.1), apple_tree(5.2)],
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

    let after_first = record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
            tree_id: TreeId(2),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(after_first.watered_tree_count, 1);
    assert_eq!(after_first.next_tree.unwrap().id, TreeId(1));

    let completed = record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(completed.watered_tree_count, 2);
    assert_eq!(completed.total_tree_count, 2);
    assert_eq!(completed.next_tree, None);
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn refuse_to_skip_the_next_tree_without_changing_progress() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree(5.1), apple_tree(5.2)],
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

    let result = record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
            tree_id: TreeId(1),
        },
        &mut storage,
    );

    assert_eq!(result, Err(TreeWateredError::TreeIsNotNext));
    let resumed = start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(resumed.next_tree.unwrap().id, TreeId(2));
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

fn apple_tree(longitude: f64) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 45.0,
        planted_on: None,
        row_name: Some("North".into()),
        roles: vec!["fruit".into()],
        is_alive: true,
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
