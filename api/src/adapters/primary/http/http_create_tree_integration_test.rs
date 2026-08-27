use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::bootstrap::start_http_server;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, PlantIdentity, PlantIdentityId, Tree,
};
use reqwest::StatusCode;

#[tokio::test]
async fn create_tree_http() {
    let (orchard, observed_orchard) = InMemoryOrchardStorage::new();
    let server = start_http_server(orchard).await;
    let apple = malus_domestica();

    let response = reqwest::Client::new()
        .post(format!("{}/trees", server.url()))
        .json(&serde_json::json!({
            "longitude": 0.72,
            "latitude": 0.24,
            "plant_identity": apple,
            "roles": ["fruit"],
            "harvest_start_day": 210,
            "harvest_end_day": 260
        }))
        .send()
        .await
        .expect("the orchard test server should answer the request");

    let expected_tree = Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude: 0.72,
        latitude: 0.24,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: None,
        adult_width_meters: None,
    };
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.json::<Tree>().await.unwrap(), expected_tree);
    assert_eq!(observed_orchard.plant_identities(), vec![malus_domestica()]);
    assert_eq!(observed_orchard.trees(), vec![expected_tree]);
}

fn malus_domestica() -> PlantIdentity {
    PlantIdentity {
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
    }
}
