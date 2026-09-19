use base64::{Engine, engine::general_purpose::STANDARD};
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

fn webp(payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&u32::try_from(payload.len() + 4).unwrap().to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(payload);
    bytes
}
