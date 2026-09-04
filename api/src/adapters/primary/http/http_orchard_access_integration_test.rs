use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, BotanicalTaxon, GeoPoint,
    IdentificationStatus, MapConfiguration, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree,
};
use reqwest::{Client, StatusCode, header};

const USERNAME: &str = "owner";
const PASSWORD: &str = "correct horse battery staple";

#[tokio::test]
async fn owner_login_opens_only_the_owned_orchard_and_sets_a_secure_cookie() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();

    let login = client
        .post(format!("{}/session", server.url()))
        .json(&serde_json::json!({ "username": USERNAME, "password": PASSWORD }))
        .send()
        .await
        .unwrap();

    assert_eq!(login.status(), StatusCode::OK);
    let set_cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.contains("; HttpOnly; Secure; SameSite=Strict"));
    let session = login.json::<serde_json::Value>().await.unwrap();
    assert_eq!(session["user"]["username"], USERNAME);
    assert_eq!(session["orchards"][0]["id"], 7);
}

#[tokio::test]
async fn anonymous_and_legacy_global_requests_never_expose_or_modify_trees() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();

    for path in ["/trees.geojson", "/map-config"] {
        assert_eq!(
            client
                .get(format!("{}{path}", server.url()))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        client
            .patch(format!("{}/trees/1", server.url()))
            .json(&serde_json::json!({ "is_in_danger": true }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(format!("{}/orchards/7/trees.geojson", server.url()))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn owner_reads_and_modifies_only_the_orchard_in_the_route() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;

    let orchard = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(orchard.status(), StatusCode::OK);
    assert_eq!(
        orchard.json::<serde_json::Value>().await.unwrap()["features"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        client
            .get(format!("{}/orchards/8/trees.geojson", server.url()))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        client
            .patch(format!("{}/orchards/7/trees/1", server.url()))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({ "is_in_danger": true }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn rotating_share_links_are_read_only_and_revoke_the_previous_link() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let first_token = create_share_token(&client, server.url(), &cookie).await;

    assert_eq!(
        client
            .get(format!("{}/orchards/7/trees.geojson", server.url()))
            .header("x-orchard-share-token", &first_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .patch(format!("{}/orchards/7/trees/1", server.url()))
            .header("x-orchard-share-token", &first_token)
            .json(&serde_json::json!({ "is_in_danger": true }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    let second_token = create_share_token(&client, server.url(), &cookie).await;
    assert_ne!(first_token, second_token);
    assert_eq!(
        client
            .get(format!("{}/orchards/7/trees.geojson", server.url()))
            .header("x-orchard-share-token", first_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn shared_access_includes_only_the_orchards_map_and_aerial_image() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let token = create_share_token(&client, server.url(), &cookie).await;

    let map = client
        .get(format!("{}/orchards/7/map-config", server.url()))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(map.status(), StatusCode::OK);
    assert_eq!(
        map.json::<serde_json::Value>().await.unwrap()["aerial_overlays"][0]["id"],
        3
    );
    let image = client
        .get(format!(
            "{}/orchards/7/aerial-overlays/3/image",
            server.url()
        ))
        .header("x-orchard-share-token", token)
        .send()
        .await
        .unwrap();
    assert_eq!(image.status(), StatusCode::OK);
    assert_eq!(image.bytes().await.unwrap().as_ref(), &[1_u8, 2, 3]);
}

async fn login_cookie(client: &Client, server_url: &str) -> String {
    client
        .post(format!("{server_url}/session"))
        .json(&serde_json::json!({ "username": USERNAME, "password": PASSWORD }))
        .send()
        .await
        .unwrap()
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn create_share_token(client: &Client, server_url: &str, cookie: &str) -> String {
    client
        .post(format!("{server_url}/orchards/7/share"))
        .header(header::COOKIE, cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()["share_token"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn owned_storage() -> InMemoryOrchardStorage {
    InMemoryOrchardStorage::with_user_owned_orchard_and_map(
        USERNAME,
        PASSWORD,
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 5.01745,
            latitude: 45.25337,
            reference_region: "Hauterives, Drôme, France".into(),
        },
        vec![apple()],
        vec![tree()],
        MapConfiguration {
            default_center: GeoPoint {
                longitude: 5.01745,
                latitude: 45.25337,
            },
            aerial_overlays: vec![AerialOverlay {
                id: AerialOverlayId(3),
                name: "Aerial".into(),
                corners: [
                    GeoPoint {
                        longitude: 5.0,
                        latitude: 45.3,
                    },
                    GeoPoint {
                        longitude: 5.1,
                        latitude: 45.3,
                    },
                    GeoPoint {
                        longitude: 5.1,
                        latitude: 45.2,
                    },
                    GeoPoint {
                        longitude: 5.0,
                        latitude: 45.2,
                    },
                ],
            }],
        },
        vec![(
            AerialOverlayId(3),
            AerialOverlayImage {
                media_type: "image/png".into(),
                bytes: vec![1, 2, 3],
            },
        )],
    )
}

fn apple() -> PlantIdentity {
    PlantIdentity {
        common_name: "Apple".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Malus".into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
    }
}

fn tree() -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude: 5.02,
        latitude: 45.25,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}
