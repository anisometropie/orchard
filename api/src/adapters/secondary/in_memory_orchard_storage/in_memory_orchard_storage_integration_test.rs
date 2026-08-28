use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, LegacyTreeSource, NamedTaxon, PlantIdentity,
    PlantIdentityId, Tree,
};
use orchard_api::hexagon::ports::{OrchardTransaction, OrchardTransactionError, OrchardUnitOfWork};

#[test]
fn reject_missing_identity() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let mut transaction = orchard_storage.begin().unwrap();

    let save_result = transaction.save_tree(tree(PlantIdentityId(1), 64));

    assert_eq!(
        save_result,
        Err(OrchardTransactionError::TreeCouldNotBeSaved)
    );
    transaction.rollback();
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn reject_duplicate_feature() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let boskoop = plant_identity();

    let mut first_transaction = orchard_storage.begin().unwrap();
    let plant_identity_id = first_transaction
        .find_or_create_plant_identity(boskoop.clone())
        .unwrap();
    first_transaction
        .save_tree(tree(plant_identity_id, 64))
        .unwrap();
    first_transaction.commit().unwrap();

    let mut second_transaction = orchard_storage.begin().unwrap();
    let plant_identity_id = second_transaction
        .find_or_create_plant_identity(boskoop)
        .unwrap();
    let save_result = second_transaction.save_tree(tree(plant_identity_id, 64));

    assert_eq!(
        save_result,
        Err(OrchardTransactionError::TreeCouldNotBeSaved)
    );
    second_transaction.rollback();
    assert_eq!(observed_orchard.plant_identities().len(), 1);
    assert_eq!(observed_orchard.trees().len(), 1);
}

#[test]
fn reject_stale_transaction() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let boskoop = plant_identity();

    let mut first_transaction = orchard_storage.begin().unwrap();
    let mut second_transaction = orchard_storage.begin().unwrap();
    let first_identity_id = first_transaction
        .find_or_create_plant_identity(boskoop.clone())
        .unwrap();
    first_transaction
        .save_tree(tree(first_identity_id, 64))
        .unwrap();
    let second_identity_id = second_transaction
        .find_or_create_plant_identity(boskoop)
        .unwrap();
    second_transaction
        .save_tree(tree(second_identity_id, 166))
        .unwrap();

    assert_eq!(first_transaction.commit(), Ok(()));
    assert_eq!(
        second_transaction.commit(),
        Err(OrchardTransactionError::CouldNotCommit)
    );
    assert_eq!(observed_orchard.plant_identities().len(), 1);
    assert_eq!(observed_orchard.trees(), vec![tree(PlantIdentityId(1), 64)]);
}

fn plant_identity() -> PlantIdentity {
    PlantIdentity {
        common_name: "Kiwi".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Actinidia".into(),
            species: Some("deliciosa".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        cultivar: Some("Boskoop".into()),
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    }
}

fn tree(plant_identity_id: PlantIdentityId, legacy_feature_id: u32) -> Tree {
    Tree {
        legacy_source: Some(LegacyTreeSource {
            feature_id: legacy_feature_id,
            name: "Kiwi ‘Boskoop’".into(),
            latin_name: "Actinidia deliciosa ‘Boskoop’".into(),
            legacy_identification: None,
            source_url: None,
        }),
        plant_identity_id,
        longitude: 0.81,
        latitude: 0.68,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}
