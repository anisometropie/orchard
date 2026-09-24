use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, TreePhoto,
};
use orchard_api::hexagon::ports::TreePhotoStorage;
use orchard_api::hexagon::use_cases::list_tree_photos::{
    TreePhotosListError, TreePhotosRequested, list_tree_photos,
};

#[test]
fn list_every_photo_newest_first_and_distinguish_a_missing_tree() {
    let mut storage = orchard_storage();
    storage
        .save_tree_photo(OrchardId(7), TreeId(1), photo(1))
        .unwrap();
    storage
        .save_tree_photo(OrchardId(7), TreeId(1), photo(2))
        .unwrap();

    let photos = list_tree_photos(
        TreePhotosRequested {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(photos.len(), 2);
    assert!(photos[0].id.0 > photos[1].id.0);
    assert!(photos.iter().all(|photo| photo.created_at_unix_seconds > 0));
    assert_eq!(
        list_tree_photos(
            TreePhotosRequested {
                orchard_id: OrchardId(8),
                tree_id: TreeId(1)
            },
            &mut storage,
        ),
        Err(TreePhotosListError::TreeNotFound),
    );
}

fn photo(byte: u8) -> TreePhoto {
    TreePhoto {
        full_webp: vec![byte],
        thumbnail_webp: vec![byte],
    }
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
