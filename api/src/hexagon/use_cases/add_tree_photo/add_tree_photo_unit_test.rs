use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, TreePhotoVariant,
};
use orchard_api::hexagon::ports::{OrchardStorage, TreePhotoStorage};
use orchard_api::hexagon::use_cases::add_tree_photo::{
    MAX_FULL_PHOTO_BYTES, TreePhotoAddError, TreePhotoAdded, add_tree_photo,
};

#[test]
fn add_a_bounded_webp_photo_to_a_tree_in_the_orchard() {
    let mut storage = orchard_storage();
    let full = webp(&[1, 2, 3, 4]);
    let thumbnail = webp(&[5, 6]);

    let result = add_tree_photo(
        TreePhotoAdded {
            orchard_id: OrchardId(7),
            tree_id: TreeId(1),
            full_webp: full.clone(),
            thumbnail_webp: thumbnail.clone(),
        },
        &mut storage,
    );

    assert_eq!(result, Ok(()));
    assert!(
        storage.trees_in_orchard(OrchardId(7)).unwrap()[0].has_photo,
        "the orchard tree projection must expose that a photo exists"
    );
    assert_eq!(
        storage
            .latest_tree_photo(OrchardId(7), TreeId(1), TreePhotoVariant::Full)
            .unwrap(),
        Some(full)
    );
    assert_eq!(
        storage
            .latest_tree_photo(OrchardId(7), TreeId(1), TreePhotoVariant::Thumbnail,)
            .unwrap(),
        Some(thumbnail)
    );
}

#[test]
fn reject_invalid_oversized_and_cross_orchard_photos() {
    let mut storage = orchard_storage();
    let valid = webp(&[1]);

    assert_eq!(
        add_tree_photo(
            TreePhotoAdded {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                full_webp: b"not webp".to_vec(),
                thumbnail_webp: valid.clone(),
            },
            &mut storage,
        ),
        Err(TreePhotoAddError::InvalidWebp)
    );
    assert_eq!(
        add_tree_photo(
            TreePhotoAdded {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                full_webp: vec![0; MAX_FULL_PHOTO_BYTES + 1],
                thumbnail_webp: valid.clone(),
            },
            &mut storage,
        ),
        Err(TreePhotoAddError::PhotoTooLarge)
    );
    assert_eq!(
        add_tree_photo(
            TreePhotoAdded {
                orchard_id: OrchardId(8),
                tree_id: TreeId(1),
                full_webp: valid.clone(),
                thumbnail_webp: valid,
            },
            &mut storage,
        ),
        Err(TreePhotoAddError::TreeNotFound)
    );
}

fn webp(payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&u32::try_from(payload.len() + 4).unwrap().to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(payload);
    bytes
}

fn orchard_storage() -> InMemoryOrchardStorage {
    InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 5.0,
            latitude: 45.0,
            reference_region: "Drôme".into(),
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
            longitude: 5.0,
            latitude: 45.0,
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
