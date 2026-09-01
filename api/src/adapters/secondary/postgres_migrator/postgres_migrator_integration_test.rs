use postgres::{Client, NoTls};

use super::{Migration, MigrationError, PostgresMigrator};

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
    assert_eq!(first_run.applied_versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 10]);
    assert!(!first_run.adopted_legacy_schema);
    assert_eq!(
        migrator.migrate().unwrap().applied_versions,
        Vec::<u32>::new()
    );

    migrator
        .client()
        .batch_execute("DROP TABLE orchard_schema_migrations")
        .unwrap();
    let adoption = migrator.migrate().unwrap();
    assert!(adoption.adopted_legacy_schema);
    assert_eq!(adoption.applied_versions, Vec::<u32>::new());

    migrator
        .client()
        .execute(
            "UPDATE orchard_schema_migrations SET checksum = 'modified' WHERE version = 10",
            &[],
        )
        .unwrap();
    assert_eq!(
        migrator.migrate(),
        Err(MigrationError::AppliedMigrationChecksumChanged { version: 10 })
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
        },
        Migration {
            version: 2,
            name: "invalid",
            sql: "CREATE TABLE must_be_rolled_back (id INTEGER); SELECT missing_function()",
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
