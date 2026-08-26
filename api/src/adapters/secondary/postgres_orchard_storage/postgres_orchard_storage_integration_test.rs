use orchard_api::adapters::secondary::PostgresOrchardStorage;
use orchard_api::hexagon::models::Tree;
use orchard_api::hexagon::ports::{OrchardTransaction, OrchardUnitOfWork, TreeRepository};
use postgres::{Client, NoTls};

#[test]
fn commit_persists_tree() {
    let database_url = std::env::var("ORCHARD_TEST_DATABASE_URL")
        .expect("ORCHARD_TEST_DATABASE_URL must point to the dedicated test database");
    let mut verification_connection = Client::connect(&database_url, NoTls).unwrap();
    verification_connection
        .batch_execute(include_str!(
            "../../../../db/migrations/001_create_trees.sql"
        ))
        .unwrap();
    verification_connection
        .batch_execute("TRUNCATE TABLE trees")
        .unwrap();
    let expected_tree = Tree {
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
    let mut orchard_unit_of_work = PostgresOrchardStorage::connect(&database_url).unwrap();

    let mut transaction = orchard_unit_of_work.begin().unwrap();
    transaction.save(expected_tree.clone()).unwrap();
    transaction.commit().unwrap();

    let persisted_tree = verification_connection
        .query_one(
            "SELECT legacy_feature_id, ST_X(location), ST_Y(location), name, latin_name, planted_on::text, row_name, roles, is_alive, harvest_start_day, harvest_end_day, adult_height_meters, adult_width_meters FROM trees WHERE legacy_feature_id = 1",
            &[],
        )
        .unwrap();
    assert_eq!(persisted_tree.get::<_, i32>(0), 1);
    assert_eq!(persisted_tree.get::<_, f64>(1), expected_tree.longitude);
    assert_eq!(persisted_tree.get::<_, f64>(2), expected_tree.latitude);
    assert_eq!(persisted_tree.get::<_, String>(3), expected_tree.name);
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(4),
        expected_tree.latin_name
    );
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(5),
        expected_tree.planted_on
    );
    assert_eq!(
        persisted_tree.get::<_, Option<String>>(6),
        expected_tree.row_name
    );
    assert_eq!(persisted_tree.get::<_, Vec<String>>(7), expected_tree.roles);
    assert_eq!(persisted_tree.get::<_, bool>(8), expected_tree.is_alive);
    assert_eq!(
        persisted_tree.get::<_, Option<i16>>(9),
        expected_tree.harvest_start_day.map(|day| day as i16)
    );
    assert_eq!(
        persisted_tree.get::<_, Option<i16>>(10),
        expected_tree.harvest_end_day.map(|day| day as i16)
    );
    assert_eq!(
        persisted_tree.get::<_, Option<f64>>(11),
        expected_tree.adult_height_meters
    );
    assert_eq!(
        persisted_tree.get::<_, Option<f64>>(12),
        expected_tree.adult_width_meters
    );
}
