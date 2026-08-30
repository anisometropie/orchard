use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, LegacyTreeSource, NamedTaxon, PlantCultivar,
    PlantCultivarId, PlantIdentification, PlantIdentity, PlantIdentityId, PlantIdentityReference,
    Tree,
};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[test]
fn reject_missing_identity() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let save_result = orchard_storage.transaction(|orchard| {
        orchard.save_tree(tree(
            PlantIdentityReference {
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: None,
            },
            64,
        ))
    });

    assert_eq!(save_result, Err(OrchardStorageError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn reject_duplicate_feature() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let boskoop = plant_identity();

    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.resolve_plant_identification(boskoop.clone())?;
            orchard.save_tree(tree(plant_identity_id, 64))
        })
        .unwrap();

    let save_result = orchard_storage.transaction(|orchard| {
        let plant_identity_id = orchard.resolve_plant_identification(boskoop)?;
        orchard.save_tree(tree(plant_identity_id, 64))
    });

    assert_eq!(save_result, Err(OrchardStorageError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities().len(), 1);
    assert_eq!(observed_orchard.trees().len(), 1);
}

#[test]
fn reject_nested_transaction() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let result = orchard_storage
        .transaction(|orchard| orchard.transaction::<_, OrchardStorageError>(|_| Ok(())));

    assert_eq!(
        result,
        Err(OrchardStorageError::AtomicOperationCouldNotBegin)
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn staged_tree_is_visible_inside_transaction_but_not_to_observers() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();

    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.resolve_plant_identification(plant_identity())?;
            orchard.save_tree(tree(plant_identity_id, 64))?;

            assert!(orchard.is_legacy_tree_already_imported(64)?);
            assert_eq!(observed_orchard.trees(), vec![]);
            Ok::<_, OrchardStorageError>(())
        })
        .unwrap();

    assert_eq!(
        observed_orchard.trees(),
        vec![tree(
            PlantIdentityReference {
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: Some(PlantCultivarId(1)),
            },
            64,
        )]
    );
    assert_eq!(
        orchard_storage.trees().unwrap()[0].plant_cultivar,
        Some(PlantCultivar {
            cultivar: "Boskoop".into(),
            trade_name: None,
        })
    );
}

fn plant_identity() -> PlantIdentification {
    PlantIdentification {
        plant_identity: PlantIdentity {
            common_name: "Kiwi".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Actinidia".into(),
                species: Some("deliciosa".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
        },
        plant_cultivar: Some(PlantCultivar {
            cultivar: "Boskoop".into(),
            trade_name: None,
        }),
        identification_status: IdentificationStatus::Confirmed,
    }
}

fn tree(plant_identity: PlantIdentityReference, legacy_feature_id: u32) -> Tree {
    Tree {
        legacy_source: Some(LegacyTreeSource {
            feature_id: legacy_feature_id,
            name: "Kiwi ‘Boskoop’".into(),
            latin_name: "Actinidia deliciosa ‘Boskoop’".into(),
            legacy_identification: None,
            source_url: None,
        }),
        plant_identity_id: plant_identity.plant_identity_id,
        cultivar_id: plant_identity.cultivar_id,
        identification_status: IdentificationStatus::Confirmed,
        longitude: 0.81,
        latitude: 0.68,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}
