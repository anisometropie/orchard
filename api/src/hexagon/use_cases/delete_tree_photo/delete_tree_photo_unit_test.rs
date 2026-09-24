use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, TreePhoto,
};
use orchard_api::hexagon::ports::{OrchardStorage, TreePhotoStorage};
use orchard_api::hexagon::use_cases::delete_tree_photo::{
    TreePhotoDeleteError, TreePhotoDeletionRequested, delete_tree_photo,
};
use orchard_api::hexagon::use_cases::list_tree_photos::{TreePhotosRequested, list_tree_photos};

#[test]
fn delete_only_the_requested_photo_and_clear_has_photo_after_the_last_one() {
    let mut storage = orchard_storage();
    storage
        .save_tree_photo(
            OrchardId(7),
            TreeId(1),
            TreePhoto {
                full_webp: vec![1],
                thumbnail_webp: vec![1],
            },
        )
        .unwrap();
    storage
        .save_tree_photo(
            OrchardId(7),
            TreeId(1),
            TreePhoto {
                full_webp: vec![2],
                thumbnail_webp: vec![2],
            },
        )
        .unwrap();
    let photos = list_tree_photos(
        TreePhotosRequested {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();

    delete_tree_photo(
        TreePhotoDeletionRequested {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            photo_id: photos[0].id,
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(
        list_tree_photos(
            TreePhotosRequested {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1)
            },
            &mut storage
        )
        .unwrap()
        .len(),
        1
    );
    delete_tree_photo(
        TreePhotoDeletionRequested {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            photo_id: photos[1].id,
        },
        &mut storage,
    )
    .unwrap();
    assert!(!storage.trees_in_orchard(OrchardId(7)).unwrap()[0].has_photo);
    assert_eq!(
        delete_tree_photo(
            TreePhotoDeletionRequested {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                photo_id: photos[1].id
            },
            &mut storage
        ),
        Err(TreePhotoDeleteError::PhotoNotFound)
    );
}

fn orchard_storage() -> InMemoryOrchardStorage {
    InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 1.0,
            latitude: 2.0,
            reference_region: "Region".into(),
        },
        vec![PlantIdentity {
            common_name: "Apple".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Malus".into(),
                species: None,
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
            longitude: 1.0,
            latitude: 2.0,
            planted_on: None,
            row_name: None,
            roles: vec![],
            is_alive: true,
            is_in_danger: false,
            is_excluded_from_watering: false,
            reproductive_role: None,
            adult_height_meters: None,
            adult_width_meters: None,
        }],
    )
    .0
}
