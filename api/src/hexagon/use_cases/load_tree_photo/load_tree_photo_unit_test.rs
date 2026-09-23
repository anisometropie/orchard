use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, TreePhoto, TreePhotoVariant,
};
use orchard_api::hexagon::ports::TreePhotoStorage;
use orchard_api::hexagon::use_cases::list_tree_photos::{TreePhotosRequested, list_tree_photos};
use orchard_api::hexagon::use_cases::load_tree_photo::{
    TreePhotoLoadError, TreePhotoRequested, load_tree_photo,
};

#[test]
fn load_a_specific_photo_only_through_its_tree_and_orchard() {
    let mut storage = orchard_storage();
    storage
        .save_tree_photo(
            OrchardId(7),
            TreeId(1),
            TreePhoto {
                full_webp: vec![1],
                thumbnail_webp: vec![2],
            },
        )
        .unwrap();
    let photo_id = list_tree_photos(
        TreePhotosRequested {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
        },
        &mut storage,
    )
    .unwrap()[0]
        .id;

    assert_eq!(
        load_tree_photo(
            TreePhotoRequested {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                photo_id,
                variant: TreePhotoVariant::Thumbnail
            },
            &mut storage
        ),
        Ok(vec![2])
    );
    assert_eq!(
        load_tree_photo(
            TreePhotoRequested {
                orchard_id: OrchardId(8),
                tree_id: TreeId(1),
                photo_id,
                variant: TreePhotoVariant::Full
            },
            &mut storage
        ),
        Err(TreePhotoLoadError::PhotoNotFound)
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
            reproductive_role: None,
            adult_height_meters: None,
            adult_width_meters: None,
        }],
    )
    .0
}
