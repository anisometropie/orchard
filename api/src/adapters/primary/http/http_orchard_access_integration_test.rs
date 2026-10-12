use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, RgbaImage};
use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, BotanicalTaxon, GeoPoint, HarvestDate,
    HarvestPeriod, HarvestRunTarget, HarvestRunTree, HarvestTreeOutcome, HarvestedPart,
    IdentificationStatus, MapConfiguration, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, WateringRunTarget,
};
use orchard_api::hexagon::ports::OrchardStorage;
use reqwest::{Client, StatusCode, header};
use std::io::Cursor;

const USERNAME: &str = "owner";
const PASSWORD: &str = "correct horse battery staple";

#[tokio::test]
async fn only_owner_can_change_watering_exclusion_and_the_map_returns_it() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;

    for token in [&view_token, &watering_token] {
        assert_eq!(
            client
                .patch(format!("{}/orchards/7/trees/1", server.url()))
                .header("x-orchard-share-token", token)
                .json(&serde_json::json!({ "is_excluded_from_watering": true }))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    for excluded in [true, false] {
        assert_eq!(
            client
                .patch(format!("{}/orchards/7/trees/1", server.url()))
                .header(header::COOKIE, &cookie)
                .json(&serde_json::json!({ "is_excluded_from_watering": excluded }))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
        let trees = client
            .get(format!("{}/orchards/7/trees.geojson", server.url()))
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(
            trees["features"][0]["properties"]["is_excluded_from_watering"],
            excluded
        );
        assert_eq!(trees["features"][0]["properties"]["is_alive"], true);
    }
    assert_eq!(
        client
            .patch(format!("{}/orchards/8/trees/1", server.url()))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({ "is_excluded_from_watering": true }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

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
    assert_eq!(
        client
            .put(format!("{}/orchards/7/trees/1/position", server.url()))
            .json(&serde_json::json!({
                "longitude": 12.25,
                "latitude": -34.24
            }))
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
async fn owner_confirms_a_new_tree_position_and_reads_it_back() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;

    let moved = client
        .put(format!("{}/orchards/7/trees/1/position", server.url()))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "longitude": 12.25,
            "latitude": -34.24
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(moved.status(), StatusCode::NO_CONTENT);
    let orchard = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        orchard["features"][0]["geometry"]["coordinates"],
        serde_json::json!([12.25, -34.24])
    );
}

#[tokio::test]
async fn additional_view_links_preserve_existing_links_and_neither_permission_can_edit_trees() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let first_token = create_share_token(&client, server.url(), &cookie).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;

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
    assert_eq!(
        client
            .put(format!("{}/orchards/7/trees/1/position", server.url()))
            .header("x-orchard-share-token", &first_token)
            .json(&serde_json::json!({
                "longitude": 12.25,
                "latitude": -34.24
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .patch(format!("{}/orchards/7/trees/1", server.url()))
            .header("x-orchard-share-token", &watering_token)
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
        StatusCode::OK
    );
    assert_eq!(
        client
            .get(format!("{}/orchards/7/trees.geojson", server.url()))
            .header("x-orchard-share-token", watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn shared_access_includes_the_orchards_map_and_aerial_image() {
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

#[tokio::test]
async fn only_the_owner_adds_photos_and_shared_readers_see_the_latest_webp_variants() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let token = create_share_token(&client, server.url(), &cookie).await;
    let photo_url = format!("{}/orchards/7/trees/1/photos", server.url());
    let large_payload = vec![1; 2 * 1024 * 1024];
    let first_full = webp(&large_payload);
    let first_thumbnail = webp(&[4, 5]);
    let second_full = webp(&[6, 7, 8]);
    let second_thumbnail = webp(&[9, 10]);

    let trees_without_photo = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        trees_without_photo["features"][0]["properties"]["has_photo"],
        false
    );

    assert_eq!(
        client
            .post(&photo_url)
            .header("x-orchard-share-token", &token)
            .json(&photo_request(&first_full, &first_thumbnail))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for (full, thumbnail) in [
        (&first_full, &first_thumbnail),
        (&second_full, &second_thumbnail),
    ] {
        assert_eq!(
            client
                .post(&photo_url)
                .header(header::COOKIE, &cookie)
                .json(&photo_request(full, thumbnail))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::CREATED
        );
    }

    let trees_with_photo = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        trees_with_photo["features"][0]["properties"]["has_photo"],
        true
    );

    let full = client
        .get(format!("{photo_url}/latest"))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(full.status(), StatusCode::OK);
    assert_eq!(full.headers()[header::CONTENT_TYPE], "image/webp");
    assert_eq!(full.bytes().await.unwrap().as_ref(), second_full);

    let thumbnail = client
        .get(format!("{photo_url}/latest/thumbnail"))
        .header("x-orchard-share-token", token)
        .send()
        .await
        .unwrap();
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()[header::CONTENT_TYPE], "image/webp");
    assert_eq!(thumbnail.bytes().await.unwrap().as_ref(), second_thumbnail);

    assert_eq!(
        client
            .get(format!("{photo_url}/latest"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn the_api_resizes_an_original_browser_image_and_converts_both_variants_to_webp() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let photo_url = format!("{}/orchards/7/trees/1/photos", server.url());
    let source = png(1200, 600);

    assert_eq!(
        client
            .post(&photo_url)
            .header(header::COOKIE, &cookie)
            .json(&source_photo_request(&source))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );

    let full = client
        .get(format!("{photo_url}/latest"))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(full.headers()[header::CONTENT_TYPE], "image/webp");
    let full = image::load_from_memory(&full.bytes().await.unwrap()).unwrap();
    assert_eq!((full.width(), full.height()), (1200, 600));

    let thumbnail = client
        .get(format!("{photo_url}/latest/thumbnail"))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(thumbnail.headers()[header::CONTENT_TYPE], "image/webp");
    let thumbnail = image::load_from_memory(&thumbnail.bytes().await.unwrap()).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (480, 240));
}

#[tokio::test]
async fn photo_links_can_list_and_add_but_only_the_owner_deletes_library_photos() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let created_link = client
        .post(format!("{}/orchards/7/share-tokens", server.url()))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "permissions": { "harvest": false, "water": false, "add_photos": true }
        }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let photo_token = created_link["share_token"].as_str().unwrap();
    let photo_url = format!("{}/orchards/7/trees/1/photos", server.url());
    let first_full = webp(&[1, 2]);
    let second_full = webp(&[3, 4]);

    for full in [&first_full, &second_full] {
        assert_eq!(
            client
                .post(&photo_url)
                .header("x-orchard-share-token", photo_token)
                .json(&photo_request(full, &webp(&[9])))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::CREATED
        );
    }

    let library = client
        .get(&photo_url)
        .header("x-orchard-share-token", photo_token)
        .send()
        .await
        .unwrap();
    assert_eq!(library.status(), StatusCode::OK);
    let library = library.json::<serde_json::Value>().await.unwrap();
    let photos = library["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 2);
    assert!(photos[0]["id"].as_u64().unwrap() > photos[1]["id"].as_u64().unwrap());
    assert!(photos[0]["created_at_unix_seconds"].as_i64().unwrap() > 0);
    let newest_id = photos[0]["id"].as_u64().unwrap();

    let newest = client
        .get(format!("{photo_url}/{newest_id}"))
        .header("x-orchard-share-token", photo_token)
        .send()
        .await
        .unwrap();
    assert_eq!(newest.status(), StatusCode::OK);
    assert_eq!(newest.bytes().await.unwrap().as_ref(), second_full);

    assert_eq!(
        client
            .delete(format!("{photo_url}/{newest_id}"))
            .header("x-orchard-share-token", photo_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .delete(format!("{photo_url}/{newest_id}"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    let remaining = client
        .get(&photo_url)
        .header("x-orchard-share-token", photo_token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(remaining["photos"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn an_owner_lists_changes_and_revokes_hashed_share_tokens_with_independent_permissions() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let tokens_url = format!("{}/orchards/7/share-tokens", server.url());

    let created = client
        .post(&tokens_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "permissions": {
                "harvest": false,
                "water": false,
                "add_photos": true
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created = created.json::<serde_json::Value>().await.unwrap();
    let share_id = created["id"].as_u64().unwrap();
    let token = created["share_token"].as_str().unwrap().to_owned();

    let listed = client
        .get(&tokens_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = listed.json::<serde_json::Value>().await.unwrap();
    assert_eq!(listed["shares"].as_array().unwrap().len(), 1);
    assert_eq!(listed["shares"][0]["id"], share_id);
    assert_eq!(listed["shares"][0]["permissions"]["add_photos"], true);
    assert!(listed["shares"][0].get("share_token").is_none());

    let share_access_url = format!("{}/orchards/7/share-access", server.url());
    let access = client
        .get(&share_access_url)
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(access.status(), StatusCode::OK);
    assert_eq!(
        access.json::<serde_json::Value>().await.unwrap()["permissions"],
        serde_json::json!({
            "harvest": false,
            "water": false,
            "add_photos": true
        })
    );

    let photo_url = format!("{}/orchards/7/trees/1/photos", server.url());
    assert_eq!(
        client
            .post(&photo_url)
            .header("x-orchard-share-token", &token)
            .json(&photo_request(&webp(&[1, 2, 3]), &webp(&[4, 5])))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );

    let token_url = format!("{tokens_url}/{share_id}");
    assert_eq!(
        client
            .patch(&token_url)
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({
                "permissions": {
                    "harvest": true,
                    "water": true,
                    "add_photos": false
                }
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .post(&photo_url)
            .header("x-orchard-share-token", &token)
            .json(&photo_request(&webp(&[6]), &webp(&[7])))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(&share_access_url)
            .header("x-orchard-share-token", &token)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["permissions"],
        serde_json::json!({
            "harvest": true,
            "water": true,
            "add_photos": false
        })
    );

    assert_eq!(
        client
            .delete(&token_url)
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .get(&share_access_url)
            .header("x-orchard-share-token", token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn only_the_owner_lists_completed_watering_and_harvest_run_details() {
    let mut storage = owned_storage();
    let watering_run_id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| {
            orchard.mark_watering_tree_watered(watering_run_id, TreeId(1))?;
            orchard.complete_watering_run(watering_run_id)
        })
        .unwrap();
    let harvest_run_id = storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(7),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                HarvestDate::parse_iso("2026-09-18").unwrap(),
                &[HarvestRunTree {
                    tree_id: TreeId(1),
                    harvested_parts: vec![HarvestedPart::Fruit],
                    period: HarvestPeriod {
                        start: HarvestDate::parse_iso("2026-09-01").unwrap(),
                        end: HarvestDate::parse_iso("2026-09-30").unwrap(),
                    },
                    outcome: None,
                }],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| {
            orchard.record_harvest_tree_outcome(
                harvest_run_id,
                TreeId(1),
                HarvestTreeOutcome::HarvestedEverything {
                    harvested_on: HarvestDate::parse_iso("2026-09-18").unwrap(),
                },
            )?;
            orchard.complete_harvest_run(harvest_run_id)
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let history_url = format!("{}/orchards/7/run-history", server.url());

    let response = client
        .get(&history_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let history = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(history["watering_runs"][0]["target_label"], "North");
    assert_eq!(history["watering_runs"][0]["trees"][0]["name"], "Apple");
    assert_eq!(
        history["harvest_runs"][0]["target_label"],
        "All currently available"
    );
    assert_eq!(
        history["harvest_runs"][0]["trees"][0]["outcome"]["kind"],
        "harvested_everything"
    );

    let share_token = create_harvest_watering_share_token(&client, server.url(), &cookie).await;
    assert_eq!(
        client
            .get(history_url)
            .header("x-orchard-share-token", share_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn only_a_watering_link_can_water_and_only_the_owner_can_order_a_row() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;

    let shared_order = client
        .put(format!("{}/orchards/7/row-order", server.url()))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({
            "row_name": "North",
            "order": { "method": "east_to_west" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(shared_order.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        client
            .post(format!("{}/orchards/7/watering-runs", server.url()))
            .header("x-orchard-share-token", &view_token)
            .json(&serde_json::json!({ "row_name": "North" }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(format!("{}/orchards/7/watering-runs", server.url()))
            .json(&serde_json::json!({ "row_name": "North" }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(format!("{}/orchards/8/watering-runs", server.url()))
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({ "row_name": "North" }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );

    let unordered = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({ "row_name": "North" }))
        .send()
        .await
        .unwrap();
    assert_eq!(unordered.status(), StatusCode::CONFLICT);
    assert_eq!(
        unordered.json::<serde_json::Value>().await.unwrap()["code"],
        "watering_row_not_ordered"
    );

    let ordered = client
        .put(format!("{}/orchards/7/row-order", server.url()))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "row_name": "North",
            "order": { "method": "east_to_west" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(ordered.status(), StatusCode::OK);
    assert_eq!(
        ordered.json::<serde_json::Value>().await.unwrap()["tree_ids"],
        serde_json::json!([1])
    );

    let started = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({ "row_name": "North" }))
        .send()
        .await
        .unwrap();
    assert_eq!(started.status(), StatusCode::OK);
    let progress = started.json::<serde_json::Value>().await.unwrap();
    assert_eq!(progress["next_tree"]["id"], 1);

    let restored = client
        .get(format!("{}/orchards/7/watering-run", server.url()))
        .header("x-orchard-share-token", &watering_token)
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        restored.json::<serde_json::Value>().await.unwrap()["next_tree"]["id"],
        1
    );

    let completed = client
        .post(format!(
            "{}/orchards/7/watering-runs/1/watered",
            server.url()
        ))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({ "tree_id": 1 }))
        .send()
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    let completed = completed.json::<serde_json::Value>().await.unwrap();
    assert_eq!(completed["watered_tree_count"], 1);
    assert!(completed["next_tree"].is_null());

    assert_eq!(
        client
            .get(format!("{}/orchards/7/watering-run", server.url()))
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
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
    assert_eq!(
        client
            .post(format!("{}/orchards/7/watering-runs", server.url()))
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({
                "target": "danger",
                "water_source": { "longitude": -73.4909, "latitude": 12.2839 },
                "carry_capacity": 0
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let danger_run = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", watering_token)
        .json(&serde_json::json!({
            "target": "danger",
            "water_source": { "longitude": -73.4909, "latitude": 12.2839 },
            "carry_capacity": 3
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(danger_run.status(), StatusCode::OK);
    let danger_progress = danger_run.json::<serde_json::Value>().await.unwrap();
    assert_eq!(danger_progress["target"], "danger");
    assert_eq!(danger_progress["target_label"], "Danger trees");
    assert!(danger_progress["row_name"].is_null());
    assert_eq!(danger_progress["water_source"]["longitude"], -73.4909);
    assert_eq!(danger_progress["water_source"]["latitude"], 12.2839);
    assert_eq!(danger_progress["carry_capacity"], 3);
    assert_eq!(danger_progress["route"][0]["id"], 1);
    assert_eq!(danger_progress["next_tree"]["id"], 1);
}

#[tokio::test]
async fn watering_links_pause_resume_and_load_only_their_orchards_saved_progress() {
    let mut storage = owned_storage();
    for orchard_id in [OrchardId(7), OrchardId(8)] {
        storage
            .transaction(|orchard| {
                orchard.create_watering_run(
                    orchard_id,
                    &WateringRunTarget::Row("North".into()),
                    None,
                    None,
                    &[TreeId(1)],
                )
            })
            .unwrap();
    }
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let token = create_watering_share_token(&client, server.url(), &cookie).await;
    let viewer = create_share_token(&client, server.url(), &cookie).await;
    for (method, path) in [
        (reqwest::Method::GET, "watering-runs"),
        (reqwest::Method::GET, "watering-runs/1"),
        (reqwest::Method::POST, "watering-runs/1/pause"),
        (reqwest::Method::POST, "watering-runs/1/resume"),
    ] {
        assert_eq!(
            client
                .request(
                    method.clone(),
                    format!("{}/orchards/7/{path}", server.url())
                )
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .request(
                    method.clone(),
                    format!("{}/orchards/7/{path}", server.url())
                )
                .header("x-orchard-share-token", &viewer)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .request(method, format!("{}/orchards/8/{path}", server.url()))
                .header("x-orchard-share-token", &token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    for suffix in ["", "/pause", "/resume"] {
        let method = if suffix.is_empty() {
            reqwest::Method::GET
        } else {
            reqwest::Method::POST
        };
        assert_eq!(
            client
                .request(
                    method,
                    format!("{}/orchards/7/watering-runs/2{suffix}", server.url())
                )
                .header("x-orchard-share-token", &token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let paused = client
        .post(format!("{}/orchards/7/watering-runs/1/pause", server.url()))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(paused.status(), StatusCode::OK);
    let paused = paused.json::<serde_json::Value>().await.unwrap();
    assert_eq!(paused["paused"], true);
    assert_eq!(
        client
            .get(format!("{}/orchards/7/watering-run", server.url()))
            .header("x-orchard-share-token", &token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let runs = client
        .get(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(runs["runs"], serde_json::json!([paused]));
    assert_eq!(
        client
            .post(format!(
                "{}/orchards/7/watering-runs/1/watered",
                server.url()
            ))
            .header("x-orchard-share-token", &token)
            .json(&serde_json::json!({"tree_id": 1}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let resumed = client
        .post(format!(
            "{}/orchards/7/watering-runs/1/resume",
            server.url()
        ))
        .header("x-orchard-share-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(resumed.status(), StatusCode::OK);
    let resumed = resumed.json::<serde_json::Value>().await.unwrap();
    assert_eq!(resumed["paused"], false);
    assert_eq!(resumed["run_id"], paused["run_id"]);
    assert_eq!(resumed["next_tree"], paused["next_tree"]);
    assert_eq!(
        client
            .get(format!("{}/orchards/7/watering-runs/1", server.url()))
            .header("x-orchard-share-token", &token)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap(),
        resumed
    );
}

#[tokio::test]
async fn watering_links_choose_independent_rows_and_can_join_each_others_run() {
    let mut south_tree = tree();
    south_tree.row_name = Some("South".into());
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        USERNAME,
        PASSWORD,
        Orchard {
            id: OrchardId(7),
            name: "Orchard".into(),
            longitude: 0.5,
            latitude: 0.5,
            reference_region: "France".into(),
        },
        vec![apple()],
        vec![tree(), south_tree],
    );
    storage
        .transaction(|orchard| {
            orchard.replace_row_order(OrchardId(7), "North", &[TreeId(1)])?;
            orchard.replace_row_order(OrchardId(7), "South", &[TreeId(2)])
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let first = create_watering_share_token(&client, server.url(), &cookie).await;
    let second = create_watering_share_token(&client, server.url(), &cookie).await;
    let mut runs = Vec::new();
    for (row, token) in [("North", &first), ("South", &second)] {
        let response = client
            .post(format!("{}/orchards/7/watering-runs", server.url()))
            .header("x-orchard-share-token", token)
            .json(&serde_json::json!({"row_name": row}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        runs.push(response.json::<serde_json::Value>().await.unwrap());
    }
    assert_ne!(runs[0]["run_id"], runs[1]["run_id"]);
    let north_id = runs[0]["run_id"].as_u64().unwrap();
    let south_id = runs[1]["run_id"].as_u64().unwrap();
    let joined = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &second)
        .json(&serde_json::json!({"row_name": "North"}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(joined, runs[0]);
    let listed = client
        .get(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &second)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(listed["runs"], serde_json::json!(runs));
    let finished = client
        .post(format!(
            "{}/orchards/7/watering-runs/{north_id}/watered",
            server.url()
        ))
        .header("x-orchard-share-token", &second)
        .json(&serde_json::json!({"tree_id": 1}))
        .send()
        .await
        .unwrap();
    assert_eq!(finished.status(), StatusCode::OK);
    let south = client
        .get(format!(
            "{}/orchards/7/watering-runs/{south_id}",
            server.url()
        ))
        .header("x-orchard-share-token", &first)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(south, runs[1]);
}

#[tokio::test]
async fn watering_links_mark_only_the_current_tree_dead_and_keep_skips_out_of_watered_history() {
    let mut danger_tree = tree();
    danger_tree.is_in_danger = true;
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        USERNAME,
        PASSWORD,
        Orchard {
            id: OrchardId(7),
            name: "Orchard".into(),
            longitude: 0.5,
            latitude: 0.5,
            reference_region: "France".into(),
        },
        vec![apple()],
        vec![danger_tree.clone(), danger_tree.clone(), danger_tree],
    );
    let run_id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(1), TreeId(2), TreeId(3)],
            )
        })
        .unwrap();
    let foreign_run_id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(8),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(1)],
            )
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;
    let run_url = format!("{}/orchards/7/watering-runs/{}", server.url(), run_id.0);
    let dead_url = format!("{run_url}/dead");
    let first_tree = serde_json::json!({"tree_id": 1});

    assert_eq!(
        client
            .post(&dead_url)
            .json(&first_tree)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(&dead_url)
            .header("x-orchard-share-token", &view_token)
            .json(&first_tree)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for url in [
        format!(
            "{}/orchards/8/watering-runs/{}/dead",
            server.url(),
            run_id.0
        ),
        format!(
            "{}/orchards/7/watering-runs/{}/dead",
            server.url(),
            foreign_run_id.0
        ),
    ] {
        assert_eq!(
            client
                .post(url)
                .header("x-orchard-share-token", &watering_token)
                .json(&first_tree)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        client
            .patch(format!("{}/orchards/7/trees/1", server.url()))
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({"is_alive": false}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&dead_url)
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({"tree_id": 3}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .post(format!("{run_url}/pause"))
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .post(&dead_url)
            .header("x-orchard-share-token", &watering_token)
            .json(&first_tree)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert!(
        observer
            .trees()
            .iter()
            .all(|tree| tree.is_alive && tree.is_in_danger)
    );
    assert_eq!(
        client
            .post(format!("{run_url}/resume"))
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let skipped = client
        .post(&dead_url)
        .header("x-orchard-share-token", &watering_token)
        .json(&first_tree)
        .send()
        .await
        .unwrap();
    assert_eq!(skipped.status(), StatusCode::OK);
    let skipped = skipped.json::<serde_json::Value>().await.unwrap();
    assert_eq!(skipped["next_tree"]["id"], 2);
    assert_eq!(skipped["watered_tree_count"], 0);
    assert_eq!(skipped["skipped_tree_count"], 1);
    assert_eq!(skipped["handled_tree_count"], 1);
    assert_eq!(skipped["total_tree_count"], 3);
    assert_eq!(skipped["watered_tree_ids"], serde_json::json!([]));
    assert_eq!(skipped["skipped_tree_ids"], serde_json::json!([1]));
    assert_eq!(
        client
            .post(&dead_url)
            .header("x-orchard-share-token", &watering_token)
            .json(&first_tree)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );

    let watered = client
        .post(format!("{run_url}/watered"))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({"tree_id": 2}))
        .send()
        .await
        .unwrap();
    assert_eq!(watered.status(), StatusCode::OK);
    let watered = watered.json::<serde_json::Value>().await.unwrap();
    assert_eq!(watered["next_tree"]["id"], 3);
    assert_eq!(watered["watered_tree_count"], 1);
    assert_eq!(watered["handled_tree_count"], 2);

    let completed = client
        .post(&dead_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({"tree_id": 3}))
        .send()
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    let completed = completed.json::<serde_json::Value>().await.unwrap();
    assert!(completed["next_tree"].is_null());
    assert_eq!(completed["watered_tree_count"], 1);
    assert_eq!(completed["skipped_tree_count"], 2);
    assert_eq!(completed["handled_tree_count"], 3);
    assert_eq!(completed["watered_tree_ids"], serde_json::json!([2]));
    assert_eq!(completed["skipped_tree_ids"], serde_json::json!([1, 3]));
    assert_eq!(
        client
            .post(&dead_url)
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({"tree_id": 3}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );

    let trees = observer.trees();
    assert!(!trees[0].is_alive && !trees[0].is_in_danger);
    assert!(trees[1].is_alive && trees[1].is_in_danger);
    assert!(!trees[2].is_alive && !trees[2].is_in_danger);
    let history = client
        .get(format!("{}/orchards/7/run-history", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let history = &history["watering_runs"][0];
    assert_eq!(history["watered_tree_count"], 1);
    assert_eq!(history["skipped_tree_count"], 2);
    assert_eq!(history["total_tree_count"], 3);
    for index in [0, 2] {
        assert!(history["trees"][index]["watered_at_unix_seconds"].is_null());
        assert!(history["trees"][index]["skipped_at_unix_seconds"].is_number());
    }
    assert!(history["trees"][1]["watered_at_unix_seconds"].is_number());
    assert!(history["trees"][1]["skipped_at_unix_seconds"].is_null());
}

#[tokio::test]
async fn a_combined_link_can_water_and_run_the_complete_harvest_workflow_but_cannot_edit() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;
    let worker_token = create_harvest_watering_share_token(&client, server.url(), &cookie).await;

    assert_eq!(
        client
            .patch(format!("{}/orchards/7/trees/1", server.url()))
            .header("x-orchard-share-token", &worker_token)
            .json(&serde_json::json!({ "is_in_danger": true }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let schedule_url = format!(
        "{}/orchards/7/plant-identities/1/harvest-windows",
        server.url()
    );
    let schedule_request = serde_json::json!({
        "reference_region": "Example Region, France",
        "windows": [{
            "start": { "month": 9, "day": 1 },
            "end": { "month": 9, "day": 20 },
            "harvested_part": "fruit"
        }]
    });
    assert_eq!(
        client
            .put(&schedule_url)
            .header("x-orchard-share-token", &worker_token)
            .json(&schedule_request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .put(format!("{}/orchards/7/row-order", server.url()))
            .header("x-orchard-share-token", &worker_token)
            .json(&serde_json::json!({
                "row_name": "North",
                "order": { "method": "east_to_west" }
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(format!("{}/orchards/7/share", server.url()))
            .header("x-orchard-share-token", &worker_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .put(&schedule_url)
            .header(header::COOKIE, &cookie)
            .json(&schedule_request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        client
            .put(format!("{}/orchards/7/row-order", server.url()))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({
                "row_name": "North",
                "order": { "method": "east_to_west" }
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let watering = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &worker_token)
        .json(&serde_json::json!({ "row_name": "North" }))
        .send()
        .await
        .unwrap();
    assert_eq!(watering.status(), StatusCode::OK);
    let watering_run_id = watering.json::<serde_json::Value>().await.unwrap()["run_id"]
        .as_u64()
        .unwrap();
    assert_eq!(
        client
            .delete(format!(
                "{}/orchards/7/watering-runs/{watering_run_id}",
                server.url()
            ))
            .header("x-orchard-share-token", &worker_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    let candidates_url = format!("{}/orchards/7/harvest-candidates", server.url());
    assert_eq!(
        client
            .get(&candidates_url)
            .header("x-orchard-share-token", &watering_token)
            .query(&[("on_date", "2026-09-17"), ("harvested_parts", "fruit")])
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(&candidates_url)
            .header("x-orchard-share-token", &worker_token)
            .query(&[("on_date", "2026-09-17"), ("harvested_parts", "fruit")])
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let started = client
        .post(format!("{}/orchards/7/harvest-runs", server.url()))
        .header("x-orchard-share-token", &worker_token)
        .json(&serde_json::json!({
            "target": "all",
            "plant_identity_id": null,
            "harvested_parts": ["fruit"],
            "on_date": "2026-09-17"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(started.status(), StatusCode::OK);
    let run_id = started.json::<serde_json::Value>().await.unwrap()["run_id"]
        .as_u64()
        .unwrap();
    let defer_url = format!("{}/orchards/7/harvest-runs/{run_id}/deferred", server.url());
    let proposal = client
        .post(&defer_url)
        .header("x-orchard-share-token", &worker_token)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-17"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(proposal.status(), StatusCode::CONFLICT);
    assert_eq!(
        proposal.json::<serde_json::Value>().await.unwrap()["code"],
        "harvest_window_extension_required"
    );
    assert_eq!(
        client
            .post(&defer_url)
            .header("x-orchard-share-token", &worker_token)
            .json(&serde_json::json!({
                "tree_id": 1,
                "action_date": "2026-09-17",
                "extend_window": true
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .post(format!(
                "{}/orchards/7/harvest-runs/{run_id}/previous",
                server.url()
            ))
            .header("x-orchard-share-token", &worker_token)
            .json(&serde_json::json!({ "action_date": "2026-09-17" }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .delete(format!("{}/orchards/7/harvest-runs/{run_id}", server.url()))
            .header("x-orchard-share-token", worker_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn a_watering_link_can_cancel_an_active_run_but_a_view_link_cannot() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;
    client
        .put(format!("{}/orchards/7/row-order", server.url()))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "row_name": "North",
            "order": { "method": "east_to_west" }
        }))
        .send()
        .await
        .unwrap();
    let started = client
        .post(format!("{}/orchards/7/watering-runs", server.url()))
        .header("x-orchard-share-token", &watering_token)
        .json(&serde_json::json!({ "row_name": "North" }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let run_id = started["run_id"].as_u64().unwrap();
    let cancel_url = format!("{}/orchards/7/watering-runs/{run_id}", server.url());

    assert_eq!(
        client
            .delete(&cancel_url)
            .header("x-orchard-share-token", view_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .delete(&cancel_url)
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .get(format!("{}/orchards/7/watering-run", server.url()))
            .header("x-orchard-share-token", watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn editing_a_harvest_window_preserves_the_unchanged_windows_reference_in_geojson() {
    use orchard_api::hexagon::models::{
        AnnualDate, AnnualHarvestWindow, HarvestDataOrigin, HarvestScheduleOwner,
    };
    let mut storage = owned_storage();
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
                [6, 9]
                    .map(|month| AnnualHarvestWindow {
                        start: AnnualDate { month, day: 1 },
                        end: AnnualDate { month, day: 20 },
                        reference_region: Some("Sapporo, Japan".into()),
                        harvested_part: HarvestedPart::Fruit,
                        data_origin: HarvestDataOrigin::ExternalReference,
                        source_url: Some(format!("https://example.com/harvest/{month}")),
                        validated: false,
                    })
                    .to_vec(),
            )
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let response = client.put(format!("{}/orchards/7/plant-identities/1/harvest-windows", server.url()))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "reference_region": "Sapporo, Japan",
            "windows": [
                { "start": {"month": 6, "day": 1}, "end": {"month": 6, "day": 25}, "harvested_part": "fruit" },
                { "start": {"month": 9, "day": 1}, "end": {"month": 9, "day": 20}, "harvested_part": "fruit" }
            ]
        })).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let map: serde_json::Value = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let windows = map["features"][0]["properties"]["harvest_windows"]
        .as_array()
        .unwrap();
    assert_eq!(windows[0]["data_origin"], "field_observation");
    assert!(windows[0]["source_url"].is_null());
    assert_eq!(windows[1]["data_origin"], "external_reference");
    assert_eq!(windows[1]["source_url"], "https://example.com/harvest/9");
}

#[tokio::test]
async fn an_owner_can_run_a_resumable_harvest_tour_and_extend_a_shared_window() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let watering_token = create_watering_share_token(&client, server.url(), &cookie).await;
    let harvest_runs_url = format!("{}/orchards/7/harvest-runs", server.url());

    let schedule = client
        .put(format!(
            "{}/orchards/7/plant-identities/1/harvest-windows",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "reference_region": "Example Region, France",
            "windows": [{
                "start": { "month": 9, "day": 1 },
                "end": { "month": 9, "day": 30 },
                "harvested_part": "fruit"
            }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(schedule.status(), StatusCode::NO_CONTENT);

    let candidates_url = format!("{}/orchards/7/harvest-candidates", server.url());
    assert_eq!(
        client
            .get(&candidates_url)
            .header("x-orchard-share-token", &watering_token)
            .query(&[("on_date", "2026-09-17"), ("harvested_parts", "fruit")])
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let candidates = client
        .get(&candidates_url)
        .header(header::COOKIE, &cookie)
        .query(&[("on_date", "2026-09-17"), ("harvested_parts", "fruit")])
        .send()
        .await
        .unwrap();
    assert_eq!(candidates.status(), StatusCode::OK);
    assert_eq!(
        candidates.json::<serde_json::Value>().await.unwrap()["candidates"],
        serde_json::json!([{"tree_id": 1, "plant_identity_id": 1}])
    );

    assert_eq!(
        client
            .post(&harvest_runs_url)
            .header("x-orchard-share-token", &watering_token)
            .json(&serde_json::json!({
                "target": "all",
                "plant_identity_id": null,
                "harvested_parts": ["fruit"],
                "on_date": "2026-09-17"
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    let started = client
        .post(&harvest_runs_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "target": "all",
            "plant_identity_id": null,
            "harvested_parts": ["fruit"],
            "on_date": "2026-09-17"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(started.status(), StatusCode::OK);
    let progress = started.json::<serde_json::Value>().await.unwrap();
    assert_eq!(progress["next_tree"]["id"], 1);
    assert_eq!(progress["handled_tree_count"], 0);
    let run_id = progress["run_id"].as_u64().unwrap();

    let outside_period = client
        .post(format!(
            "{}/orchards/7/harvest-runs/{run_id}/harvested",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-16"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(outside_period.status(), StatusCode::CONFLICT);
    assert_eq!(
        outside_period.json::<serde_json::Value>().await.unwrap()["code"],
        "harvest_action_date_outside_run_period"
    );

    let restored = client
        .get(format!(
            "{}/orchards/7/harvest-run?on_date=2026-09-17",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        restored.json::<serde_json::Value>().await.unwrap()["run_id"],
        run_id
    );

    let defer_url = format!("{}/orchards/7/harvest-runs/{run_id}/deferred", server.url());
    let changed_schedule = client
        .put(format!(
            "{}/orchards/7/plant-identities/1/harvest-windows",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "reference_region": "Example Region, France",
            "windows": [{
                "start": { "month": 9, "day": 2 },
                "end": { "month": 9, "day": 30 },
                "harvested_part": "fruit"
            }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(changed_schedule.status(), StatusCode::NO_CONTENT);
    let changed_harvest_window = client
        .post(format!(
            "{}/orchards/7/harvest-runs/{run_id}/harvested",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-25"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(changed_harvest_window.status(), StatusCode::CONFLICT);
    assert_eq!(
        changed_harvest_window
            .json::<serde_json::Value>()
            .await
            .unwrap()["code"],
        "harvest_window_changed"
    );
    let changed_window = client
        .post(&defer_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-25",
            "extend_window": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(changed_window.status(), StatusCode::CONFLICT);
    assert_eq!(
        changed_window.json::<serde_json::Value>().await.unwrap()["code"],
        "harvest_window_changed"
    );
    let restored_schedule = client
        .put(format!(
            "{}/orchards/7/plant-identities/1/harvest-windows",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "reference_region": "Example Region, France",
            "windows": [{
                "start": { "month": 9, "day": 1 },
                "end": { "month": 9, "day": 30 },
                "harvested_part": "fruit"
            }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(restored_schedule.status(), StatusCode::NO_CONTENT);

    let proposal = client
        .post(&defer_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-25"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(proposal.status(), StatusCode::CONFLICT);
    let proposal = proposal.json::<serde_json::Value>().await.unwrap();
    assert_eq!(proposal["code"], "harvest_window_extension_required");
    assert_eq!(proposal["current_end"], "2026-09-30");
    assert_eq!(proposal["proposed_end"], "2026-10-07");
    assert_eq!(proposal["retry_on"], "2026-10-02");

    let deferred = client
        .post(&defer_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-25",
            "extend_window": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(deferred.status(), StatusCode::OK);
    let deferred = deferred.json::<serde_json::Value>().await.unwrap();
    assert!(deferred["next_tree"].is_null());
    assert_eq!(deferred["done_for_window_tree_count"], 1);
    assert_eq!(deferred["deferred_tree_count"], 0);

    let previous = client
        .post(format!(
            "{}/orchards/7/harvest-runs/{run_id}/previous",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "action_date": "2026-09-25" }))
        .send()
        .await
        .unwrap();
    assert_eq!(previous.status(), StatusCode::OK);
    let previous = previous.json::<serde_json::Value>().await.unwrap();
    assert_eq!(previous["next_tree"]["id"], 1);
    assert_eq!(previous["next_tree"]["window_end"], "2026-09-30");
    assert_eq!(previous["done_for_window_tree_count"], 0);

    let deferred = client
        .post(&defer_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-09-25",
            "extend_window": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(deferred.status(), StatusCode::OK);
    let deferred = deferred.json::<serde_json::Value>().await.unwrap();
    assert!(deferred["next_tree"].is_null());
    assert_eq!(deferred["done_for_window_tree_count"], 0);
    assert_eq!(deferred["deferred_tree_count"], 1);

    let before_retry = client
        .post(&harvest_runs_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "target": "all",
            "plant_identity_id": null,
            "harvested_parts": ["fruit"],
            "on_date": "2026-10-01"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(before_retry.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        client
            .get(&candidates_url)
            .header(header::COOKIE, &cookie)
            .query(&[("on_date", "2026-10-01"), ("harvested_parts", "fruit")])
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["candidates"],
        serde_json::json!([])
    );

    let retry_candidates = client
        .get(&candidates_url)
        .header(header::COOKIE, &cookie)
        .query(&[("on_date", "2026-10-02"), ("harvested_parts", "fruit")])
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        retry_candidates["candidates"],
        serde_json::json!([{"tree_id": 1, "plant_identity_id": 1}])
    );

    let retried = client
        .post(&harvest_runs_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "target": "all",
            "plant_identity_id": null,
            "harvested_parts": ["fruit"],
            "on_date": "2026-10-02"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(retried.status(), StatusCode::OK);
    let retried = retried.json::<serde_json::Value>().await.unwrap();
    let retry_run_id = retried["run_id"].as_u64().unwrap();
    let harvested = client
        .post(format!(
            "{}/orchards/7/harvest-runs/{retry_run_id}/harvested",
            server.url()
        ))
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "tree_id": 1,
            "action_date": "2026-10-02"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(harvested.status(), StatusCode::OK);
    assert!(harvested.json::<serde_json::Value>().await.unwrap()["next_tree"].is_null());
    assert_eq!(
        client
            .post(&harvest_runs_url)
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({
                "target": "all",
                "plant_identity_id": null,
                "harvested_parts": ["fruit"],
                "on_date": "2026-10-03"
            }))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .get(&candidates_url)
            .header(header::COOKIE, &cookie)
            .query(&[("on_date", "2026-10-03"), ("harvested_parts", "fruit")])
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["candidates"],
        serde_json::json!([])
    );

    let cancellable = client
        .post(&harvest_runs_url)
        .header(header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "target": "all",
            "plant_identity_id": null,
            "harvested_parts": ["fruit"],
            "on_date": "2027-09-17"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(cancellable.status(), StatusCode::OK);
    let cancellable_run_id = cancellable.json::<serde_json::Value>().await.unwrap()["run_id"]
        .as_u64()
        .unwrap();
    let cancel_url = format!(
        "{}/orchards/7/harvest-runs/{cancellable_run_id}",
        server.url()
    );
    assert_eq!(
        client
            .delete(&cancel_url)
            .header("x-orchard-share-token", &watering_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .delete(&cancel_url)
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .get(format!(
                "{}/orchards/7/harvest-run?on_date=2027-09-17",
                server.url()
            ))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
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

async fn create_watering_share_token(client: &Client, server_url: &str, cookie: &str) -> String {
    client
        .post(format!("{server_url}/orchards/7/share/watering"))
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

async fn create_harvest_watering_share_token(
    client: &Client,
    server_url: &str,
    cookie: &str,
) -> String {
    client
        .post(format!("{server_url}/orchards/7/share/harvest-watering"))
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
            longitude: 0.5,
            latitude: 0.5,
            reference_region: "Exampletown, Example Region, France".into(),
        },
        vec![apple()],
        vec![tree()],
        MapConfiguration {
            default_center: GeoPoint {
                longitude: 0.5,
                latitude: 0.5,
            },
            aerial_overlays: vec![AerialOverlay {
                id: AerialOverlayId(3),
                name: "Aerial".into(),
                corners: [
                    GeoPoint {
                        longitude: -73.5,
                        latitude: 12.589,
                    },
                    GeoPoint {
                        longitude: -73.409,
                        latitude: 12.589,
                    },
                    GeoPoint {
                        longitude: -73.409,
                        latitude: 12.476,
                    },
                    GeoPoint {
                        longitude: -73.5,
                        latitude: 12.476,
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
        longitude: -73.4818,
        latitude: 12.5325,
        planted_on: None,
        row_name: Some("North".into()),
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        is_excluded_from_watering: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}

fn photo_request(full: &[u8], thumbnail: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "full_webp_base64": STANDARD.encode(full),
        "thumbnail_webp_base64": STANDARD.encode(thumbnail),
    })
}

fn source_photo_request(image: &[u8]) -> serde_json::Value {
    serde_json::json!({ "image_base64": STANDARD.encode(image) })
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(
            width,
            height,
            vec![127; usize::try_from(width * height * 4).unwrap()],
        )
        .unwrap(),
    );
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

fn webp(payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&u32::try_from(payload.len() + 4).unwrap().to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(payload);
    bytes
}

#[tokio::test]
async fn harvesters_collect_observations_but_only_the_owner_can_review_and_change_windows() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let harvest_token = create_harvest_watering_share_token(&client, server.url(), &cookie).await;
    let windows_url = format!(
        "{}/orchards/7/plant-identities/1/harvest-windows",
        server.url()
    );
    let windows = serde_json::json!({ "reference_region": "France", "windows": [{
        "start": { "month": 9, "day": 1 }, "end": { "month": 9, "day": 30 }, "harvested_part": "fruit"
    }, {
        "start": { "month": 6, "day": 1 }, "end": { "month": 6, "day": 30 }, "harvested_part": "fruit"
    }] });
    assert_eq!(
        client
            .put(&windows_url)
            .header(header::COOKIE, &cookie)
            .json(&windows)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let start_url = format!("{}/orchards/7/harvest-runs", server.url());
    let run = client.post(&start_url).header("x-orchard-share-token", &harvest_token)
        .json(&serde_json::json!({ "target": "all", "harvested_parts": ["fruit"], "on_date": "2026-09-28" }))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let record_url = format!(
        "{}/orchards/7/harvest-runs/{}/observations",
        server.url(),
        run["run_id"]
    );
    let event =
        serde_json::json!({ "tree_id": 1, "action_date": "2026-09-28", "kind": "too_soon" });
    assert_eq!(
        client
            .post(&record_url)
            .header("x-orchard-share-token", &view_token)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let recorded = client
        .post(&record_url)
        .header("x-orchard-share-token", &harvest_token)
        .json(&event)
        .send()
        .await
        .unwrap();
    assert_eq!(recorded.status(), StatusCode::OK);
    assert!(recorded.json::<serde_json::Value>().await.unwrap()["next_tree"].is_null());
    let review_url = format!("{}/orchards/7/harvest-observations/species/1", server.url());
    let points = client
        .get(&review_url)
        .header("x-orchard-share-token", &view_token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(points[0]["kind"], "too_soon");
    assert_eq!(points[0]["recorded_on"], "2026-09-28");
    assert_eq!(points[0]["window_end"], "2026-09-30");
    assert_eq!(points[0]["reviewed"], false);
    let selected_window = windows["windows"][0].clone();
    let mut changed_windows = windows;
    changed_windows["windows"] = serde_json::json!([selected_window]);
    changed_windows["reference_region"] = serde_json::json!("Paris");
    changed_windows["windows"][0]["start"]["day"] = serde_json::json!(15);
    let review = serde_json::json!({ "observation_ids": [points[0]["id"]], "window": selected_window, "window_change": changed_windows });
    assert_eq!(
        client
            .post(&review_url)
            .header("x-orchard-share-token", &harvest_token)
            .json(&review)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut wrong_window = review.clone();
    wrong_window["window"]["start"]["day"] = serde_json::json!(2);
    assert_eq!(
        client
            .post(&review_url)
            .header(header::COOKIE, &cookie)
            .json(&wrong_window)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .post(&review_url)
            .header(header::COOKIE, &cookie)
            .json(&review)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let reviewed = client
        .get(&review_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(reviewed[0]["reviewed"], true);
    assert_eq!(
        client
            .post(&review_url)
            .header(header::COOKIE, &cookie)
            .json(&review)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let map = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let saved_windows = map["features"][0]["properties"]["harvest_windows"]
        .as_array()
        .unwrap();
    assert_eq!(saved_windows.len(), 2);
    let edited = saved_windows
        .iter()
        .find(|window| window["start"] == "09-15")
        .unwrap();
    assert_eq!(edited["reference_region"], "Paris");
    let other = saved_windows
        .iter()
        .find(|window| window["start"] == "06-01")
        .unwrap();
    assert_eq!(other["end"], "06-30");
    assert_eq!(other["reference_region"], "France");
    let history = client
        .get(format!("{}/orchards/7/run-history", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        history["harvest_runs"][0]["trees"][0]["outcome"]["kind"],
        "too_soon"
    );
}

#[tokio::test]
async fn harvest_visit_facts_are_validated_and_production_only_visits_are_not_window_evidence() {
    for (kind, fruit_state) in [
        ("empty_on_arrival", None),
        ("harvested_all", None),
        ("all_wasted", None),
        (
            "fruit_remaining",
            Some(serde_json::json!({"ready": false, "unripe": true, "wasted": true})),
        ),
    ] {
        let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let client = Client::new();
        let cookie = login_cookie(&client, server.url()).await;
        let windows_url = format!(
            "{}/orchards/7/plant-identities/1/harvest-windows",
            server.url()
        );
        let windows = serde_json::json!({"reference_region":"France", "windows":[{
            "start":{"month":9,"day":1},"end":{"month":9,"day":30},"harvested_part":"fruit"
        }]});
        assert_eq!(
            client
                .put(&windows_url)
                .header(header::COOKIE, &cookie)
                .json(&windows)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
        let run = client.post(format!("{}/orchards/7/harvest-runs", server.url())).header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"target":"all","harvested_parts":["fruit"],"on_date":"2026-09-28"}))
            .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
        let record_url = format!(
            "{}/orchards/7/harvest-runs/{}/observations",
            server.url(),
            run["run_id"]
        );
        let invalid = serde_json::json!({"tree_id":1,"action_date":"2026-09-28","kind":"fruit_remaining","fruit_state":{"ready":false,"unripe":false,"wasted":true}});
        assert_eq!(
            client
                .post(&record_url)
                .header(header::COOKIE, &cookie)
                .json(&invalid)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
        let event = serde_json::json!({"tree_id":1,"action_date":"2026-09-28","kind":kind,"fruit_state":fruit_state});
        assert_eq!(
            client
                .post(&record_url)
                .header(header::COOKIE, &cookie)
                .json(&event)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let all_url = format!("{}/orchards/7/harvest-observations", server.url());
        assert_eq!(
            client.get(&all_url).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let all = client
            .get(&all_url)
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(all.as_array().unwrap().len(), 1);
        assert_eq!(all[0]["kind"], kind);
        assert_eq!(all[0]["owner_kind"], "species");
        assert_eq!(all[0]["owner_id"], 1);
        assert_eq!(all[0]["fruit_state"]["ready"], kind == "harvested_all");
        let scope_url = format!("{all_url}/species/1");
        let scope = client
            .get(&scope_url)
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(
            scope.as_array().unwrap().len(),
            usize::from(kind != "empty_on_arrival")
        );
        if kind == "empty_on_arrival" {
            assert_eq!(all[0]["evidence"], serde_json::json!([]));
            let review = serde_json::json!({"observation_ids":[all[0]["id"]],"window_change":null});
            assert_eq!(
                client
                    .post(&scope_url)
                    .header(header::COOKIE, &cookie)
                    .json(&review)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::CONFLICT
            );
        } else if kind == "fruit_remaining" {
            assert_eq!(
                all[0]["evidence"],
                serde_json::json!(["earlier_fruit_missed", "mixed_ripeness"])
            );
        }
        let undone = client
            .post(format!(
                "{}/orchards/7/harvest-runs/{}/previous",
                server.url(),
                run["run_id"]
            ))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"action_date":"2026-09-28"}))
            .send()
            .await
            .unwrap();
        assert_eq!(undone.status(), StatusCode::OK);
        let all = client
            .get(&all_url)
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(all, serde_json::json!([]));
    }
}

#[tokio::test]
async fn standalone_harvest_observations_work_without_a_window_and_become_reviewed_cues() {
    let server = start_http_server(owned_storage(), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let view_token = create_share_token(&client, server.url(), &cookie).await;
    let harvest_token = create_harvest_watering_share_token(&client, server.url(), &cookie).await;
    let record_url = format!("{}/orchards/7/trees/1/harvest-observations", server.url());
    let event = serde_json::json!({ "action_date": "2026-07-12", "harvested_part": "flower", "kind": "fruit_remaining", "fruit_state": { "ready": true, "unripe": true, "wasted": false }, "window": null });
    assert_eq!(
        client
            .post(&record_url)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(&record_url)
            .header("x-orchard-share-token", &view_token)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let recorded = client
        .post(&record_url)
        .header("x-orchard-share-token", &harvest_token)
        .json(&event)
        .send()
        .await
        .unwrap();
    assert_eq!(recorded.status(), StatusCode::CREATED);
    let id = recorded.json::<serde_json::Value>().await.unwrap()["id"].clone();
    let mut empty = event.clone();
    empty["kind"] = serde_json::json!("empty_on_arrival");
    empty.as_object_mut().unwrap().remove("fruit_state");
    let recorded = client
        .post(&record_url)
        .header(header::COOKIE, &cookie)
        .json(&empty)
        .send()
        .await
        .unwrap();
    assert_eq!(recorded.status(), StatusCode::CREATED);
    let empty_id = recorded.json::<serde_json::Value>().await.unwrap()["id"].clone();
    let mut bad = event.clone();
    bad["action_date"] = serde_json::json!("2026-02-30");
    assert_eq!(
        client
            .post(&record_url)
            .header(header::COOKIE, &cookie)
            .json(&bad)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut bad = event.clone();
    bad["window"] = serde_json::json!({ "start": {"month":7,"day":1}, "end":{"month":7,"day":31}, "start_year":2026 });
    assert_eq!(
        client
            .post(&record_url)
            .header(header::COOKIE, &cookie)
            .json(&bad)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .post(format!(
                "{}/orchards/7/trees/999/harvest-observations",
                server.url()
            ))
            .header(header::COOKIE, &cookie)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let history_url = format!("{}/orchards/7/harvest-observations", server.url());
    let points = client
        .get(&history_url)
        .header("x-orchard-share-token", &view_token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(points.as_array().unwrap().len(), 2);
    assert!(points[0]["harvest_run_id"].is_null());
    assert!(points[0]["window_start"].is_null());
    assert!(points[0]["window_end"].is_null());
    assert_eq!(points[0]["harvested_parts"], serde_json::json!(["flower"]));
    assert_eq!(points[0]["evidence"], serde_json::json!([]));
    assert_eq!(points[0]["reviewed"], false);
    let review_url = format!("{}/orchards/7/harvest-observations/species/1", server.url());
    let review = serde_json::json!({ "observation_ids": [id, empty_id], "window_change": { "reference_region": "France", "windows": [{ "start": { "month": 7, "day": 1 }, "end": { "month": 7, "day": 31 }, "harvested_part": "flower" }] } });
    assert_eq!(
        client
            .post(&review_url)
            .header("x-orchard-share-token", &harvest_token)
            .json(&review)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&review_url)
            .header(header::COOKIE, &cookie)
            .json(&review)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let points = client
        .get(&history_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(
        points
            .as_array()
            .unwrap()
            .iter()
            .all(|point| point["reviewed"] == true && point["window_start"].is_null())
    );
    let map = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let window = &map["features"][0]["properties"]["harvest_windows"][0];
    assert_eq!(window["start"], "07-01");
    assert_eq!(window["harvested_part"], "flower");
    assert_eq!(
        client
            .get(format!(
                "{}/orchards/7/harvest-run?on_date=2026-07-12",
                server.url()
            ))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn legacy_harvest_observations_can_be_classified_once_by_the_owner_from_run_history() {
    let mut storage = owned_storage();
    let date = HarvestDate::new(2026, 1, 5).unwrap();
    let period = HarvestPeriod {
        start: HarvestDate::new(2025, 12, 10).unwrap(),
        end: HarvestDate::new(2026, 1, 20).unwrap(),
    };
    let mut run_ids = Vec::new();
    for outcome in [
        HarvestTreeOutcome::HarvestedEverything { harvested_on: date },
        HarvestTreeOutcome::Deferred {
            deferred_on: date,
            retry_on: date.add_days(7).unwrap(),
        },
        HarvestTreeOutcome::DoneForWindow { recorded_on: date },
    ] {
        let id = storage
            .transaction(|storage| {
                storage.create_harvest_run(
                    OrchardId(7),
                    HarvestRunTarget::All,
                    &[HarvestedPart::Fruit, HarvestedPart::Flower],
                    date,
                    &[HarvestRunTree {
                        tree_id: TreeId(1),
                        harvested_parts: vec![HarvestedPart::Fruit, HarvestedPart::Flower],
                        period,
                        outcome: None,
                    }],
                )
            })
            .unwrap();
        storage
            .transaction(|storage| {
                storage.record_harvest_tree_outcome(id, TreeId(1), outcome)?;
                storage.complete_harvest_run(id)
            })
            .unwrap();
        run_ids.push(id.0);
    }
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let share = create_harvest_watering_share_token(&client, server.url(), &cookie).await;
    let history_url = format!("{}/orchards/7/run-history", server.url());
    let history = client
        .get(&history_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert!(
        history["harvest_runs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|run| run["trees"][0]["needs_legacy_classification"] == true
                && run["trees"][0]["observation"].is_null())
    );
    let record_url = |run| {
        format!(
            "{}/orchards/7/harvest-runs/{run}/trees/1/legacy-observation",
            server.url()
        )
    };
    let event = serde_json::json!({ "kind": "harvested_all" });
    assert_eq!(
        client
            .post(record_url(run_ids[0]))
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(record_url(run_ids[0]))
            .header("x-orchard-share-token", &share)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(format!(
                "{}/orchards/8/harvest-runs/{}/trees/1/legacy-observation",
                server.url(),
                run_ids[0]
            ))
            .header(header::COOKIE, &cookie)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .post(record_url(999))
            .header(header::COOKIE, &cookie)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let mut bad = event.clone();
    bad["action_date"] = serde_json::json!("2026-09-28");
    assert_eq!(
        client
            .post(record_url(run_ids[0]))
            .header(header::COOKIE, &cookie)
            .json(&bad)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        client
            .post(record_url(run_ids[0]))
            .header(header::COOKIE, &cookie)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        client
            .post(record_url(run_ids[0]))
            .header(header::COOKIE, &cookie)
            .json(&event)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let mixed = serde_json::json!({ "kind": "fruit_remaining", "fruit_state": { "ready": true, "unripe": true, "wasted": true } });
    assert_eq!(
        client
            .post(record_url(run_ids[1]))
            .header(header::COOKIE, &cookie)
            .json(&mixed)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );
    let empty = serde_json::json!({ "kind": "empty_on_arrival" });
    assert_eq!(
        client
            .post(record_url(run_ids[2]))
            .header(header::COOKIE, &cookie)
            .json(&empty)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );
    let after = client
        .get(&history_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    for (before, run) in history["harvest_runs"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["harvest_runs"].as_array().unwrap())
    {
        assert_eq!(run["trees"][0]["outcome"], before["trees"][0]["outcome"]);
        assert_eq!(
            run["completed_at_unix_seconds"],
            before["completed_at_unix_seconds"]
        );
        assert_eq!(run["trees"][0]["needs_legacy_classification"], false);
        assert!(!run["trees"][0]["observation"].is_null());
    }
    let points = client
        .get(format!("{}/orchards/7/harvest-observations", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(points.as_array().unwrap().len(), 3);
    for (point, run_id) in points.as_array().unwrap().iter().zip(run_ids) {
        assert_eq!(point["harvest_run_id"], run_id);
        assert_eq!(point["recorded_on"], "2026-01-05");
        assert_eq!(point["window_start"], "2025-12-10");
        assert_eq!(point["window_end"], "2026-01-20");
        assert_eq!(
            point["harvested_parts"],
            serde_json::json!(["fruit", "flower"])
        );
        assert_eq!(point["reviewed"], false);
    }
}

#[tokio::test]
async fn window_validation_is_an_explicit_owner_action_separate_from_observations_and_edits() {
    use orchard_api::hexagon::models::{
        AnnualDate, AnnualHarvestWindow, HarvestDataOrigin, HarvestScheduleOwner,
    };
    let mut storage = owned_storage();
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
                [6, 9]
                    .map(|month| AnnualHarvestWindow {
                        start: AnnualDate { month, day: 1 },
                        end: AnnualDate { month, day: 30 },
                        reference_region: Some("France".into()),
                        harvested_part: HarvestedPart::Fruit,
                        data_origin: HarvestDataOrigin::ExternalReference,
                        source_url: Some("https://example.com/harvest".into()),
                        validated: false,
                    })
                    .to_vec(),
            )
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let share = create_share_token(&client, server.url(), &cookie).await;
    let path = format!(
        "{}/orchards/7/harvest-window-validation/species/1",
        server.url()
    );
    let window = serde_json::json!({"start":{"month":9,"day":1},"end":{"month":9,"day":30},"harvested_part":"fruit"});
    assert_eq!(
        client
            .post(&path)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(&path)
            .header("x-orchard-share-token", &share)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .delete(&path)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .delete(&path)
            .header("x-orchard-share-token", &share)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let report_url =
        format!("{path}?start_month=9&start_day=1&end_month=9&end_day=30&harvested_part=fruit");
    let empty: serde_json::Value = client
        .get(&report_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(empty["evidence"]["observation_count"], 0);
    assert_eq!(empty["validated"], false);
    assert_eq!(client.post(format!("{}/orchards/7/trees/1/harvest-observations",server.url())).header(header::COOKIE,&cookie).json(&serde_json::json!({"action_date":"2026-09-15","kind":"harvested_all","harvested_part":"fruit","window":{"start":{"month":9,"day":1},"end":{"month":9,"day":30},"start_year":2026}})).send().await.unwrap().status(),StatusCode::CREATED);
    let report: serde_json::Value = client
        .get(&report_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["validated"], false);
    assert_eq!(report["evidence"]["observation_count"], 1);
    assert_eq!(report["evidence"]["start_supported"], false);
    assert_eq!(report["evidence"]["end_supported"], false);
    let map_url = format!("{}/orchards/7/trees.geojson", server.url());
    let before: serde_json::Value = client
        .get(&map_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        before["features"][0]["properties"]["harvest_windows"][1]["data_origin"],
        "external_reference"
    );
    assert_eq!(
        client
            .post(&path)
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let after: serde_json::Value = client
        .get(&map_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let windows = after["features"][0]["properties"]["harvest_windows"]
        .as_array()
        .unwrap();
    assert_eq!(windows[0]["validated"], false);
    assert_eq!(windows[1]["validated"], true);
    assert_eq!(windows[1]["data_origin"], "field_observation");
    let observations: serde_json::Value = client
        .get(format!("{}/orchards/7/harvest-observations", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(observations[0]["reviewed"], false);
    assert_eq!(windows[1]["source_url"], "https://example.com/harvest");
    assert_eq!(
        client
            .delete(&path)
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let restored: serde_json::Value = client
        .get(&map_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let restored_windows = restored["features"][0]["properties"]["harvest_windows"]
        .as_array()
        .unwrap();
    assert_eq!(restored_windows[0], windows[0]);
    assert_eq!(restored_windows[1]["data_origin"], "external_reference");
    assert_eq!(restored_windows[1]["validated"], false);
    assert_eq!(
        restored_windows[1]["source_url"],
        "https://example.com/harvest"
    );
    assert_eq!(restored_windows[1]["start"], windows[1]["start"]);
    assert_eq!(restored_windows[1]["end"], windows[1]["end"]);
    let points_after: serde_json::Value = client
        .get(format!("{}/orchards/7/harvest-observations", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(points_after, observations);
    assert_eq!(
        client
            .post(&path)
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let stale = serde_json::json!({"window":{"start":{"month":9,"day":1},"end":{"month":9,"day":29},"harvested_part":"fruit"}});
    assert_eq!(
        client
            .post(&path)
            .header(header::COOKIE, &cookie)
            .json(&stale)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .delete(&path)
            .header(header::COOKIE, &cookie)
            .json(&stale)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        client
            .delete(path.replace("/orchards/7/", "/orchards/8/"))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .post(path.replace("/orchards/7/", "/orchards/8/"))
            .header(header::COOKIE, &cookie)
            .json(&serde_json::json!({"window":window}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(client.put(format!("{}/orchards/7/plant-identities/1/harvest-windows",server.url())).header(header::COOKIE,&cookie).json(&serde_json::json!({"reference_region":"France","windows":[{"start":{"month":6,"day":1},"end":{"month":6,"day":30},"harvested_part":"fruit"},{"start":{"month":9,"day":1},"end":{"month":9,"day":29},"harvested_part":"fruit"}]})).send().await.unwrap().status(),StatusCode::NO_CONTENT);
    let edited: serde_json::Value = client
        .get(&map_url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        edited["features"][0]["properties"]["harvest_windows"][1]["validated"],
        false
    );
}

#[tokio::test]
async fn review_editor_can_save_and_validate_its_replacement_window_in_one_request() {
    use orchard_api::hexagon::models::{
        AnnualDate, AnnualHarvestWindow, HarvestDataOrigin, HarvestScheduleOwner,
    };
    let mut storage = owned_storage();
    storage
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
                [6, 9]
                    .map(|month| AnnualHarvestWindow {
                        start: AnnualDate { month, day: 1 },
                        end: AnnualDate { month, day: 30 },
                        reference_region: Some("France".into()),
                        harvested_part: HarvestedPart::Fruit,
                        data_origin: HarvestDataOrigin::ExternalReference,
                        source_url: Some("https://example.com/apple".into()),
                        validated: false,
                    })
                    .to_vec(),
            )
        })
        .unwrap();
    let server = start_http_server(storage, "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let client = Client::new();
    let cookie = login_cookie(&client, server.url()).await;
    let share = create_share_token(&client, server.url(), &cookie).await;
    let recorded:serde_json::Value=client.post(format!("{}/orchards/7/trees/1/harvest-observations",server.url())).header(header::COOKIE,&cookie).json(&serde_json::json!({"action_date":"2026-09-15","kind":"harvested_all","harvested_part":"fruit","window":{"start":{"month":9,"day":1},"end":{"month":9,"day":30},"start_year":2026}})).send().await.unwrap().json().await.unwrap();
    let id = recorded["id"].as_u64().unwrap();
    let url = format!("{}/orchards/7/harvest-observations/species/1", server.url());
    let mut request = serde_json::json!({"observation_ids":[id],"window":{"start":{"month":9,"day":1},"end":{"month":9,"day":30},"harvested_part":"fruit"},"window_change":{"reference_region":"France","windows":[{"start":{"month":9,"day":5},"end":{"month":9,"day":25},"harvested_part":"fruit"}]},"validate_window":true});
    assert_eq!(
        client
            .post(&url)
            .header("x-orchard-share-token", &share)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    request["window_change"]["windows"][0]["start"]["day"] = serde_json::json!(32);
    assert_eq!(
        client
            .post(&url)
            .header(header::COOKIE, &cookie)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let before: serde_json::Value = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        before["features"][0]["properties"]["harvest_windows"][1]["start"],
        "09-01"
    );
    assert_eq!(
        before["features"][0]["properties"]["harvest_windows"][1]["validated"],
        false
    );
    let points: serde_json::Value = client
        .get(&url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(points[0]["reviewed"], false);
    request["window_change"]["windows"][0]["start"]["day"] = serde_json::json!(5);
    assert_eq!(
        client
            .post(&url)
            .header(header::COOKIE, &cookie)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let after: serde_json::Value = client
        .get(format!("{}/orchards/7/trees.geojson", server.url()))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let windows = &after["features"][0]["properties"]["harvest_windows"];
    assert_eq!(
        windows[0],
        before["features"][0]["properties"]["harvest_windows"][0]
    );
    assert_eq!(windows[1]["start"], "09-05");
    assert_eq!(windows[1]["end"], "09-25");
    assert_eq!(windows[1]["validated"], true);
    assert_eq!(windows[1]["data_origin"], "field_observation");
    let points: serde_json::Value = client
        .get(&url)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(points[0]["reviewed"], true);
    assert_eq!(points[0]["window_start"], "2026-09-01");
    request["observation_ids"] = serde_json::json!([]);
    request["window"] = request["window_change"]["windows"][0].clone();
    request["window_change"]["windows"][0]["end"]["day"] = serde_json::json!(26);
    assert_eq!(
        client
            .post(&url)
            .header(header::COOKIE, &cookie)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    request.as_object_mut().unwrap().remove("window");
    assert_eq!(
        client
            .post(&url)
            .header(header::COOKIE, &cookie)
            .json(&request)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}
