use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use reqwest::StatusCode;

#[tokio::test]
async fn legacy_single_tree_creation_is_not_public() {
    let (storage, observer) = InMemoryOrchardStorage::new();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .post(format!("{}/trees", server.url()))
        .json(&serde_json::json!({
            "longitude": 0.72,
            "latitude": 0.24,
            "roles": ["fruit"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(observer.trees().is_empty());
}
