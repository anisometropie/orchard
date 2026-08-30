use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, PlantIdentification, PlantIdentity,
    PlantIdentityId, Tree,
};
use orchard_api::hexagon::use_cases::create_tree::{
    TreeCreationError, TreeCreationRequested, create_tree,
};

#[test]
fn save_new_tree() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let apple = malus_domestica();

    let created_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identification: apple.clone(),
            roles: vec!["fruit".into()],
        },
        &mut orchard_storage,
    );

    let expected_tree = Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude: 0.72,
        latitude: 0.24,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    };
    assert_eq!(created_tree, Ok(expected_tree.clone()));
    assert_eq!(
        observed_orchard.plant_identities(),
        vec![apple.plant_identity]
    );
    assert_eq!(observed_orchard.trees(), vec![expected_tree]);
}

#[test]
fn reuse_identity() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let apple = malus_domestica();

    let first_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identification: apple.clone(),
            roles: vec!["fruit".into()],
        },
        &mut orchard_storage,
    )
    .unwrap();
    let second_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.13,
            latitude: 0.41,
            plant_identification: apple.clone(),
            roles: vec!["fruit".into()],
        },
        &mut orchard_storage,
    )
    .unwrap();

    assert_eq!(first_tree.plant_identity_id, PlantIdentityId(1));
    assert_eq!(second_tree.plant_identity_id, PlantIdentityId(1));
    assert_eq!(
        observed_orchard.plant_identities(),
        vec![apple.plant_identity]
    );
    assert_eq!(observed_orchard.trees(), vec![first_tree, second_tree]);
}

#[test]
fn roll_back_on_save_failure() {
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::failing_when_saving_any_tree();

    let creation_result = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identification: malus_domestica(),
            roles: vec!["fruit".into()],
        },
        &mut orchard_storage,
    );

    assert_eq!(creation_result, Err(TreeCreationError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

fn malus_domestica() -> PlantIdentification {
    PlantIdentification {
        plant_identity: PlantIdentity {
            common_name: "Pommier".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Malus".into(),
                species: Some("domestica".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
        },
        plant_cultivar: None,
        identification_status: IdentificationStatus::Confirmed,
    }
}
