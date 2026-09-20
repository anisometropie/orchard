use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, GeoPoint, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::move_tree::{
    OrchardTreeMoveConfirmed, OrchardTreeMoveError, move_orchard_tree,
};

#[test]
fn confirming_a_tree_move_changes_only_that_trees_position() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(12.10, -34.10), tree(12.20, -34.20)],
    );

    let result = move_orchard_tree(
        OrchardTreeMoveConfirmed {
            orchard_id: OrchardId(7),
            tree_id: TreeId(2),
            position: GeoPoint {
                longitude: 12.25,
                latitude: -34.24,
            },
        },
        &mut storage,
    );

    assert_eq!(result, Ok(()));
    let trees = observer.trees();
    assert_eq!((trees[0].longitude, trees[0].latitude), (12.10, -34.10));
    assert_eq!((trees[1].longitude, trees[1].latitude), (12.25, -34.24));
}

#[test]
fn reject_a_position_outside_the_world_without_moving_the_tree() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(12.10, -34.10)],
    );

    let result = move_orchard_tree(
        OrchardTreeMoveConfirmed {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            position: GeoPoint {
                longitude: 181.0,
                latitude: -34.12,
            },
        },
        &mut storage,
    );

    assert_eq!(result, Err(OrchardTreeMoveError::InvalidPosition));
    let stored_tree = &observer.trees()[0];
    assert_eq!(
        (stored_tree.longitude, stored_tree.latitude),
        (12.10, -34.10)
    );
}

#[test]
fn reject_a_tree_from_another_orchard_without_moving_it() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(12.10, -34.10)],
    );

    let result = move_orchard_tree(
        OrchardTreeMoveConfirmed {
            orchard_id: OrchardId(8),
            tree_id: TreeId(1),
            position: GeoPoint {
                longitude: 12.25,
                latitude: -34.24,
            },
        },
        &mut storage,
    );

    assert_eq!(result, Err(OrchardTreeMoveError::TreeNotFound));
    let stored_tree = &observer.trees()[0];
    assert_eq!(
        (stored_tree.longitude, stored_tree.latitude),
        (12.10, -34.10)
    );
}

fn orchard() -> Orchard {
    Orchard {
        id: OrchardId(7),
        name: "Test orchard".into(),
        longitude: 12.0,
        latitude: -34.0,
        reference_region: "Synthetic test region".into(),
    }
}

fn tree(longitude: f64, latitude: f64) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude,
        planted_on: None,
        row_name: Some("Test row".into()),
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
