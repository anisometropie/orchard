use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, InfraspecificRank, InfraspecificTaxon,
    LegacyPlantIdentification, LegacyTreeSource, NamedTaxon, PlantIdentity, PlantIdentityId,
    ReproductiveRole, Tree,
};
use orchard_api::hexagon::use_cases::import_legacy_orchard::{
    LegacyOrchardImportError, LegacyOrchardImportRequested, LegacyTreeSnapshot,
    import_legacy_orchard,
};

#[test]
fn import_john_rivers() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let john_rivers = john_rivers();
    let expected_identity = john_rivers.plant_identity.clone();
    let expected_tree = imported_tree(&john_rivers, 1);

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![john_rivers],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(observed_orchard.plant_identities(), vec![expected_identity]);
    assert_eq!(observed_orchard.trees(), vec![expected_tree]);
}

#[test]
fn import_tree_batch() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let pistachio = pistachio();
    let fig = fig(2);
    let expected_identities = vec![pistachio.plant_identity.clone(), fig.plant_identity.clone()];
    let expected_trees = vec![imported_tree(&pistachio, 1), imported_tree(&fig, 2)];

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio, fig],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(2));
    assert_eq!(observed_orchard.plant_identities(), expected_identities);
    assert_eq!(observed_orchard.trees(), expected_trees);
}

#[test]
fn reuse_identity() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let first_fig = fig(2);
    let second_fig = fig(249);
    let expected_identity = first_fig.plant_identity.clone();
    let expected_trees = vec![imported_tree(&first_fig, 1), imported_tree(&second_fig, 1)];

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![first_fig, second_fig],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(2));
    assert_eq!(observed_orchard.plant_identities(), vec![expected_identity]);
    assert_eq!(observed_orchard.trees(), expected_trees);
}

#[test]
fn reuse_identity_with_different_legacy_names() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let first_raspberry = surprise_dautomne(190);
    let second_raspberry = surprise_dautomne(209);
    let expected_identity = first_raspberry.plant_identity.clone();
    let expected_trees = vec![
        imported_tree(&first_raspberry, 1),
        imported_tree(&second_raspberry, 1),
    ];

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![first_raspberry, second_raspberry],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(2));
    assert_eq!(observed_orchard.plant_identities(), vec![expected_identity]);
    assert_eq!(observed_orchard.trees(), expected_trees);
}

#[test]
fn preserve_reproductive_role_and_historical_identification() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let boskoop = LegacyTreeSnapshot {
        legacy_source: LegacyTreeSource {
            feature_id: 64,
            name: "Kiwi ‘Boskoop’".into(),
            latin_name: "Actinidia deliciosa ‘Boskoop’".into(),
            legacy_identification: None,
            source_url: None,
        },
        plant_identity: named_identity("Kiwi", "Actinidia", "deliciosa", Some("Boskoop")),
        longitude: 0.81,
        latitude: 0.68,
        planted_on: Some("2024-12-07".into()),
        row_name: "4. Bas bas".into(),
        is_pioneer: false,
        is_alive: true,
        reproductive_role: Some(ReproductiveRole::SelfFertile),
        harvest_start_day: None,
        adult_height_meters: 6.0,
        adult_width_meters: 5.0,
        harvest_end_day: None,
    };
    let cranberry = LegacyTreeSnapshot {
        legacy_source: LegacyTreeSource {
            feature_id: 166,
            name: "Canneberge commune".into(),
            latin_name: "Vaccinium oxycoccos".into(),
            legacy_identification: Some(LegacyPlantIdentification {
                name: "Cranberry oxycoccos".into(),
                latin_name: "Vaccinium macrocarpon ‘Howes’".into(),
            }),
            source_url: None,
        },
        longitude: 0.36,
        latitude: 0.17,
        plant_identity: named_identity("Canneberge commune", "Vaccinium", "oxycoccos", None),
        planted_on: Some("2024-12-07".into()),
        row_name: "4. Bas bas".into(),
        is_pioneer: false,
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: 0.2,
        adult_width_meters: 0.5,
    };
    let expected_historical_identification = cranberry.legacy_source.legacy_identification.clone();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![boskoop, cranberry],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(2));
    let trees = observed_orchard.trees();
    assert_eq!(
        trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == 64)
            })
            .unwrap()
            .reproductive_role,
        Some(ReproductiveRole::SelfFertile)
    );
    assert_eq!(
        trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == 166)
            })
            .unwrap()
            .legacy_source
            .as_ref()
            .unwrap()
            .legacy_identification,
        expected_historical_identification
    );
}

#[test]
fn preserve_source_url() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let source_url = "https://www.promessedefleurs.com/fruitiers/petits-fruits/petits-fruits-de-a-a-z/lonicera-kamtschatica-eisbar-baie-de-mai.html";

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![LegacyTreeSnapshot {
                legacy_source: LegacyTreeSource {
                    feature_id: 157,
                    name: "Camérisier du Kamtchatka ‘Eisbär’".into(),
                    latin_name: "Lonicera caerulea var. kamtschatica ‘Eisbär’".into(),
                    legacy_identification: None,
                    source_url: Some(source_url.into()),
                },
                longitude: 0.57,
                latitude: 0.83,
                plant_identity: PlantIdentity {
                    common_name: "Camérisier du Kamtchatka".into(),
                    botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                        genus: "Lonicera".into(),
                        species: Some("caerulea".into()),
                        species_is_hybrid: false,
                        infraspecific: Some(InfraspecificTaxon {
                            rank: InfraspecificRank::Variety,
                            name: "kamtschatica".into(),
                        }),
                        is_aggregate: false,
                        cultivar_group: None,
                    }),
                    cultivar: Some("Eisbär".into()),
                    trade_name: None,
                    identification_status: IdentificationStatus::Confirmed,
                },
                planted_on: Some("2023-10-21".into()),
                row_name: "8. Bas".into(),
                is_pioneer: false,
                is_alive: true,
                reproductive_role: None,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: 1.5,
                adult_width_meters: 1.2,
            }],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(
        observed_orchard.trees()[0]
            .legacy_source
            .as_ref()
            .unwrap()
            .source_url,
        Some(source_url.into())
    );
}

#[test]
fn roll_back_batch_on_save_failure() {
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::failing_when_saving_tree_with_legacy_feature_id(3);

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio(), fig(2), caragana()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TreeCouldNotBeSaved {
            legacy_feature_id: 3,
        })
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn roll_back_when_identity_cannot_be_resolved() {
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::failing_when_resolving_plant_identity_with_genus("Caragana");

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio(), fig(2), caragana()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::PlantIdentityCouldNotBeResolved {
            legacy_feature_id: 3,
        })
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn preserve_existing_orchard_on_save_failure() {
    let existing = pistachio();
    let existing_identity = existing.plant_identity.clone();
    let existing_tree = imported_tree(&existing, 1);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard_failing_when_saving_tree_with_legacy_feature_id(
            vec![existing_identity.clone()],
            vec![existing_tree.clone()],
            3,
        );

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![john_rivers(), fig(2), caragana()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TreeCouldNotBeSaved {
            legacy_feature_id: 3,
        })
    );
    assert_eq!(observed_orchard.plant_identities(), vec![existing_identity]);
    assert_eq!(observed_orchard.trees(), vec![existing_tree]);
}

#[test]
fn reject_duplicate_legacy_feature() {
    let existing = pistachio();
    let existing_identity = existing.plant_identity.clone();
    let existing_tree = imported_tree(&existing, 1);
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::with_existing_orchard(
        vec![existing_identity.clone()],
        vec![existing_tree.clone()],
    );

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![fig(2), pistachio()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
            legacy_feature_id: 1,
        })
    );
    assert_eq!(observed_orchard.plant_identities(), vec![existing_identity]);
    assert_eq!(observed_orchard.trees(), vec![existing_tree]);
}

#[test]
fn reject_duplicate_legacy_feature_staged_by_the_same_import() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![fig(2), fig(2)],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::LegacyFeatureAlreadyImported {
            legacy_feature_id: 2,
        })
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn roll_back_batch_on_commit_failure() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::failing_on_commit();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio(), fig(2)],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TransactionCouldNotCommit)
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn preserve_pioneer() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let caragana = caragana();
    let expected_identity = caragana.plant_identity.clone();
    let expected_tree = imported_tree(&caragana, 1);

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![caragana],
        },
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(observed_orchard.plant_identities(), vec![expected_identity]);
    assert_eq!(observed_orchard.trees(), vec![expected_tree]);
}

#[test]
fn reject_when_transaction_cannot_start() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::failing_to_begin();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::TransactionCouldNotBegin)
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn reject_when_duplicate_check_fails() {
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::failing_when_checking_legacy_feature_ids();

    let import_result = import_legacy_orchard(
        LegacyOrchardImportRequested {
            trees: vec![pistachio()],
        },
        &mut orchard_storage,
    );

    assert_eq!(
        import_result,
        Err(LegacyOrchardImportError::ExistingLegacyFeaturesCouldNotBeChecked)
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

fn john_rivers() -> LegacyTreeSnapshot {
    LegacyTreeSnapshot {
        legacy_source: LegacyTreeSource {
            feature_id: 17,
            name: "Brugnon blanc ‘John Rivers’".into(),
            latin_name: "Prunus persica var. nucipersica ‘John Rivers’".into(),
            legacy_identification: None,
            source_url: None,
        },
        longitude: 0.72,
        latitude: 0.24,
        plant_identity: PlantIdentity {
            common_name: "Brugnon blanc".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Prunus".into(),
                species: Some("persica".into()),
                species_is_hybrid: false,
                infraspecific: Some(InfraspecificTaxon {
                    rank: InfraspecificRank::Variety,
                    name: "nucipersica".into(),
                }),
                is_aggregate: false,
                cultivar_group: None,
            }),
            cultivar: Some("John Rivers".into()),
            trade_name: None,
            identification_status: IdentificationStatus::Confirmed,
        },
        planted_on: Some("2024-12-07".into()),
        row_name: "10. Bas bas bas".into(),
        is_pioneer: false,
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: 4.0,
        adult_width_meters: 3.0,
    }
}

fn pistachio() -> LegacyTreeSnapshot {
    legacy_tree(LegacyTreeFixture {
        feature_id: 1,
        name: "Pistachier térébinthe",
        latin_name: "Pistacia terebinthus",
        plant_identity: named_identity("Pistachier térébinthe", "Pistacia", "terebinthus", None),
        longitude: 0.72,
        latitude: 0.24,
        planted_on: Some("2022-06-23"),
        row_name: "10. Bas bas bas",
        is_pioneer: false,
        is_alive: false,
        adult_height_meters: 5.0,
        adult_width_meters: 4.0,
    })
}

fn fig(feature_id: u32) -> LegacyTreeSnapshot {
    let (longitude, latitude, planted_on, row_name) = match feature_id {
        2 => (
            0.13,
            0.41,
            Some("2023-03-19"),
            "10. Bas bas bas",
        ),
        249 => (
            0.88,
            0.76,
            Some("2024-12-09"),
            "1. Haut haut haut",
        ),
        _ => panic!("only real Goutte d’Or feature ids belong in this fixture"),
    };
    legacy_tree(LegacyTreeFixture {
        feature_id,
        name: "Figuier ‘Goutte d’Or’",
        latin_name: "Ficus carica ‘Goutte d’Or’",
        plant_identity: named_identity("Figuier", "Ficus", "carica", Some("Goutte d’Or")),
        longitude,
        latitude,
        planted_on,
        row_name,
        is_pioneer: false,
        is_alive: true,
        adult_height_meters: 3.0,
        adult_width_meters: 4.0,
    })
}

fn caragana() -> LegacyTreeSnapshot {
    legacy_tree(LegacyTreeFixture {
        feature_id: 3,
        name: "Caraganier de Sibérie",
        latin_name: "Caragana arborescens",
        plant_identity: named_identity("Caraganier de Sibérie", "Caragana", "arborescens", None),
        longitude: 0.45,
        latitude: 0.11,
        planted_on: Some("2024-12-07"),
        row_name: "10. Bas bas bas",
        is_pioneer: true,
        is_alive: true,
        adult_height_meters: 4.5,
        adult_width_meters: 4.5,
    })
}

fn surprise_dautomne(feature_id: u32) -> LegacyTreeSnapshot {
    let (name, common_name, longitude, latitude, row_name, is_alive, adult_height_meters) =
        match feature_id {
            190 => (
                "Framboisier remontant ‘Surprise d’Automne’",
                "Framboisier remontant",
                0.93,
                0.57,
                "2. Haut haut",
                false,
                1.5,
            ),
            209 => (
                "Framboisier ‘Surprise d’Automne’",
                "Framboisier",
                0.29,
                0.92,
                "7. Bas haut",
                true,
                2.0,
            ),
            _ => panic!("only real Surprise d’Automne feature ids belong in this fixture"),
        };
    legacy_tree(LegacyTreeFixture {
        feature_id,
        name,
        latin_name: "Rubus idaeus ‘Surprise d’Automne’",
        plant_identity: named_identity(common_name, "Rubus", "idaeus", Some("Surprise d’Automne")),
        longitude,
        latitude,
        planted_on: Some("2024-12-09"),
        row_name,
        is_pioneer: false,
        is_alive,
        adult_height_meters,
        adult_width_meters: 1.0,
    })
}

struct LegacyTreeFixture<'a> {
    feature_id: u32,
    name: &'a str,
    latin_name: &'a str,
    plant_identity: PlantIdentity,
    longitude: f64,
    latitude: f64,
    planted_on: Option<&'a str>,
    row_name: &'a str,
    is_pioneer: bool,
    is_alive: bool,
    adult_height_meters: f64,
    adult_width_meters: f64,
}

fn legacy_tree(fixture: LegacyTreeFixture<'_>) -> LegacyTreeSnapshot {
    LegacyTreeSnapshot {
        legacy_source: LegacyTreeSource {
            feature_id: fixture.feature_id,
            name: fixture.name.into(),
            latin_name: fixture.latin_name.into(),
            legacy_identification: None,
            source_url: None,
        },
        longitude: fixture.longitude,
        latitude: fixture.latitude,
        plant_identity: fixture.plant_identity,
        planted_on: fixture.planted_on.map(str::to_owned),
        row_name: fixture.row_name.into(),
        is_pioneer: fixture.is_pioneer,
        is_alive: fixture.is_alive,
        reproductive_role: None,
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: fixture.adult_height_meters,
        adult_width_meters: fixture.adult_width_meters,
    }
}

fn named_identity(
    common_name: &str,
    genus: &str,
    species: &str,
    cultivar: Option<&str>,
) -> PlantIdentity {
    PlantIdentity {
        common_name: common_name.into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: genus.into(),
            species: Some(species.into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        cultivar: cultivar.map(str::to_owned),
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    }
}

fn imported_tree(legacy_tree: &LegacyTreeSnapshot, plant_identity_id: u64) -> Tree {
    Tree {
        legacy_source: Some(legacy_tree.legacy_source.clone()),
        plant_identity_id: PlantIdentityId(plant_identity_id),
        longitude: legacy_tree.longitude,
        latitude: legacy_tree.latitude,
        planted_on: legacy_tree.planted_on.clone(),
        row_name: Some(legacy_tree.row_name.clone()),
        roles: legacy_tree
            .is_pioneer
            .then(|| "pioneer".into())
            .into_iter()
            .collect(),
        is_alive: legacy_tree.is_alive,
        reproductive_role: legacy_tree.reproductive_role,
        harvest_start_day: legacy_tree.harvest_start_day,
        harvest_end_day: legacy_tree.harvest_end_day,
        adult_height_meters: Some(legacy_tree.adult_height_meters),
        adult_width_meters: Some(legacy_tree.adult_width_meters),
    }
}
