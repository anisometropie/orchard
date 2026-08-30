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
    let (first_tree, second_tree) = runtime.block_on(async {
        let client = reqwest::Client::new();
        let first_tree = create_apple_tree(&client, &server_url, 0.72).await;
        let second_tree = create_apple_tree(&client, &server_url, 0.18).await;
        (first_tree, second_tree)
    });
    drop(runtime);

    assert_eq!(first_tree["plant_identity_id"], 1);
    assert_eq!(second_tree["plant_identity_id"], 1);
    assert!(
        server.is_running(),
        "the server should continue accepting requests"
    );
    assert_eq!(database_snapshot(&mut verification_connection), (2, 1));

    let persisted_tree = verification_connection
        .query_one(
            "SELECT plant_identities.common_name, trees.roles,
                    trees.harvest_start_day, trees.harvest_end_day
             FROM trees
             INNER JOIN plant_identities ON plant_identities.id = trees.plant_identity_id
             ORDER BY trees.id
             LIMIT 1",
            &[],
        )
        .unwrap();
    assert_eq!(persisted_tree.get::<_, String>(0), "Pommier");
    assert_eq!(persisted_tree.get::<_, Vec<String>>(1), vec!["fruit"]);
    assert_eq!(persisted_tree.get::<_, Option<i16>>(2), Some(210));
    assert_eq!(persisted_tree.get::<_, Option<i16>>(3), Some(260));
}

async fn create_apple_tree(
    client: &reqwest::Client,
    server_url: &str,
    longitude: f64,
) -> serde_json::Value {
    let response = client
        .post(format!("{server_url}/trees"))
        .json(&serde_json::json!({
            "longitude": longitude,
            "latitude": 0.24,
            "plant_identity": {
                "common_name": "Pommier",
                "botanical_taxon": {
                    "Named": {
                        "genus": "Malus",
                        "species": "domestica",
                        "species_is_hybrid": false,
                        "infraspecific": null,
                        "is_aggregate": false,
                        "cultivar_group": null
                    }
                },
                "cultivar": null,
                "trade_name": null,
                "identification_status": "Confirmed"
            },
            "roles": ["fruit"],
            "harvest_start_day": 210,
            "harvest_end_day": 260
        }))
        .send()
        .await
        .expect("the server should answer the create-tree request");

    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.unwrap()
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
    verification_connection
        .batch_execute(
            "TRUNCATE TABLE aerial_overlays, users, trees, plant_identities
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
