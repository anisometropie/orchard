use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::Tree;
use orchard_api::hexagon::ports::TreeRepository;
use orchard_api::hexagon::use_cases::import_legacy_orchard::{
    LegacyOrchardImportError, LegacyOrchardImportRequested, LegacyTreeSnapshot,
    import_legacy_orchard,
};

#[test]
fn import_one_tree() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_feature_id: 1,
                longitude: 0.72,
                latitude: 0.24,
                name: "Pistachier térébinthe".into(),
                latin_name: "Pistacia terebinthus".into(),
                planted_on: Some("2022-06-23".into()),
                row_name: "10. Bas bas bas".into(),
                is_pioneer: false,
                is_alive: false,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 5.0,
                adult_width_meters: 4.0,
            }],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(
        observed_orchard.trees(),
        vec![Tree {
            legacy_feature_id: Some(1),
            longitude: 0.72,
            latitude: 0.24,
            name: "Pistachier térébinthe".into(),
            latin_name: Some("Pistacia terebinthus".into()),
            planted_on: Some("2022-06-23".into()),
            row_name: Some("10. Bas bas bas".into()),
            roles: vec![],
            is_alive: false,
            harvest_start_day: None,
            harvest_end_day: None,
            adult_height_meters: Some(5.0),
            adult_width_meters: Some(4.0),
        }]
    );
}

#[test]
fn import_tree_batch() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![
                LegacyTreeSnapshot {
                    legacy_feature_id: 1,
                    longitude: 0.72,
                    latitude: 0.24,
                    name: "Pistachier térébinthe".into(),
                    latin_name: "Pistacia terebinthus".into(),
                    planted_on: Some("2022-06-23".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: false,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 5.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 2,
                    longitude: 0.13,
                    latitude: 0.41,
                    name: "Figuier ‘Goutte d’Or’".into(),
                    latin_name: "Ficus carica ‘Goutte d’Or’".into(),
                    planted_on: Some("2023-03-19".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 3.0,
                    adult_width_meters: 4.0,
                },
            ],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(import_result, Ok(2));
    assert_eq!(
        observed_orchard.trees(),
        vec![
            Tree {
                legacy_feature_id: Some(1),
                longitude: 0.72,
                latitude: 0.24,
                name: "Pistachier térébinthe".into(),
                latin_name: Some("Pistacia terebinthus".into()),
                planted_on: Some("2022-06-23".into()),
                row_name: Some("10. Bas bas bas".into()),
                roles: vec![],
                is_alive: false,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: Some(5.0),
                adult_width_meters: Some(4.0),
            },
            Tree {
                legacy_feature_id: Some(2),
                longitude: 0.13,
                latitude: 0.41,
                name: "Figuier ‘Goutte d’Or’".into(),
                latin_name: Some("Ficus carica ‘Goutte d’Or’".into()),
                planted_on: Some("2023-03-19".into()),
                row_name: Some("10. Bas bas bas".into()),
                roles: vec![],
                is_alive: true,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: Some(3.0),
                adult_width_meters: Some(4.0),
            },
        ]
    );
}

#[test]
fn roll_back_batch_on_save_failure() {
    let (mut orchard_unit_of_work, observed_orchard) =
        InMemoryOrchardStorage::failing_when_saving_tree_with_legacy_feature_id(3);

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![
                LegacyTreeSnapshot {
                    legacy_feature_id: 1,
                    longitude: 0.72,
                    latitude: 0.24,
                    name: "Pistachier térébinthe".into(),
                    latin_name: "Pistacia terebinthus".into(),
                    planted_on: Some("2022-06-23".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: false,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 5.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 2,
                    longitude: 0.13,
                    latitude: 0.41,
                    name: "Figuier ‘Goutte d’Or’".into(),
                    latin_name: "Ficus carica ‘Goutte d’Or’".into(),
                    planted_on: Some("2023-03-19".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 3.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 3,
                    longitude: 0.45,
                    latitude: 0.11,
                    name: "Caraganier de Sibérie".into(),
                    latin_name: "Caragana arborescens".into(),
                    planted_on: Some("2024-12-07".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: true,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 4.5,
                    adult_width_meters: 4.5,
                },
            ],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TreeCouldNotBeSaved {
            legacy_feature_id: 3,
        })
    );
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn preserve_existing_trees_on_save_failure() {
    let existing_tree = Tree {
        legacy_feature_id: Some(400),
        longitude: 0.22,
        latitude: 0.34,
        name: "Abricotier".into(),
        latin_name: Some("Prunus armeniaca".into()),
        planted_on: Some("2021-04-23".into()),
        row_name: Some("1. Haut haut haut".into()),
        roles: vec!["fruit".into()],
        is_alive: true,
        harvest_start_day: Some(180),
        harvest_end_day: Some(210),
        adult_height_meters: Some(5.0),
        adult_width_meters: Some(4.0),
    };
    let (mut orchard_unit_of_work, observed_orchard) =
        InMemoryOrchardStorage::with_existing_trees_failing_when_saving_tree_with_legacy_feature_id(
            vec![existing_tree.clone()],
            3,
        );

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![
                LegacyTreeSnapshot {
                    legacy_feature_id: 1,
                    longitude: 0.72,
                    latitude: 0.24,
                    name: "Pistachier térébinthe".into(),
                    latin_name: "Pistacia terebinthus".into(),
                    planted_on: Some("2022-06-23".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: false,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 5.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 2,
                    longitude: 0.13,
                    latitude: 0.41,
                    name: "Figuier ‘Goutte d’Or’".into(),
                    latin_name: "Ficus carica ‘Goutte d’Or’".into(),
                    planted_on: Some("2023-03-19".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 3.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 3,
                    longitude: 0.45,
                    latitude: 0.11,
                    name: "Caraganier de Sibérie".into(),
                    latin_name: "Caragana arborescens".into(),
                    planted_on: Some("2024-12-07".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: true,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 4.5,
                    adult_width_meters: 4.5,
                },
            ],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TreeCouldNotBeSaved {
            legacy_feature_id: 3,
        })
    );
    assert_eq!(observed_orchard.trees(), vec![existing_tree]);
}

#[test]
fn reject_duplicate_legacy_feature() {
    let existing_tree = Tree {
        legacy_feature_id: Some(1),
        longitude: 0.72,
        latitude: 0.24,
        name: "Pistachier térébinthe".into(),
        latin_name: Some("Pistacia terebinthus".into()),
        planted_on: Some("2022-06-23".into()),
        row_name: Some("10. Bas bas bas".into()),
        roles: vec![],
        is_alive: false,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: Some(5.0),
        adult_width_meters: Some(4.0),
    };
    let (mut orchard_unit_of_work, observed_orchard) =
        InMemoryOrchardStorage::with_existing_trees(vec![existing_tree.clone()]);

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![
                LegacyTreeSnapshot {
                    legacy_feature_id: 2,
                    longitude: 0.13,
                    latitude: 0.41,
                    name: "Figuier ‘Goutte d’Or’".into(),
                    latin_name: "Ficus carica ‘Goutte d’Or’".into(),
                    planted_on: Some("2023-03-19".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 3.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 1,
                    longitude: 0.72,
                    latitude: 0.24,
                    name: "Pistachier térébinthe".into(),
                    latin_name: "Pistacia terebinthus".into(),
                    planted_on: Some("2022-06-23".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: false,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 5.0,
                    adult_width_meters: 4.0,
                },
            ],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
            legacy_feature_id: 1,
        })
    );
    assert_eq!(observed_orchard.trees(), vec![existing_tree]);
}

#[test]
fn repository_tree_is_visible_to_import() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let existing_tree = Tree {
        legacy_feature_id: Some(1),
        longitude: 0.72,
        latitude: 0.24,
        name: "Pistachier térébinthe".into(),
        latin_name: Some("Pistacia terebinthus".into()),
        planted_on: Some("2022-06-23".into()),
        row_name: Some("10. Bas bas bas".into()),
        roles: vec![],
        is_alive: false,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: Some(5.0),
        adult_width_meters: Some(4.0),
    };
    orchard_storage.save(existing_tree.clone()).unwrap();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_feature_id: 1,
                longitude: 0.72,
                latitude: 0.24,
                name: "Pistachier térébinthe".into(),
                latin_name: "Pistacia terebinthus".into(),
                planted_on: Some("2022-06-23".into()),
                row_name: "10. Bas bas bas".into(),
                is_pioneer: false,
                is_alive: false,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 5.0,
                adult_width_meters: 4.0,
            }],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
            legacy_feature_id: 1
        })
    );
    assert_eq!(observed_orchard.trees(), vec![existing_tree]);
}

#[test]
fn roll_back_batch_on_commit_failure() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::failing_on_commit();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![
                LegacyTreeSnapshot {
                    legacy_feature_id: 1,
                    longitude: 0.72,
                    latitude: 0.24,
                    name: "Pistachier térébinthe".into(),
                    latin_name: "Pistacia terebinthus".into(),
                    planted_on: Some("2022-06-23".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: false,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 5.0,
                    adult_width_meters: 4.0,
                },
                LegacyTreeSnapshot {
                    legacy_feature_id: 2,
                    longitude: 0.13,
                    latitude: 0.41,
                    name: "Figuier ‘Goutte d’Or’".into(),
                    latin_name: "Ficus carica ‘Goutte d’Or’".into(),
                    planted_on: Some("2023-03-19".into()),
                    row_name: "10. Bas bas bas".into(),
                    is_pioneer: false,
                    is_alive: true,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: 3.0,
                    adult_width_meters: 4.0,
                },
            ],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TransactionCouldNotCommit)
    );
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn preserve_pioneer_without_planting_date() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_feature_id: 3,
                longitude: 0.45,
                latitude: 0.11,
                name: "Caraganier de Sibérie".into(),
                latin_name: "Caragana arborescens".into(),
                planted_on: None,
                row_name: "10. Bas bas bas".into(),
                is_pioneer: true,
                is_alive: true,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 4.5,
                adult_width_meters: 4.5,
            }],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(
        observed_orchard.trees(),
        vec![Tree {
            legacy_feature_id: Some(3),
            longitude: 0.45,
            latitude: 0.11,
            name: "Caraganier de Sibérie".into(),
            latin_name: Some("Caragana arborescens".into()),
            planted_on: None,
            row_name: Some("10. Bas bas bas".into()),
            roles: vec!["pioneer".into()],
            is_alive: true,
            harvest_start_day: None,
            harvest_end_day: None,
            adult_height_meters: Some(4.5),
            adult_width_meters: Some(4.5),
        }]
    );
}

#[test]
fn reject_import_when_transaction_cannot_start() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::failing_to_begin();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_feature_id: 1,
                longitude: 0.72,
                latitude: 0.24,
                name: "Pistachier térébinthe".into(),
                latin_name: "Pistacia terebinthus".into(),
                planted_on: Some("2022-06-23".into()),
                row_name: "10. Bas bas bas".into(),
                is_pioneer: false,
                is_alive: false,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 5.0,
                adult_width_meters: 4.0,
            }],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TransactionCouldNotBegin)
    );
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn reject_import_when_legacy_feature_check_fails() {
    let (mut orchard_unit_of_work, observed_orchard) =
        InMemoryOrchardStorage::failing_when_checking_legacy_feature_ids();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_feature_id: 1,
                longitude: 0.72,
                latitude: 0.24,
                name: "Pistachier térébinthe".into(),
                latin_name: "Pistacia terebinthus".into(),
                planted_on: Some("2022-06-23".into()),
                row_name: "10. Bas bas bas".into(),
                is_pioneer: false,
                is_alive: false,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 5.0,
                adult_width_meters: 4.0,
            }],
        },
        &mut orchard_unit_of_work,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::ExistingLegacyFeaturesCouldNotBeChecked)
    );
    assert_eq!(observed_orchard.trees(), vec![]);
}
