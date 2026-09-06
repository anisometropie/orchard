use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, TreePhoto, TreePhotoVariant,
};
use orchard_api::hexagon::ports::TreePhotoStorage;
use orchard_api::hexagon::use_cases::load_latest_tree_photo::{
    LatestTreePhotoLoadError, LatestTreePhotoRequested, load_latest_tree_photo,
};

#[test]
fn load_the_requested_variant_from_the_most_recent_photo() {
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
    storage
        .save_tree_photo(
            OrchardId(7),
            TreeId(1),
            TreePhoto {
                full_webp: vec![3],
                thumbnail_webp: vec![4],
            },
        )
        .unwrap();

    assert_eq!(
        load_latest_tree_photo(
            LatestTreePhotoRequested {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                variant: TreePhotoVariant::Full,
            },
            &mut storage,
        ),
        Ok(vec![3])
    );
    assert_eq!(
        load_latest_tree_photo(
            LatestTreePhotoRequested {
                orchard_id: OrchardId(7),
                tree_id: TreeId(1),
                variant: TreePhotoVariant::Thumbnail,
            },
            &mut storage,
        ),
        Ok(vec![4])
    );
}

#[test]
fn do_not_load_a_photo_through_another_orchard() {
    let mut storage = orchard_storage();

    assert_eq!(
        load_latest_tree_photo(
            LatestTreePhotoRequested {
                orchard_id: OrchardId(8),
                tree_id: TreeId(1),
                variant: TreePhotoVariant::Full,
            },
            &mut storage,
        ),
        Err(LatestTreePhotoLoadError::PhotoNotFound)
    );
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
