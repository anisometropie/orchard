use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::cancel_watering_run::{
    WateringRunCancellationError, WateringRunCancellationRequested, cancel_watering_run,
};
use orchard_api::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use orchard_api::hexagon::use_cases::record_tree_watered::{TreeWatered, record_tree_watered};
use orchard_api::hexagon::use_cases::start_watering_run::{
    WateringRunStartRequested, start_watering_run,
};

#[test]
fn delete_the_active_run_and_all_of_its_progress() {
    let (mut storage, observer) = watering_storage();
    let started = start_run(&mut storage);
    record_tree_watered(
        TreeWatered {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();

    cancel_watering_run(
        WateringRunCancellationRequested {
            orchard_id: OrchardId(7),
            watering_run_id: started.run_id,
        },
        &mut storage,
    )
    .unwrap();

    assert!(!observer.watering_run_exists(started.run_id));
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn refuse_to_delete_another_orchards_run() {
    let (mut storage, observer) = watering_storage();
    let started = start_run(&mut storage);

    let result = cancel_watering_run(
        WateringRunCancellationRequested {
            orchard_id: OrchardId(8),
            watering_run_id: started.run_id,
        },
        &mut storage,
    );

    assert_eq!(
        result,
        Err(WateringRunCancellationError::WateringRunNotFound)
    );
    assert!(observer.watering_run_exists(started.run_id));
}

fn watering_storage() -> (
    InMemoryOrchardStorage,
    orchard_api::adapters::secondary::InMemoryOrchardObserver,
) {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 5.0,
            latitude: 45.0,
            reference_region: "Drôme, France".into(),
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
        vec![apple_tree(5.1), apple_tree(5.2)],
    );
    order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(1), TreeId(2)]),
        },
        &mut storage,
    )
    .unwrap();
    (storage, observer)
}

fn start_run(
    storage: &mut InMemoryOrchardStorage,
) -> orchard_api::hexagon::use_cases::start_watering_run::WateringProgress {
    start_watering_run(
        WateringRunStartRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
        },
        storage,
    )
    .unwrap()
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
