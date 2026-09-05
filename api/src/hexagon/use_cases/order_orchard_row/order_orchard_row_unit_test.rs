use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderError, OrchardRowOrderRequested, RowOrder, order_orchard_row,
};

#[test]
fn order_every_tree_in_a_row_from_east_to_west_and_persist_its_rank() {
    let trees = vec![
        apple_tree("North", -73.409, 45.2),
        apple_tree("South", -73.591, 45.0),
        apple_tree("North", -73.227, 45.1),
        apple_tree("North", -73.318, 45.3),
    ];
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        trees,
    );

    let ordered = order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::EastToWest,
        },
        &mut storage,
    );

    assert_eq!(ordered, Ok(vec![TreeId(3), TreeId(4), TreeId(1)]));
    assert_eq!(
        observer.row_order(OrchardId(7), "North"),
        vec![TreeId(3), TreeId(4), TreeId(1)]
    );
    assert!(observer.row_order(OrchardId(7), "South").is_empty());
}

#[test]
fn support_both_directions_on_each_axis() {
    for (order, expected) in [
        (RowOrder::EastToWest, vec![TreeId(3), TreeId(2), TreeId(1)]),
        (RowOrder::WestToEast, vec![TreeId(1), TreeId(2), TreeId(3)]),
        (
            RowOrder::NorthToSouth,
            vec![TreeId(2), TreeId(3), TreeId(1)],
        ),
        (
            RowOrder::SouthToNorth,
            vec![TreeId(1), TreeId(3), TreeId(2)],
        ),
    ] {
        let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
            "owner",
            "password",
            orchard(),
            vec![apple_identity()],
            vec![
                apple_tree("North", -73.409, 45.1),
                apple_tree("North", -73.318, 45.3),
                apple_tree("North", -73.227, 45.2),
            ],
        );

        let ordered = order_orchard_row(
            OrchardRowOrderRequested {
                orchard_id: OrchardId(7),
                row_name: "North".into(),
                order,
            },
            &mut storage,
        )
        .unwrap();

        assert_eq!(ordered, expected);
    }
}

#[test]
fn reject_a_manual_order_that_omits_or_repeats_a_tree() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![
            apple_tree("North", -73.409, 45.1),
            apple_tree("North", -73.318, 45.2),
        ],
    );

    let result = order_orchard_row(
        OrchardRowOrderRequested {
            orchard_id: OrchardId(7),
            row_name: "North".into(),
            order: RowOrder::Manual(vec![TreeId(1), TreeId(1)]),
        },
        &mut storage,
    );

    assert_eq!(result, Err(OrchardRowOrderError::InvalidManualOrder));
    assert!(observer.row_order(OrchardId(7), "North").is_empty());
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

fn apple_tree(row_name: &str, longitude: f64, latitude: f64) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude,
        planted_on: None,
        row_name: Some(row_name.into()),
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
