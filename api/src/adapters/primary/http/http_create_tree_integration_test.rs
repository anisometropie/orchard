use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, BotanicalTaxon, GeoPoint,
    IdentificationStatus, MapConfiguration, NamedTaxon, PlantIdentity, PlantIdentityId, Tree,
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
        is_in_danger: false,
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
        is_in_danger: true,
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
                "id": 1,
                "geometry": {
                    "type": "Point",
                    "coordinates": [0.64, 0.68]
                },
                "properties": {
                    "name": "Pommier",
                    "latin_name": "Malus domestica",
                    "plant_identity_id": 1,
                    "plant_identity_name": "Pommier",
                    "plant_identity_taxon_name": "Malus domestica",
                    "plant_identity_botanical_name": "Malus domestica",
                    "plant_identity_cultivar": null,
                    "botanical_genera": ["Malus"],
                    "botanical_species": ["Malus domestica"],
                    "planted_on": "2024-02-03",
                    "row_name": "1. Haut haut haut",
                    "roles": ["fruit", "pioneer"],
                    "is_alive": true,
                    "is_in_danger": true,
                    "adult_height": 4.0,
                    "adult_width": 3.0
                }
            }]
        })
    );
}

#[tokio::test]
async fn partially_update_tree_danger_http() {
    let tree = tree_with_condition(true, false);
    let (orchard, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![malus_domestica()], vec![tree]);
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .patch(format!("{}/trees/1", server.url()))
        .json(&serde_json::json!({ "is_in_danger": true }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(observed_orchard.trees()[0].is_in_danger);
}

#[tokio::test]
async fn report_missing_tree_when_partially_updating_http() {
    let (orchard, _) = InMemoryOrchardStorage::with_existing_orchard(
        vec![malus_domestica()],
        vec![tree_with_condition(true, false)],
    );
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .patch(format!("{}/trees/2", server.url()))
        .json(&serde_json::json!({ "is_in_danger": true }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn reject_partial_update_that_marks_a_dead_tree_in_danger() {
    let (orchard, observed_orchard) = InMemoryOrchardStorage::with_existing_orchard(
        vec![malus_domestica()],
        vec![tree_with_condition(false, false)],
    );
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .patch(format!("{}/trees/1", server.url()))
        .json(&serde_json::json!({ "is_in_danger": true }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(!observed_orchard.trees()[0].is_in_danger);
}

#[tokio::test]
async fn partially_update_tree_life_status_and_clear_danger() {
    let tree = tree_with_condition(true, true);
    let (orchard, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![malus_domestica()], vec![tree]);
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .patch(format!("{}/trees/1", server.url()))
        .json(&serde_json::json!({ "is_alive": false }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let tree = &observed_orchard.trees()[0];
    assert!(!tree.is_alive);
    assert!(!tree.is_in_danger);
}

#[tokio::test]
async fn reject_an_empty_partial_tree_update() {
    let (orchard, _) = InMemoryOrchardStorage::with_existing_orchard(
        vec![malus_domestica()],
        vec![tree_with_condition(true, false)],
    );
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let response = reqwest::Client::new()
        .patch(format!("{}/trees/1", server.url()))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn serve_default_map_configuration_and_aerial_image() {
    let image = AerialOverlayImage {
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3, 4],
    };
    let orchard = InMemoryOrchardStorage::with_map_configuration(
        MapConfiguration {
            default_center: GeoPoint {
                longitude: 0.5,
                latitude: 0.5,
            },
            aerial_overlays: vec![AerialOverlay {
                id: AerialOverlayId(7),
                name: "Main orchard".into(),
                corners: [
                    GeoPoint {
                        longitude: 0.0,
                        latitude: 1.0,
                    },
                    GeoPoint {
                        longitude: 1.0,
                        latitude: 1.0,
                    },
                    GeoPoint {
                        longitude: 1.0,
                        latitude: 0.0,
                    },
                    GeoPoint {
                        longitude: 0.0,
                        latitude: 0.0,
                    },
                ],
            }],
        },
        vec![(AerialOverlayId(7), image.clone())],
    );
    let server = start_http_server(orchard, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();

    let configuration_response = reqwest::get(format!("{}/map-config", server.url()))
        .await
        .unwrap();
    let image_response = reqwest::get(format!("{}/aerial-overlays/7/image", server.url()))
        .await
        .unwrap();

    assert_eq!(configuration_response.status(), StatusCode::OK);
    assert_eq!(
        configuration_response
            .json::<serde_json::Value>()
            .await
            .unwrap(),
        serde_json::json!({
            "default_center": [0.5, 0.5],
            "aerial_overlays": [{
                "id": 7,
                "name": "Main orchard",
                "coordinates": [
                    [0.0, 1.0],
                    [1.0, 1.0],
                    [1.0, 0.0],
                    [0.0, 0.0]
                ]
            }]
        })
    );
    assert_eq!(image_response.status(), StatusCode::OK);
    assert_eq!(
        image_response.headers().get("content-type").unwrap(),
        "image/png"
    );
    assert_eq!(image_response.bytes().await.unwrap(), image.bytes);
}

fn tree_with_condition(is_alive: bool, is_in_danger: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude: 0.64,
        latitude: 0.68,
        planted_on: Some("2024-02-03".into()),
        row_name: Some("1. Haut haut haut".into()),
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: Some(4.0),
        adult_width_meters: Some(3.0),
    }
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
