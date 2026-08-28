use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, PlantIdentity, PlantIdentityId, Tree,
};
use reqwest::StatusCode;

#[tokio::test]
async fn create_tree_http() {
    let (orchard, observed_orchard) = InMemoryOrchardStorage::new();
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
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

#[tokio::test]
async fn list_orchard_trees_as_geojson() {
    let apple = malus_domestica();
    let tree = Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude: 0.64,
        latitude: 0.68,
        planted_on: Some("2024-02-03".into()),
        row_name: Some("1. Haut haut haut".into()),
        roles: vec!["fruit".into(), "pioneer".into()],
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: Some(4.0),
        adult_width_meters: Some(3.0),
    };
    let (orchard, _) = InMemoryOrchardStorage::with_existing_orchard(vec![apple], vec![tree]);
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::get(format!("{}/trees.geojson", server.url()))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap(),
        serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [0.64, 0.68]
                },
                "properties": {
                    "name": "Pommier",
                    "latin_name": "Malus domestica",
                    "planted_on": "2024-02-03",
                    "row_name": "1. Haut haut haut",
                    "roles": ["fruit", "pioneer"],
                    "is_alive": true,
                    "adult_height": 4.0,
                    "adult_width": 3.0
                }
            }]
        })
    );
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
