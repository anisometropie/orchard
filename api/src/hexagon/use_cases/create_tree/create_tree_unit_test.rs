use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, PlantIdentity, PlantIdentityId, Tree,
};
use orchard_api::hexagon::use_cases::create_tree::{
    TreeCreationError, TreeCreationRequested, create_tree,
};

#[test]
fn save_new_tree() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();
    let apple = malus_domestica();

    let created_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identity: apple.clone(),
            roles: vec!["fruit".into()],
            harvest_start_day: Some(210),
            harvest_end_day: Some(260),
        },
        &mut orchard_unit_of_work,
    );

    let expected_tree = Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude: 0.72,
        latitude: 0.24,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: None,
        adult_width_meters: None,
    };
    assert_eq!(created_tree, Ok(expected_tree.clone()));
    assert_eq!(observed_orchard.plant_identities(), vec![apple]);
    assert_eq!(observed_orchard.trees(), vec![expected_tree]);
}

#[test]
fn reuse_identity() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();
    let apple = malus_domestica();

    let first_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identity: apple.clone(),
            roles: vec!["fruit".into()],
            harvest_start_day: Some(210),
            harvest_end_day: Some(260),
        },
        &mut orchard_unit_of_work,
    )
    .unwrap();
    let second_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.13,
            latitude: 0.41,
            plant_identity: apple.clone(),
            roles: vec!["fruit".into()],
            harvest_start_day: Some(210),
            harvest_end_day: Some(260),
        },
        &mut orchard_unit_of_work,
    )
    .unwrap();

    assert_eq!(first_tree.plant_identity_id, PlantIdentityId(1));
    assert_eq!(second_tree.plant_identity_id, PlantIdentityId(1));
    assert_eq!(observed_orchard.plant_identities(), vec![apple]);
    assert_eq!(observed_orchard.trees(), vec![first_tree, second_tree]);
}

#[test]
fn roll_back_on_save_failure() {
    let (mut orchard_unit_of_work, observed_orchard) =
        InMemoryOrchardStorage::failing_when_saving_any_tree();

    let creation_result = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            plant_identity: malus_domestica(),
            roles: vec!["fruit".into()],
            harvest_start_day: Some(210),
            harvest_end_day: Some(260),
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(creation_result, Err(TreeCreationError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

fn malus_domestica() -> PlantIdentity {
    PlantIdentity {
        common_name: "Pommier".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Malus".into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        cultivar: None,
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    }
}
