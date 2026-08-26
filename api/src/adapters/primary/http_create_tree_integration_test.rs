use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::bootstrap::start_http_server;
use orchard_api::hexagon::models::Tree;
use reqwest::StatusCode;

#[tokio::test]
async fn when_a_post_request_creates_a_tree_it_returns_created_and_saves_the_tree() {
    let (trees, observed_trees) = InMemoryOrchardStorage::new();
    let server = start_http_server(trees).await;

    let response = reqwest::Client::new()
        .post(format!("{}/trees", server.url()))
        .json(&serde_json::json!({
            "longitude": 0.72,
            "latitude": 0.24,
            "name": "Pommier",
            "latin_name": "Malus domestica",
            "roles": ["fruit"],
            "harvest_start_day": 210,
            "harvest_end_day": 260
        }))
        .send()
        .await
        .expect("the orchard test server should answer the request");

    let expected_tree = Tree {
        legacy_feature_id: None,
        longitude: 0.72,
        latitude: 0.24,
        name: "Pommier".into(),
        latin_name: Some("Malus domestica".into()),
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: None,
        adult_width_meters: None,
    };
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.json::<Tree>().await.unwrap(), expected_tree);
    assert_eq!(observed_trees.trees(), vec![expected_tree]);
}
