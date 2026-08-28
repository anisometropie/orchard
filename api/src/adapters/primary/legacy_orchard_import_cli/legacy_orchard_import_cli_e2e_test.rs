use std::{env, process::Command};

use postgres::{Client, NoTls};

#[test]
fn import_orchard() {
    let import_command = env!("CARGO_BIN_EXE_import_legacy_orchard");
    let _database_lock = database_lock();
    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = empty_orchard_database(&database_url);

    let first_import = Command::new(import_command)
        .env("ORCHARD_DATABASE_URL", &database_url)
        .output()
        .expect("the import command should start");

    assert!(
        first_import.status.success(),
        "{}",
        String::from_utf8_lossy(&first_import.stderr)
    );
    assert_eq!(
        String::from_utf8(first_import.stdout).unwrap(),
        "Imported 278 trees.\n"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (278, 152));

    let pink_giant = verification_connection
        .query_one(
            "SELECT plant_identities.common_name, plant_identities.cultivar, plant_identities.trade_name
             FROM trees
             INNER JOIN plant_identities ON plant_identities.id = trees.plant_identity_id
             WHERE trees.legacy_feature_id = 215",
            &[],
        )
        .unwrap();
    assert_eq!(pink_giant.get::<_, String>(0), "Arbousier");
    assert_eq!(pink_giant.get::<_, Option<String>>(1), Some("Nevez".into()));
    assert_eq!(
        pink_giant.get::<_, Option<String>>(2),
        Some("Pink Giant".into())
    );

    let second_import = Command::new(import_command)
        .env("ORCHARD_DATABASE_URL", &database_url)
        .output()
        .expect("the import command should start a second time");

    assert_eq!(second_import.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(second_import.stderr).unwrap(),
        "Import failed: legacy feature 1 is already imported. No changes were made.\n"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (278, 152));
}

fn empty_orchard_database(database_url: &str) -> Client {
    let mut verification_connection = Client::connect(database_url, NoTls).unwrap();
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
        .batch_execute("TRUNCATE TABLE trees, plant_identities RESTART IDENTITY CASCADE")
        .unwrap();
    verification_connection
}

fn database_snapshot(verification_connection: &mut Client) -> (i64, i64) {
    let tree_count = verification_connection
        .query_one("SELECT count(*) FROM trees", &[])
        .unwrap()
        .get(0);
    let plant_identity_count = verification_connection
        .query_one("SELECT count(*) FROM plant_identities", &[])
        .unwrap()
        .get(0);
    (tree_count, plant_identity_count)
}

struct DatabaseLock {
    _connection: Client,
}

fn database_lock() -> DatabaseLock {
    const ORCHARD_TEST_DATABASE_LOCK: i64 = 7_208_004_281;

    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
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
