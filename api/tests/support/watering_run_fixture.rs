use orchard_api::adapters::secondary::{InMemoryOrchardObserver, InMemoryOrchardStorage};
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, WateringRunTarget,
};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub fn storage() -> (InMemoryOrchardStorage, InMemoryOrchardObserver) {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "Orchard".into(),
            longitude: 5.1,
            latitude: 12.25,
            reference_region: "France".into(),
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
        vec![tree(5.1), tree(5.2)],
    );
    storage
        .transaction::<_, OrchardStorageError>(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(1), TreeId(2)],
            )
        })
        .unwrap();
    (storage, observer)
}

fn tree(longitude: f64) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 12.25,
        planted_on: None,
        row_name: Some("North".into()),
        roles: vec![],
        is_alive: true,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}
