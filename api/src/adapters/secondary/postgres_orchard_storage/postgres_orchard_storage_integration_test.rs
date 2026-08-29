use orchard_api::adapters::secondary::PostgresOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, InfraspecificRank, InfraspecificTaxon,
    LegacyPlantIdentification, LegacyTreeSource, NamedTaxon, OrchardTree, PlantIdentity,
    PlantIdentityId, ReproductiveRole, Tree, TreeId,
};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};
use orchard_api::hexagon::use_cases::change_tree_condition::{
    TreeConditionChanged, change_tree_condition,
};
use postgres::{Client, NoTls};

#[test]
fn commit_persists_identity_and_tree() {
    let _database_lock = database_lock();
    let (database_url, mut verification_connection) = empty_orchard_database();

    let john_rivers = PlantIdentity {
        common_name: "Brugnon blanc".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Prunus".into(),
            species: Some("persica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        cultivar: Some("John Rivers".into()),
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    };
    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    let (plant_identity_id, expected_tree) = orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.find_or_create_plant_identity(john_rivers.clone())?;
            let expected_tree = Tree {
                legacy_source: Some(LegacyTreeSource {
                    feature_id: 17,
                    name: "Brugnon blanc ‘John Rivers’".into(),
                    latin_name: "Prunus persica var. nucipersica ‘John Rivers’".into(),
                    legacy_identification: None,
                    source_url: None,
                }),
                plant_identity_id,
                longitude: 0.72,
                latitude: 0.24,
                planted_on: Some("2024-12-07".into()),
                row_name: Some("10. Bas bas bas".into()),
                roles: vec![],
                is_alive: true,
                is_in_danger: true,
                reproductive_role: None,
                harvest_start_day: None,
                harvest_end_day: None,
                adult_height_meters: Some(4.0),
                adult_width_meters: Some(3.0),
            };
            orchard.save_tree(expected_tree.clone())?;
            Ok::<_, OrchardStorageError>((plant_identity_id, expected_tree))
        })
        .unwrap();

    assert_eq!(plant_identity_id, PlantIdentityId(1));
    let persisted_identity = verification_connection
        .query_one(
            "SELECT id, common_name, cultivar, trade_name, identification_status FROM plant_identities",
            &[],
        )
        .unwrap();
    assert_eq!(persisted_identity.get::<_, i64>(0), 1);
    assert_eq!(
        persisted_identity.get::<_, String>(1),
        john_rivers.common_name
    );
    assert_eq!(
        persisted_identity.get::<_, Option<String>>(2),
        john_rivers.cultivar
    );
    assert_eq!(persisted_identity.get::<_, Option<String>>(3), None);
    assert_eq!(persisted_identity.get::<_, String>(4), "confirmed");

    let persisted_tree = verification_connection
        .query_one(
            "SELECT legacy_feature_id, plant_identity_id, ST_X(location), ST_Y(location), legacy_name, legacy_latin_name, planted_on::text, row_name, roles, is_alive, is_in_danger, harvest_start_day, harvest_end_day, adult_height_meters, adult_width_meters FROM trees WHERE legacy_feature_id = 17",
            &[],
        )
        .unwrap();
    assert_eq!(persisted_tree.get::<_, i32>(0), 17);
    assert_eq!(persisted_tree.get::<_, i64>(1), 1);
    assert_eq!(persisted_tree.get::<_, f64>(2), expected_tree.longitude);
    assert_eq!(persisted_tree.get::<_, f64>(3), expected_tree.latitude);
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(4),
        Some("Brugnon blanc ‘John Rivers’".into())
    );
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(5),
        Some("Prunus persica var. nucipersica ‘John Rivers’".into())
    );
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(6),
        expected_tree.planted_on
    );
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(7),
        expected_tree.row_name
    );
    assert_eq!(persisted_tree.get::<_, Vec<String>>(8), expected_tree.roles);
    assert_eq!(persisted_tree.get::<_, bool>(9), expected_tree.is_alive);
    assert_eq!(
        persisted_tree.get::<_, bool>(10),
        expected_tree.is_in_danger
    );
    assert_eq!(
        persisted_tree.get::<_, Option<i16>>(11),
        expected_tree.harvest_start_day.map(|day| day as i16)
    );
    assert_eq!(
        persisted_tree.get::<_, Option<i16>>(12),
        expected_tree.harvest_end_day.map(|day| day as i16)
    );
    assert_eq!(
        persisted_tree.get::<_, Option<f64>>(13),
        expected_tree.adult_height_meters
    );
    assert_eq!(
        persisted_tree.get::<_, Option<f64>>(14),
        expected_tree.adult_width_meters
    );
}

#[test]
fn read_tree_with_its_identity() {
    let _database_lock = database_lock();
    let (database_url, _) = empty_orchard_database();
    let apple = PlantIdentity {
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
    };
    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    let tree = orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.find_or_create_plant_identity(apple.clone())?;
            let tree = tree(plant_identity_id, 17);
            orchard.save_tree(tree.clone())?;
            Ok::<_, OrchardStorageError>(tree)
        })
        .unwrap();

    assert_eq!(
        orchard_storage.trees(),
        Ok(vec![OrchardTree {
            id: TreeId(1),
            tree,
            plant_identity: apple,
        }])
    );
}

#[test]
fn change_tree_danger_by_numeric_id() {
    let _database_lock = database_lock();
    let (database_url, mut verification_connection) = empty_orchard_database();
    let apple = PlantIdentity {
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
    };
    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.find_or_create_plant_identity(apple)?;
            orchard.save_tree(tree(plant_identity_id, 17))
        })
        .unwrap();

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: None,
            is_in_danger: Some(true),
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Ok(()));
    let is_in_danger: bool = verification_connection
        .query_one("SELECT is_in_danger FROM trees WHERE id = 1", &[])
        .unwrap()
        .get(0);
    assert!(is_in_danger);
}

#[test]
fn change_tree_life_status_by_numeric_id_and_clear_danger() {
    let _database_lock = database_lock();
    let (database_url, mut verification_connection) = empty_orchard_database();
    let apple = PlantIdentity {
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
    };
    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.find_or_create_plant_identity(apple)?;
            let mut tree = tree(plant_identity_id, 17);
            tree.is_in_danger = true;
            orchard.save_tree(tree)
        })
        .unwrap();

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: Some(false),
            is_in_danger: None,
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Ok(()));
    let row = verification_connection
        .query_one("SELECT is_alive, is_in_danger FROM trees WHERE id = 1", &[])
        .unwrap();
    assert!(!row.get::<_, bool>(0));
    assert!(!row.get::<_, bool>(1));
}

#[test]
fn persist_legacy_details() {
    let _database_lock = database_lock();
    let (database_url, mut verification_connection) = empty_orchard_database();

    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    let source_url = "https://www.promessedefleurs.com/fruitiers/petits-fruits/petits-fruits-de-a-a-z/lonicera-kamtschatica-eisbar-baie-de-mai.html";
    orchard_storage
        .transaction(|orchard| {
            let boskoop_identity_id = orchard
                .find_or_create_plant_identity(PlantIdentity {
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
                })
                .unwrap();
            orchard
                .save_tree(Tree {
                    legacy_source: Some(LegacyTreeSource {
                        feature_id: 64,
                        name: "Kiwi ‘Boskoop’".into(),
                        latin_name: "Actinidia deliciosa ‘Boskoop’".into(),
                        legacy_identification: None,
                        source_url: None,
                    }),
                    plant_identity_id: boskoop_identity_id,
                    longitude: 0.81,
                    latitude: 0.68,
                    planted_on: Some("2024-12-07".into()),
                    row_name: Some("4. Bas bas".into()),
                    roles: vec!["fruit".into()],
                    is_alive: true,
                    is_in_danger: false,
                    reproductive_role: Some(ReproductiveRole::SelfFertile),
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: Some(6.0),
                    adult_width_meters: Some(5.0),
                })
                .unwrap();
            let cranberry_identity_id = orchard
                .find_or_create_plant_identity(PlantIdentity {
                    common_name: "Canneberge commune".into(),
                    botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                        genus: "Vaccinium".into(),
                        species: Some("oxycoccos".into()),
                        species_is_hybrid: false,
                        infraspecific: None,
                        is_aggregate: false,
                        cultivar_group: None,
                    }),
                    cultivar: None,
                    trade_name: None,
                    identification_status: IdentificationStatus::Confirmed,
                })
                .unwrap();
            orchard
                .save_tree(Tree {
                    legacy_source: Some(LegacyTreeSource {
                        feature_id: 166,
                        name: "Canneberge commune".into(),
                        latin_name: "Vaccinium oxycoccos".into(),
                        legacy_identification: Some(LegacyPlantIdentification {
                            name: "Cranberry oxycoccos".into(),
                            latin_name: "Vaccinium macrocarpon ‘Howes’".into(),
                        }),
                        source_url: None,
                    }),
                    plant_identity_id: cranberry_identity_id,
                    longitude: 0.36,
                    latitude: 0.17,
                    planted_on: Some("2024-12-07".into()),
                    row_name: Some("4. Bas bas".into()),
                    roles: vec!["fruit".into()],
                    is_alive: true,
                    is_in_danger: false,
                    reproductive_role: None,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: Some(0.2),
                    adult_width_meters: Some(0.5),
                })
                .unwrap();
            let eisbar_identity_id = orchard
                .find_or_create_plant_identity(PlantIdentity {
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
                })
                .unwrap();
            orchard
                .save_tree(Tree {
                    legacy_source: Some(LegacyTreeSource {
                        feature_id: 157,
                        name: "Camérisier du Kamtchatka ‘Eisbär’".into(),
                        latin_name: "Lonicera caerulea var. kamtschatica ‘Eisbär’".into(),
                        legacy_identification: None,
                        source_url: Some(source_url.into()),
                    }),
                    plant_identity_id: eisbar_identity_id,
                    longitude: 0.57,
                    latitude: 0.83,
                    planted_on: Some("2023-10-21".into()),
                    row_name: Some("8. Bas".into()),
                    roles: vec!["fruit".into()],
                    is_alive: true,
                    is_in_danger: false,
                    reproductive_role: None,
                    harvest_start_day: None,
                    harvest_end_day: None,
                    adult_height_meters: Some(1.5),
                    adult_width_meters: Some(1.2),
                })
                .unwrap();
            Ok::<_, OrchardStorageError>(())
        })
        .unwrap();

    let boskoop = verification_connection
        .query_one(
            "SELECT reproductive_role, legacy_identification_name, legacy_identification_latin_name
             FROM trees WHERE legacy_feature_id = 64",
            &[],
        )
        .unwrap();
    assert_eq!(
        boskoop.get::<_, Option<String>>(0),
        Some("self_fertile".into())
    );
    assert_eq!(boskoop.get::<_, Option<String>>(1), None);
    assert_eq!(boskoop.get::<_, Option<String>>(2), None);

    let cranberry = verification_connection
        .query_one(
            "SELECT reproductive_role, legacy_identification_name, legacy_identification_latin_name
             FROM trees WHERE legacy_feature_id = 166",
            &[],
        )
        .unwrap();
    assert_eq!(cranberry.get::<_, Option<String>>(0), None);
    assert_eq!(
        cranberry.get::<_, Option<String>>(1),
        Some("Cranberry oxycoccos".into())
    );
    assert_eq!(
        cranberry.get::<_, Option<String>>(2),
        Some("Vaccinium macrocarpon ‘Howes’".into())
    );

    let eisbar = verification_connection
        .query_one(
            "SELECT legacy_source_url FROM trees WHERE legacy_feature_id = 157",
            &[],
        )
        .unwrap();
    assert_eq!(eisbar.get::<_, Option<String>>(0), Some(source_url.into()));
}

#[test]
fn roll_back_batch() {
    let _database_lock = database_lock();
    let (database_url, mut verification_connection) = empty_orchard_database();

    let mut orchard_storage = PostgresOrchardStorage::connect(&database_url).unwrap();
    let result = orchard_storage.transaction(|orchard| {
        let plant_identity_id = orchard.find_or_create_plant_identity(PlantIdentity {
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
        })?;
        orchard.save_tree(tree(plant_identity_id, 64))?;
        orchard.save_tree(tree(plant_identity_id, 132))?;
        assert!(orchard.is_legacy_tree_already_imported(64)?);
        orchard.save_tree(tree(plant_identity_id, 64))
    });

    assert_eq!(result, Err(OrchardStorageError::TreeCouldNotBeSaved));

    let persisted_tree_count: i64 = verification_connection
        .query_one("SELECT count(*) FROM trees", &[])
        .unwrap()
        .get(0);
    let persisted_identity_count: i64 = verification_connection
        .query_one("SELECT count(*) FROM plant_identities", &[])
        .unwrap()
        .get(0);
    assert_eq!(persisted_tree_count, 0);
    assert_eq!(persisted_identity_count, 0);
}

#[test]
fn migration_preserves_legacy_labels() {
    let _database_lock = database_lock();
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut connection = Client::connect(&database_url, NoTls).unwrap();
    connection
        .batch_execute(
            "DROP SCHEMA IF EXISTS migration_test CASCADE;
             CREATE SCHEMA migration_test;
             SET search_path TO migration_test, public;
             CREATE TABLE trees (
                id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                legacy_feature_id INTEGER UNIQUE,
                location geometry(Point, 4326) NOT NULL,
                name TEXT NOT NULL,
                latin_name TEXT,
                planted_on DATE,
                row_name TEXT,
                roles TEXT[] NOT NULL DEFAULT '{}',
                is_alive BOOLEAN NOT NULL,
                harvest_start_day SMALLINT,
                harvest_end_day SMALLINT,
                adult_height_meters DOUBLE PRECISION,
                adult_width_meters DOUBLE PRECISION
             );
             INSERT INTO trees (
                legacy_feature_id, location, name, latin_name, is_alive
             ) VALUES (
                17,
                ST_SetSRID(ST_MakePoint(0.72, 0.24), 4326),
                'Brugnon blanc ‘John Rivers’',
                'Prunus persica var. nucipersica ‘John Rivers’',
                TRUE
             );",
        )
        .unwrap();

    connection
        .batch_execute(include_str!(
            "../../../../db/migrations/002_add_plant_identities.sql"
        ))
        .unwrap();
    connection
        .batch_execute(include_str!(
            "../../../../db/migrations/003_preserve_legacy_tree_details.sql"
        ))
        .unwrap();
    connection
        .batch_execute(include_str!(
            "../../../../db/migrations/004_preserve_legacy_source_url.sql"
        ))
        .unwrap();
    connection
        .batch_execute(include_str!(
            "../../../../db/migrations/005_add_tree_danger.sql"
        ))
        .unwrap();

    let migrated_tree = connection
        .query_one(
            "SELECT legacy_feature_id, legacy_name, legacy_latin_name, plant_identity_id,
                    reproductive_role, legacy_identification_name, legacy_identification_latin_name,
                    legacy_source_url, is_in_danger
             FROM trees",
            &[],
        )
        .unwrap();
    assert_eq!(migrated_tree.get::<_, i32>(0), 17);
    assert_eq!(
        migrated_tree.get::<_, Option<String>>(1),
        Some("Brugnon blanc ‘John Rivers’".into())
    );
    assert_eq!(
        migrated_tree.get::<_, Option<String>>(2),
        Some("Prunus persica var. nucipersica ‘John Rivers’".into())
    );
    assert_eq!(migrated_tree.get::<_, Option<i64>>(3), None);
    assert_eq!(migrated_tree.get::<_, Option<String>>(4), None);
    assert_eq!(migrated_tree.get::<_, Option<String>>(5), None);
    assert_eq!(migrated_tree.get::<_, Option<String>>(6), None);
    assert_eq!(migrated_tree.get::<_, Option<String>>(7), None);
    assert!(!migrated_tree.get::<_, bool>(8));

    let old_name_column_count: i64 = connection
        .query_one(
            "SELECT count(*)
             FROM information_schema.columns
             WHERE table_schema = current_schema()
               AND table_name = 'trees'
               AND column_name = 'name'",
            &[],
        )
        .unwrap()
        .get(0);
    let legacy_name_is_nullable: String = connection
        .query_one(
            "SELECT is_nullable
             FROM information_schema.columns
             WHERE table_schema = current_schema()
               AND table_name = 'trees'
               AND column_name = 'legacy_name'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(old_name_column_count, 0);
    assert_eq!(legacy_name_is_nullable, "YES");

    let plant_identity_foreign_key_count: i64 = connection
        .query_one(
            "SELECT count(*)
             FROM pg_constraint
             WHERE conrelid = 'trees'::regclass
               AND conname = 'trees_plant_identity_id_fkey'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(plant_identity_foreign_key_count, 1);

    let danger_constraint_count: i64 = connection
        .query_one(
            "SELECT count(*)
             FROM pg_constraint
             WHERE conrelid = 'trees'::regclass
               AND conname = 'trees_danger_requires_alive_check'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(danger_constraint_count, 1);

    connection
        .batch_execute("DROP SCHEMA migration_test CASCADE")
        .unwrap();
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
        planted_on: Some("2024-12-07".into()),
        row_name: Some("4. Bas bas".into()),
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        reproductive_role: Some(ReproductiveRole::SelfFertile),
        harvest_start_day: None,
        harvest_end_day: None,
        adult_height_meters: Some(6.0),
        adult_width_meters: Some(5.0),
    }
}

fn empty_orchard_database() -> (String, Client) {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = Client::connect(&database_url, NoTls).unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/001_create_trees.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/002_add_plant_identities.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/003_preserve_legacy_tree_details.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/004_preserve_legacy_source_url.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/005_add_tree_danger.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute("TRUNCATE TABLE trees, plant_identities RESTART IDENTITY CASCADE")
        .unwrap();
    (database_url, verification_connection)
}

struct DatabaseLock {
    _connection: Client,
}

fn database_lock() -> DatabaseLock {
    const ORCHARD_TEST_DATABASE_LOCK: i64 = 7_208_004_281;

    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut connection = Client::connect(&database_url, NoTls).unwrap();
    connection
        .query_one(
            "SELECT pg_advisory_lock($1)",
            &[&ORCHARD_TEST_DATABASE_LOCK],
        )
        .unwrap();
    DatabaseLock {
        _connection: connection,
    }
}
