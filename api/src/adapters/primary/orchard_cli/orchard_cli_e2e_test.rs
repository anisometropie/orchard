use std::{
    env,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
    time::Duration,
};

use postgres::{Client, NoTls};
use reqwest::StatusCode;

#[test]
fn adopt_an_existing_untracked_database_and_make_repeated_migration_safe() {
    let orchard_command = env!("CARGO_BIN_EXE_orchard");
    let _database_lock = database_lock();
    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let (migration_database_url, mut verification_connection) =
        untracked_version_10_schema(&database_url);

    let adoption = Command::new(orchard_command)
        .arg("migrate")
        .env("ORCHARD_DATABASE_URL", &migration_database_url)
        .output()
        .expect("the migration command should start");

    assert!(
        adoption.status.success(),
        "{}",
        String::from_utf8_lossy(&adoption.stderr)
    );
    assert_eq!(
        String::from_utf8(adoption.stdout).unwrap(),
        "Adopted the existing version-10 schema.\nApplied migrations: 11, 12.\n"
    );
    let versions = verification_connection
        .query(
            "SELECT version FROM orchard_schema_migrations ORDER BY version",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get::<_, i32>(0))
        .collect::<Vec<_>>();
    assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12]);

    let repeated = Command::new(orchard_command)
        .arg("migrate")
        .env("ORCHARD_DATABASE_URL", &migration_database_url)
        .output()
        .expect("the migration command should start again");
    assert!(repeated.status.success());
    assert_eq!(
        String::from_utf8(repeated.stdout).unwrap(),
        "Database schema is up to date.\n"
    );
    let revert = Command::new(orchard_command)
        .args(["migrate", "revert", "--to", "10"])
        .env("ORCHARD_DATABASE_URL", &migration_database_url)
        .output()
        .expect("the migration revert command should start");
    assert!(revert.status.success());
    assert_eq!(
        String::from_utf8(revert.stdout).unwrap(),
        "Reverted migrations: 12, 11.\n"
    );
    verification_connection
        .batch_execute(
            "SET search_path TO public;
             DROP SCHEMA cli_migration_test CASCADE",
        )
        .unwrap();
}

fn untracked_version_10_schema(database_url: &str) -> (String, Client) {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    let schema_database_url =
        format!("{database_url}{separator}options=-csearch_path%3Dcli_migration_test%2Cpublic");
    let mut admin = Client::connect(database_url, NoTls).unwrap();
    admin
        .batch_execute(
            "DROP SCHEMA IF EXISTS cli_migration_test CASCADE;
             CREATE SCHEMA cli_migration_test",
        )
        .unwrap();
    let mut connection = Client::connect(&schema_database_url, NoTls).unwrap();
    for migration in [
        include_str!("../../../../db/migrations/001_create_trees.sql"),
        include_str!("../../../../db/migrations/002_add_plant_identities.sql"),
        include_str!("../../../../db/migrations/003_preserve_legacy_tree_details.sql"),
        include_str!("../../../../db/migrations/004_preserve_legacy_source_url.sql"),
        include_str!("../../../../db/migrations/005_add_tree_danger.sql"),
        include_str!("../../../../db/migrations/006_create_users_and_aerial_overlays.sql"),
        include_str!("../../../../db/migrations/007_normalize_plant_identities.sql"),
        include_str!("../../../../db/migrations/008_create_plant_harvest_windows.sql"),
        include_str!("../../../../db/migrations/010_describe_harvest_windows.sql"),
    ] {
        connection.batch_execute(migration).unwrap();
    }
    (schema_database_url, connection)
}

#[test]
fn import_orchard() {
    let orchard_command = env!("CARGO_BIN_EXE_orchard");
    let _database_lock = database_lock();
    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = empty_orchard_database(&database_url);
    let legacy_geojson = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/adapters/primary/orchard_cli/one-tree.geojson");

    let first_import = Command::new(orchard_command)
        .args(["import_legacy_orchard"])
        .arg(&legacy_geojson)
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
        "Imported 1 trees.\n"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (1, 1));

    let second_import = Command::new(orchard_command)
        .args(["import_legacy_orchard"])
        .arg(&legacy_geojson)
        .env("ORCHARD_DATABASE_URL", &database_url)
        .output()
        .expect("the import command should start a second time");

    assert_eq!(second_import.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(second_import.stderr).unwrap(),
        "Import failed: legacy feature 1 is already imported. No changes were made.\n"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (1, 1));
}

#[test]
fn import_one_tree() {
    let orchard_command = env!("CARGO_BIN_EXE_orchard");
    let _database_lock = database_lock();
    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = empty_orchard_database(&database_url);
    let one_tree_geojson = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/adapters/primary/orchard_cli/one-tree.geojson");

    let import = Command::new(orchard_command)
        .args(["import_legacy_orchard"])
        .arg(&one_tree_geojson)
        .env("ORCHARD_DATABASE_URL", &database_url)
        .output()
        .expect("the import command should start");

    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    assert_eq!(
        String::from_utf8(import.stdout).unwrap(),
        "Imported 1 trees.\n"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (1, 1));

    let persisted_tree = verification_connection
        .query_one(
            "SELECT plant_identities.common_name, trees.row_name, trees.is_alive
             FROM trees
             INNER JOIN plant_identities ON plant_identities.id = trees.plant_identity_id",
            &[],
        )
        .unwrap();
    assert_eq!(persisted_tree.get::<_, String>(0), "Pistachier térébinthe");
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(1),
        Some("10. Bas bas bas".into())
    );
    assert!(!persisted_tree.get::<_, bool>(2));
}

#[test]
fn require_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_orchard"))
        .args(["runserver", "--address", "127.0.0.1:0"])
        .env_remove("ORCHARD_DATABASE_URL")
        .output()
        .expect("the server command should start");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "ORCHARD_DATABASE_URL is not configured.\n"
    );
}

#[test]
fn runserver() {
    let orchard_command = env!("CARGO_BIN_EXE_orchard");
    let _database_lock = database_lock();
    let database_url = env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = empty_orchard_database(&database_url);
    let mut server = ServerProcess::start(orchard_command, &database_url);
    let server_url = server.wait_until_ready();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let legacy_endpoint_status = runtime.block_on(async {
        reqwest::Client::new()
            .get(format!("{server_url}/trees.geojson"))
            .send()
            .await
            .unwrap()
            .status()
    });
    drop(runtime);

    assert_eq!(legacy_endpoint_status, StatusCode::UNAUTHORIZED);
    assert!(
        server.is_running(),
        "the server should continue accepting requests"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (0, 0));
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
        .batch_execute(include_str!(
            "../../../../db/migrations/005_add_tree_danger.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/006_create_users_and_aerial_overlays.sql"
        ))
        .unwrap();
    let normalization_was_applied: bool = verification_connection
        .query_one("SELECT to_regclass('plant_cultivars') IS NOT NULL", &[])
        .unwrap()
        .get(0);
    if !normalization_was_applied {
        verification_connection
            .batch_execute(include_str!(
                "../../../../db/migrations/007_normalize_plant_identities.sql"
            ))
            .unwrap();
    }
    let harvest_windows_were_applied: bool = verification_connection
        .query_one(
            "SELECT to_regclass('plant_harvest_windows') IS NOT NULL",
            &[],
        )
        .unwrap()
        .get(0);
    if !harvest_windows_were_applied {
        verification_connection
            .batch_execute(include_str!(
                "../../../../db/migrations/008_create_plant_harvest_windows.sql"
            ))
            .unwrap();
    }
    let harvest_window_metadata_was_applied: bool = verification_connection
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = current_schema()
                  AND table_name = 'plant_harvest_windows'
                  AND column_name = 'data_origin'
             )",
            &[],
        )
        .unwrap()
        .get(0);
    if !harvest_window_metadata_was_applied {
        verification_connection
            .batch_execute(include_str!(
                "../../../../db/migrations/010_describe_harvest_windows.sql"
            ))
            .unwrap();
    }
    verification_connection
        .batch_execute(
            "TRUNCATE TABLE plant_harvest_windows, aerial_overlays, users, trees,
                            plant_cultivars, plant_identities
             RESTART IDENTITY CASCADE",
        )
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

struct ServerProcess {
    child: Child,
    readiness: Receiver<Result<String, String>>,
    readiness_reader: Option<JoinHandle<()>>,
}

impl ServerProcess {
    fn start(orchard_command: &str, database_url: &str) -> Self {
        let mut child = Command::new(orchard_command)
            .args(["runserver", "--address", "127.0.0.1:0"])
            .env("ORCHARD_DATABASE_URL", database_url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("the server command should start");
        let stdout = child
            .stdout
            .take()
            .expect("the server command should provide standard output");
        let (readiness_sender, readiness) = mpsc::channel();
        let readiness_reader = std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout)
                .read_line(&mut line)
                .map_err(|error| error.to_string())
                .and_then(|bytes_read| {
                    if bytes_read == 0 {
                        Err("the server stopped before reporting its address".into())
                    } else {
                        Ok(line)
                    }
                });
            let _ = readiness_sender.send(result);
        });

        Self {
            child,
            readiness,
            readiness_reader: Some(readiness_reader),
        }
    }

    fn wait_until_ready(&self) -> String {
        let line = self
            .readiness
            .recv_timeout(Duration::from_secs(5))
            .expect("the server should report its address within five seconds")
            .expect("the server should report its address");
        line.strip_prefix("Listening on ")
            .expect("the server should use the documented readiness message")
            .trim()
            .into()
    }

    fn is_running(&mut self) -> bool {
        self.child
            .try_wait()
            .expect("the server process should be observable")
            .is_none()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        if let Some(readiness_reader) = self.readiness_reader.take() {
            let _ = readiness_reader.join();
        }
    }
}
