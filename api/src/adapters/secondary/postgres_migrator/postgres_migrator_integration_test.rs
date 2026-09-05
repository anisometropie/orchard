use postgres::{Client, NoTls};

use super::{MIGRATIONS, Migration, MigrationError, PostgresMigrator};

const TEST_DATABASE_LOCK: i64 = 7_208_004_281;

#[test]
fn migrate_fresh_adopt_legacy_and_reject_checksum_drift() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();

    let mut migrator = empty_test_schema(&database_url);
    let first_run = migrator.migrate().unwrap();
    assert_eq!(
        first_run.applied_versions,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14]
    );
    assert!(!first_run.adopted_legacy_schema);
    assert_eq!(
        migrator.migrate().unwrap().applied_versions,
        Vec::<u32>::new()
    );

    assert_eq!(
        migrator.revert_to(10).unwrap().reverted_versions,
        vec![14, 13, 12, 11]
    );
    migrator
        .client()
        .batch_execute("DROP TABLE orchard_schema_migrations")
        .unwrap();
    let adoption = migrator.migrate().unwrap();
    assert!(adoption.adopted_legacy_schema);
    assert_eq!(adoption.applied_versions, vec![11, 12, 13, 14]);

    migrator
        .client()
        .execute(
            "UPDATE orchard_schema_migrations SET checksum = 'modified' WHERE version = 12",
            &[],
        )
        .unwrap();
    assert_eq!(
        migrator.migrate(),
        Err(MigrationError::AppliedMigrationChecksumChanged { version: 12 })
    );

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn assign_existing_orchard_data_to_the_default_users_orchard_before_adding_authentication() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    let version_10_migrations = MIGRATIONS
        .iter()
        .copied()
        .filter(|migration| migration.version <= 10)
        .collect::<Vec<_>>();
    migrator
        .migrate_while_locked(&version_10_migrations)
        .unwrap();
    migrator
        .client()
        .batch_execute(
            r#"
            INSERT INTO users (username, default_center, is_default)
            VALUES (
                'owner',
                ST_SetSRID(ST_MakePoint(5.01745, 45.25337), 4326),
                TRUE
            );
            INSERT INTO plant_identities (common_name, botanical_taxon)
            VALUES ('Apple', '{"Named":{"genus":"Malus"}}');
            INSERT INTO trees (
                plant_identity_id, location, roles, is_alive, identification_status
            ) VALUES (
                1,
                ST_SetSRID(ST_MakePoint(5.02, 45.25), 4326),
                '{fruit}',
                TRUE,
                'confirmed'
            );
            INSERT INTO plant_harvest_windows (
                plant_identity_id, start_month, start_day, end_month, end_day,
                reference_region, harvested_part, data_origin
            ) VALUES (
                1, 9, 1, 10, 15,
                'Hauterives, Drôme, France', 'fruit', 'field_observation'
            );
            INSERT INTO aerial_overlays (
                user_id, name, image_bytes, media_type,
                top_left, top_right, bottom_right, bottom_left
            ) VALUES (
                1, 'Aerial', '\\x01', 'image/png',
                ST_SetSRID(ST_MakePoint(5.0, 45.3), 4326),
                ST_SetSRID(ST_MakePoint(5.1, 45.3), 4326),
                ST_SetSRID(ST_MakePoint(5.1, 45.2), 4326),
                ST_SetSRID(ST_MakePoint(5.0, 45.2), 4326)
            );
            "#,
        )
        .unwrap();

    assert_eq!(
        migrator.migrate().unwrap().applied_versions,
        vec![11, 12, 13, 14]
    );

    let migrated = migrator
        .client()
        .query_one(
            "SELECT
                orchard.name,
                orchard.reference_region,
                ST_X(orchard.center),
                ST_Y(orchard.center),
                tree.orchard_id = orchard.id,
                harvest.orchard_id = orchard.id,
                overlay.orchard_id = orchard.id,
                owner.password_hash IS NULL,
                orchard.share_token_hash IS NULL
             FROM orchards orchard
             JOIN users owner ON owner.id = orchard.owner_user_id
             JOIN trees tree ON TRUE
             JOIN plant_harvest_windows harvest ON TRUE
             JOIN aerial_overlays overlay ON TRUE",
            &[],
        )
        .unwrap();
    assert_eq!(migrated.get::<_, String>(0), "My orchard");
    assert_eq!(migrated.get::<_, String>(1), "Hauterives, Drôme, France");
    assert_eq!(migrated.get::<_, f64>(2), 5.01745);
    assert_eq!(migrated.get::<_, f64>(3), 45.25337);
    for column in 4..=8 {
        assert!(migrated.get::<_, bool>(column));
    }

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn refuse_to_guess_the_version_of_a_partial_untracked_schema() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    migrator
        .client()
        .batch_execute("CREATE TABLE plant_identities (id BIGINT PRIMARY KEY)")
        .unwrap();

    assert_eq!(
        migrator.migrate(),
        Err(MigrationError::UnexpectedLegacySchema)
    );
    let trees_table_exists = migrator
        .client()
        .query_one(
            "SELECT to_regclass(format('%I.trees', current_schema())) IS NOT NULL",
            &[],
        )
        .unwrap()
        .get::<_, bool>(0);
    assert!(!trees_table_exists);
    let recorded_migration_count = migrator
        .client()
        .query_one("SELECT count(*) FROM orchard_schema_migrations", &[])
        .unwrap()
        .get::<_, i64>(0);
    assert_eq!(recorded_migration_count, 0);

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn revert_and_reapply_the_embedded_migration_chain() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);

    migrator.migrate().unwrap();
    assert_eq!(
        migrator.revert_to(0).unwrap().reverted_versions,
        vec![14, 13, 12, 11, 10, 8, 7, 6, 5, 4, 3, 2, 1]
    );
    let reverted = migrator
        .client()
        .query_one(
            "SELECT
                to_regclass(format('%I.trees', current_schema())) IS NULL,
                to_regclass(format('%I.users', current_schema())) IS NULL,
                to_regtype(format('%I.harvested_part', current_schema())) IS NULL,
                (SELECT count(*) FROM orchard_schema_migrations)",
            &[],
        )
        .unwrap();
    assert!(reverted.get::<_, bool>(0));
    assert!(reverted.get::<_, bool>(1));
    assert!(reverted.get::<_, bool>(2));
    assert_eq!(reverted.get::<_, i64>(3), 0);
    assert_eq!(
        migrator.migrate().unwrap().applied_versions,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14]
    );

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn preserve_representable_orchard_data_while_reverting_to_version_6_and_reapplying() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    migrator.migrate().unwrap();
    migrator
        .client()
        .batch_execute(
            "WITH identity AS (
                INSERT INTO plant_identities (common_name, botanical_taxon)
                VALUES ('Apple', '{\"Named\": {\"genus\": \"Malus\"}}')
                RETURNING id
             ), cultivar AS (
                INSERT INTO plant_cultivars (plant_identity_id, cultivar, trade_name)
                SELECT id, 'Discovery', 'Discover Me' FROM identity
                RETURNING id, plant_identity_id
             ), tree AS (
                INSERT INTO trees (
                    plant_identity_id, cultivar_id, identification_status,
                    location, roles, is_alive
                )
                SELECT
                    plant_identity_id, id, 'uncertain',
                    ST_SetSRID(ST_MakePoint(1, 2), 4326), '{}', TRUE
                FROM cultivar
             )
             INSERT INTO plant_harvest_windows (
                plant_identity_id, cultivar_id,
                start_month, start_day, end_month, end_day,
                harvested_part, data_origin
             )
             SELECT
                id, NULL, 2, 29, 8, 20,
                'fruit', 'external_reference'
             FROM identity",
        )
        .unwrap();

    assert_eq!(
        migrator.revert_to(6).unwrap().reverted_versions,
        vec![14, 13, 12, 11, 10, 8, 7]
    );
    let version_6_tree = migrator
        .client()
        .query_one(
            "SELECT
                identity.cultivar,
                identity.trade_name,
                identity.identification_status,
                tree.harvest_start_day,
                tree.harvest_end_day
             FROM trees tree
             JOIN plant_identities identity ON identity.id = tree.plant_identity_id",
            &[],
        )
        .unwrap();
    assert_eq!(version_6_tree.get::<_, String>(0), "Discovery");
    assert_eq!(version_6_tree.get::<_, String>(1), "Discover Me");
    assert_eq!(version_6_tree.get::<_, String>(2), "uncertain");
    assert_eq!(version_6_tree.get::<_, i16>(3), 60);
    assert_eq!(version_6_tree.get::<_, i16>(4), 233);

    assert_eq!(
        migrator.migrate().unwrap().applied_versions,
        vec![7, 8, 10, 11, 12, 13, 14]
    );
    let version_10_tree = migrator
        .client()
        .query_one(
            "SELECT
                cultivar.cultivar,
                cultivar.trade_name,
                tree.identification_status,
                harvest_window.start_month,
                harvest_window.start_day,
                harvest_window.end_month,
                harvest_window.end_day
             FROM trees tree
             JOIN plant_cultivars cultivar ON cultivar.id = tree.cultivar_id
             JOIN plant_harvest_windows harvest_window
               ON harvest_window.plant_identity_id = tree.plant_identity_id
              AND harvest_window.cultivar_id IS NULL",
            &[],
        )
        .unwrap();
    assert_eq!(version_10_tree.get::<_, String>(0), "Discovery");
    assert_eq!(version_10_tree.get::<_, String>(1), "Discover Me");
    assert_eq!(version_10_tree.get::<_, String>(2), "uncertain");
    assert_eq!(version_10_tree.get::<_, i16>(3), 2);
    assert_eq!(version_10_tree.get::<_, i16>(4), 29);
    assert_eq!(version_10_tree.get::<_, i16>(5), 8);
    assert_eq!(version_10_tree.get::<_, i16>(6), 20);

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn revert_is_atomic_when_an_older_down_migration_fails() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    let migrations = [
        Migration {
            version: 1,
            name: "base",
            sql: "CREATE TABLE revert_base (id INTEGER PRIMARY KEY)",
            down_sql: Some("DROP TABLE revert_base"),
        },
        Migration {
            version: 2,
            name: "guarded",
            sql: "CREATE TABLE revert_guarded (id INTEGER PRIMARY KEY); INSERT INTO revert_guarded VALUES (1)",
            down_sql: Some(
                "DO $$ BEGIN
                    IF EXISTS (SELECT 1 FROM revert_guarded) THEN
                        RAISE EXCEPTION 'guarded data exists';
                    END IF;
                 END $$;
                 DROP TABLE revert_guarded",
            ),
        },
        Migration {
            version: 3,
            name: "latest",
            sql: "CREATE TABLE revert_latest (id INTEGER PRIMARY KEY)",
            down_sql: Some("DROP TABLE revert_latest"),
        },
    ];

    migrator.migrate_while_locked(&migrations).unwrap();
    assert!(matches!(
        migrator.revert_to_while_locked(&migrations, 1),
        Err(MigrationError::MigrationCouldNotBeReverted { version: 2, .. })
    ));
    let unchanged = migrator
        .client()
        .query_one(
            "SELECT
                to_regclass(format('%I.revert_latest', current_schema())) IS NOT NULL,
                ARRAY(SELECT version FROM orchard_schema_migrations ORDER BY version)",
            &[],
        )
        .unwrap();
    assert!(unchanged.get::<_, bool>(0));
    assert_eq!(unchanged.get::<_, Vec<i32>>(1), vec![1, 2, 3]);

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn refuse_a_revert_with_a_missing_down_migration_before_changing_schema() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    let migrations = [
        Migration {
            version: 1,
            name: "base",
            sql: "CREATE TABLE irreversible_base (id INTEGER PRIMARY KEY)",
            down_sql: Some("DROP TABLE irreversible_base"),
        },
        Migration {
            version: 2,
            name: "irreversible",
            sql: "CREATE TABLE irreversible_change (id INTEGER PRIMARY KEY)",
            down_sql: None,
        },
        Migration {
            version: 3,
            name: "newest",
            sql: "CREATE TABLE reversible_newest (id INTEGER PRIMARY KEY)",
            down_sql: Some("DROP TABLE reversible_newest"),
        },
    ];

    migrator.migrate_while_locked(&migrations).unwrap();
    assert_eq!(
        migrator.revert_to_while_locked(&migrations, 1),
        Err(MigrationError::IrreversibleMigration { version: 2 })
    );
    let newest_table_still_exists = migrator
        .client()
        .query_one(
            "SELECT to_regclass(format('%I.reversible_newest', current_schema())) IS NOT NULL",
            &[],
        )
        .unwrap()
        .get::<_, bool>(0);
    assert!(newest_table_still_exists);

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

#[test]
fn roll_back_both_schema_and_ledger_when_one_migration_fails() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut database_lock = Client::connect(&database_url, NoTls).unwrap();
    database_lock
        .query_one("SELECT pg_advisory_lock($1)", &[&TEST_DATABASE_LOCK])
        .unwrap();
    let mut migrator = empty_test_schema(&database_url);
    let migrations = [
        Migration {
            version: 1,
            name: "valid",
            sql: "CREATE TABLE valid_migration (id INTEGER PRIMARY KEY)",
            down_sql: None,
        },
        Migration {
            version: 2,
            name: "invalid",
            sql: "CREATE TABLE must_be_rolled_back (id INTEGER); SELECT missing_function()",
            down_sql: None,
        },
    ];

    assert!(matches!(
        migrator.migrate_while_locked(&migrations),
        Err(MigrationError::MigrationCouldNotBeApplied { version: 2, .. })
    ));
    let snapshot = migrator
        .client()
        .query_one(
            "SELECT
                to_regclass(format('%I.valid_migration', current_schema())) IS NOT NULL,
                to_regclass(format('%I.must_be_rolled_back', current_schema())) IS NULL,
                ARRAY(SELECT version FROM orchard_schema_migrations ORDER BY version)",
            &[],
        )
        .unwrap();
    assert!(snapshot.get::<_, bool>(0));
    assert!(snapshot.get::<_, bool>(1));
    assert_eq!(snapshot.get::<_, Vec<i32>>(2), vec![1]);

    migrator
        .client()
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA migration_runner_test CASCADE",
        )
        .unwrap();
}

fn empty_test_schema(database_url: &str) -> PostgresMigrator {
    let mut client = Client::connect(database_url, NoTls).unwrap();
    client
        .batch_execute(
            "DROP SCHEMA IF EXISTS migration_runner_test CASCADE;
             CREATE SCHEMA migration_runner_test;
             SET search_path TO migration_runner_test, public",
        )
        .unwrap();
    PostgresMigrator::from_client(client)
}
