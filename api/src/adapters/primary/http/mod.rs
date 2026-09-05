use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use crate::hexagon::models::{
    AerialOverlayId, AnnualDate, BotanicalTaxon, GeoPoint, HarvestScheduleOwner, HarvestedPart,
    InfraspecificRank, MapConfiguration, NamedTaxon, OrchardId, OrchardSharePermission,
    OrchardTree, PlantCultivarId, PlantIdentity, PlantIdentityId, Tree, TreeId, WateringRunId,
    WateringRunTarget,
};
use crate::hexagon::ports::{AccessControl, MapConfigurationStorage, OrchardStorage};
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};
use crate::hexagon::use_cases::authorize_orchard_reader::{
    OrchardReadAccessError, OrchardReadAccessRequested, OrchardReadCredential,
    authorize_orchard_reader,
};
use crate::hexagon::use_cases::authorize_orchard_waterer::{
    OrchardWateringAccessError, OrchardWateringAccessRequested, OrchardWateringCredential,
    authorize_orchard_waterer,
};
use crate::hexagon::use_cases::change_tree_condition::{
    OrchardTreeConditionChanged, TreeConditionChangeError, change_orchard_tree_condition,
};
use crate::hexagon::use_cases::list_orchard_trees::list_trees_for_orchard;
use crate::hexagon::use_cases::load_active_watering_run::{
    ActiveWateringRunError, load_active_watering_run,
};
use crate::hexagon::use_cases::load_aerial_overlay_image::{
    AerialOverlayImageLoadError, load_orchard_aerial_overlay_image,
};
use crate::hexagon::use_cases::load_map_configuration::{
    MapConfigurationLoadError, load_orchard_map_configuration,
};
use crate::hexagon::use_cases::log_in_user::{UserLoginError, UserLoginRequested, log_in_user};
use crate::hexagon::use_cases::log_out_user::log_out_user;
use crate::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderError, OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use crate::hexagon::use_cases::record_tree_watered::{
    TreeWatered, TreeWateredError, record_tree_watered,
};
use crate::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, PlantHarvestWindowsReplacementError,
    replace_orchard_harvest_windows,
};
use crate::hexagon::use_cases::restore_user_session::{
    UserSessionRestorationError, restore_user_session,
};
use crate::hexagon::use_cases::share_orchard::{
    OrchardShareError, OrchardShareLinkRequested, share_orchard,
};
use crate::hexagon::use_cases::start_danger_watering_run::{
    DangerWateringRunStartError, DangerWateringRunStartRequested, start_danger_watering_run,
};
use crate::hexagon::use_cases::start_watering_run::{
    WateringProgress, WateringRunStartError, WateringRunStartRequested, start_watering_run,
};

pub async fn start_http_server<U>(
    orchard_storage: U,
    address: SocketAddr,
) -> Result<RunningHttpServer, std::io::Error>
where
    U: AccessControl + OrchardStorage + MapConfigurationStorage + Send + 'static,
{
    let listener = TcpListener::bind(address).await?;
    let address = listener.local_addr()?;
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router(Arc::new(Mutex::new(orchard_storage))))
            .await
            .expect("the orchard HTTP server should run");
    });

    Ok(RunningHttpServer {
        url: format!("http://{address}"),
        server_task: Some(server_task),
    })
}

pub fn router<U>(orchard_storage: Arc<Mutex<U>>) -> Router
where
    U: AccessControl + OrchardStorage + MapConfigurationStorage + Send + 'static,
{
    Router::new()
        .route(
            "/session",
            post(log_in_handler::<U>)
                .get(restore_session_handler::<U>)
                .delete(log_out_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees.geojson",
            get(list_trees_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}",
            patch(change_tree_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/map-config",
            get(map_configuration_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/aerial-overlays/{overlay_id}/image",
            get(aerial_overlay_image_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/share",
            post(share_orchard_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/share/watering",
            post(share_orchard_for_watering_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/row-order",
            put(order_orchard_row_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-run",
            get(active_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs",
            post(start_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}/watered",
            post(record_tree_watered_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/plant-identities/{plant_identity_id}/harvest-windows",
            put(replace_identity_harvest_windows_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/plant-cultivars/{cultivar_id}/harvest-windows",
            put(replace_cultivar_harvest_windows_handler::<U>),
        )
        .route("/trees", post(legacy_endpoint_handler))
        .route("/trees/{tree_id}", patch(legacy_endpoint_handler))
        .route(
            "/plant-identities/{plant_identity_id}/harvest-windows",
            put(legacy_endpoint_handler),
        )
        .route(
            "/plant-cultivars/{cultivar_id}/harvest-windows",
            put(legacy_endpoint_handler),
        )
        .route("/trees.geojson", get(legacy_endpoint_handler))
        .route("/map-config", get(legacy_endpoint_handler))
        .route(
            "/aerial-overlays/{overlay_id}/image",
            get(legacy_endpoint_handler),
        )
        .with_state(orchard_storage)
}

async fn legacy_endpoint_handler() -> StatusCode {
    StatusCode::UNAUTHORIZED
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn log_in_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    request: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Response, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    let session = tokio::task::spawn_blocking(move || {
        log_in_user(
            UserLoginRequested {
                username: request.username,
                password: request.password,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|error| match error {
        UserLoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        UserLoginError::AuthenticationUnavailable => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    let cookie = HeaderValue::from_str(&format!(
        "orchard_session={}; Path=/; HttpOnly{}; SameSite=Strict; Max-Age=2592000",
        session.token,
        secure_cookie_attribute(),
    ))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut response = Json(json!({
        "user": session.user,
        "orchards": session.orchards,
    }))
    .into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

async fn restore_session_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token =
        cookie_value(&headers, "orchard_session").ok_or(StatusCode::UNAUTHORIZED)?;
    tokio::task::spawn_blocking(move || {
        restore_user_session(session_token, &mut *access_control.lock().unwrap())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|session| {
        Json(json!({
            "user": session.user,
            "orchards": session.orchards,
        }))
    })
    .map_err(|error| match error {
        UserSessionRestorationError::SessionNotFound => StatusCode::UNAUTHORIZED,
        UserSessionRestorationError::AuthenticationUnavailable => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

async fn log_out_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token =
        cookie_value(&headers, "orchard_session").ok_or(StatusCode::UNAUTHORIZED)?;
    tokio::task::spawn_blocking(move || {
        log_out_user(&session_token, &mut *access_control.lock().unwrap())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = HeaderValue::from_str(&format!(
        "orchard_session=; Path=/; HttpOnly{}; SameSite=Strict; Max-Age=0",
        secure_cookie_attribute(),
    ))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

async fn share_orchard_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    create_share_link(
        access_control,
        OrchardId(orchard_id),
        headers,
        OrchardSharePermission::View,
    )
    .await
}

async fn share_orchard_for_watering_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    create_share_link(
        access_control,
        OrchardId(orchard_id),
        headers,
        OrchardSharePermission::Watering,
    )
    .await
}

async fn create_share_link<U>(
    access_control: Arc<Mutex<U>>,
    orchard_id: OrchardId,
    headers: HeaderMap,
    permission: OrchardSharePermission,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    tokio::task::spawn_blocking(move || {
        share_orchard(
            OrchardShareLinkRequested {
                orchard_id,
                session_token,
                permission,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|share_token| Json(json!({ "share_token": share_token })))
    .map_err(|error| match error {
        OrchardShareError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardShareError::OrchardNotOwned => StatusCode::NOT_FOUND,
        OrchardShareError::ShareLinkCouldNotBeCreated => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderOrchardRowRequest {
    row_name: String,
    order: RequestedRowOrder,
}

#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum RequestedRowOrder {
    Manual { tree_ids: Vec<u64> },
    EastToWest,
    WestToEast,
    NorthToSouth,
    SouthToNorth,
}

async fn order_orchard_row_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
    request: Result<Json<OrderOrchardRowRequest>, JsonRejection>,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_owner_access(&mut *storage, orchard_id, session_token)?;
        let order = match request.order {
            RequestedRowOrder::Manual { tree_ids } => {
                RowOrder::Manual(tree_ids.into_iter().map(TreeId).collect())
            }
            RequestedRowOrder::EastToWest => RowOrder::EastToWest,
            RequestedRowOrder::WestToEast => RowOrder::WestToEast,
            RequestedRowOrder::NorthToSouth => RowOrder::NorthToSouth,
            RequestedRowOrder::SouthToNorth => RowOrder::SouthToNorth,
        };
        order_orchard_row(
            OrchardRowOrderRequested {
                orchard_id,
                row_name: request.row_name,
                order,
            },
            &mut *storage,
        )
        .map(|tree_ids| {
            Json(json!({
                "tree_ids": tree_ids.into_iter().map(|tree_id| tree_id.0).collect::<Vec<_>>()
            }))
        })
        .map_err(|error| match error {
            OrchardRowOrderError::RowNotFound => StatusCode::NOT_FOUND,
            OrchardRowOrderError::InvalidManualOrder => StatusCode::BAD_REQUEST,
            OrchardRowOrderError::OrderCouldNotBeSaved => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartWateringRunRequest {
    row_name: Option<String>,
    target: Option<RequestedWateringTarget>,
    water_source: Option<RequestedWaterSource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestedWateringTarget {
    Danger,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedWaterSource {
    longitude: f64,
    latitude: f64,
}

async fn start_watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
    request: Result<Json<StartWateringRunRequest>, JsonRejection>,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        let progress = match (request.row_name, request.target, request.water_source) {
            (Some(row_name), None, None) => start_watering_run(
                WateringRunStartRequested {
                    orchard_id,
                    row_name,
                },
                &mut *storage,
            )
            .map_err(|error| match error {
                WateringRunStartError::RowNotFound => StatusCode::NOT_FOUND,
                WateringRunStartError::RowNotOrdered
                | WateringRunStartError::AnotherWateringRunIsActive => StatusCode::CONFLICT,
                WateringRunStartError::WateringRunCouldNotBeStarted => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })?,
            (None, Some(RequestedWateringTarget::Danger), Some(water_source)) => {
                start_danger_watering_run(
                    DangerWateringRunStartRequested {
                        orchard_id,
                        water_source: GeoPoint {
                            longitude: water_source.longitude,
                            latitude: water_source.latitude,
                        },
                    },
                    &mut *storage,
                )
                .map_err(|error| match error {
                    DangerWateringRunStartError::InvalidWaterSource => StatusCode::BAD_REQUEST,
                    DangerWateringRunStartError::NoDangerTrees => StatusCode::NOT_FOUND,
                    DangerWateringRunStartError::AnotherWateringRunIsActive => StatusCode::CONFLICT,
                    DangerWateringRunStartError::WateringRunCouldNotBeStarted => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                })?
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        Ok(Json(watering_progress_json(progress)))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn active_watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        load_active_watering_run(orchard_id, &mut *storage)
            .map(|progress| match progress {
                Some(progress) => Json(watering_progress_json(progress)).into_response(),
                None => StatusCode::NO_CONTENT.into_response(),
            })
            .map_err(|error| match error {
                ActiveWateringRunError::WateringRunCouldNotBeLoaded => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordTreeWateredRequest {
    tree_id: u64,
}

async fn record_tree_watered_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, watering_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<RecordTreeWateredRequest>, JsonRejection>,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        record_tree_watered(
            TreeWatered {
                orchard_id,
                watering_run_id: WateringRunId(watering_run_id),
                tree_id: TreeId(request.tree_id),
            },
            &mut *storage,
        )
        .map(|progress| Json(watering_progress_json(progress)))
        .map_err(|error| match error {
            TreeWateredError::WateringRunNotFound => StatusCode::NOT_FOUND,
            TreeWateredError::WateringRunAlreadyCompleted | TreeWateredError::TreeIsNotNext => {
                StatusCode::CONFLICT
            }
            TreeWateredError::TreeCouldNotBeRecorded => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

fn watering_progress_json(progress: WateringProgress) -> Value {
    let (target, row_name) = match &progress.target {
        WateringRunTarget::Row(row_name) => ("row", Some(row_name.as_str())),
        WateringRunTarget::DangerTrees => ("danger", None),
    };
    json!({
        "run_id": progress.run_id.0,
        "target": target,
        "target_label": progress.target.label(),
        "row_name": row_name,
        "water_source": progress.water_source.map(|source| json!({
            "longitude": source.longitude,
            "latitude": source.latitude,
        })),
        "route": progress.route.into_iter().map(watering_tree_json).collect::<Vec<_>>(),
        "watered_tree_count": progress.watered_tree_count,
        "total_tree_count": progress.total_tree_count,
        "next_tree": progress.next_tree.map(watering_tree_json),
    })
}

fn watering_tree_json(tree: crate::hexagon::use_cases::start_watering_run::WateringTree) -> Value {
    json!({
        "id": tree.id.0,
        "name": tree.name,
        "longitude": tree.longitude,
        "latitude": tree.latitude,
        "row_rank": tree.row_rank,
    })
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find(|(cookie_name, _)| *cookie_name == name)
        .map(|(_, value)| value.to_owned())
}

fn orchard_read_credential(headers: &HeaderMap) -> Result<OrchardReadCredential, StatusCode> {
    if let Some(share_token) = headers
        .get("x-orchard-share-token")
        .and_then(|value| value.to_str().ok())
    {
        return Ok(OrchardReadCredential::ShareToken(share_token.to_owned()));
    }
    cookie_value(headers, "orchard_session")
        .map(OrchardReadCredential::OwnerSession)
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn orchard_watering_credential(
    headers: &HeaderMap,
) -> Result<OrchardWateringCredential, StatusCode> {
    if let Some(share_token) = headers
        .get("x-orchard-share-token")
        .and_then(|value| value.to_str().ok())
    {
        return Ok(OrchardWateringCredential::ShareToken(
            share_token.to_owned(),
        ));
    }
    cookie_value(headers, "orchard_session")
        .map(OrchardWateringCredential::OwnerSession)
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn owner_session_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    if headers.contains_key("x-orchard-share-token") {
        return Err(StatusCode::FORBIDDEN);
    }
    cookie_value(headers, "orchard_session").ok_or(StatusCode::UNAUTHORIZED)
}

fn authorize_owner_access(
    access_control: &mut impl AccessControl,
    orchard_id: OrchardId,
    session_token: String,
) -> Result<(), StatusCode> {
    authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id,
            session_token,
        },
        access_control,
    )
    .map(|_| ())
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardOwnerAccessError::OrchardNotOwned => StatusCode::NOT_FOUND,
        OrchardOwnerAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

fn authorize_watering_access(
    access_control: &mut impl AccessControl,
    orchard_id: OrchardId,
    credential: OrchardWateringCredential,
) -> Result<(), StatusCode> {
    authorize_orchard_waterer(
        OrchardWateringAccessRequested {
            orchard_id,
            credential,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardWateringAccessError::AccessNotFound => StatusCode::NOT_FOUND,
        OrchardWateringAccessError::PermissionDenied => StatusCode::FORBIDDEN,
        OrchardWateringAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

fn secure_cookie_attribute() -> &'static str {
    match std::env::var("ORCHARD_ALLOW_INSECURE_HTTP").as_deref() {
        Ok("true" | "1") => "",
        _ => "; Secure",
    }
}

pub struct RunningHttpServer {
    url: String,
    server_task: Option<JoinHandle<()>>,
}

impl RunningHttpServer {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn wait(mut self) -> Result<(), ()> {
        self.server_task
            .take()
            .expect("a running HTTP server should own its task")
            .await
            .map_err(|_| ())
    }
}

impl Drop for RunningHttpServer {
    fn drop(&mut self) {
        if let Some(server_task) = self.server_task.take() {
            server_task.abort();
        }
    }
}

async fn map_configuration_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + MapConfigurationStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id: OrchardId(orchard_id),
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        load_orchard_map_configuration(OrchardId(orchard_id), &mut *storage)
            .map(|configuration| Json(map_configuration_json(configuration)))
            .map_err(|error| match error {
                MapConfigurationLoadError::ConfigurationNotFound => StatusCode::NOT_FOUND,
                MapConfigurationLoadError::ConfigurationCouldNotBeRead => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn aerial_overlay_image_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, overlay_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + MapConfigurationStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    let image = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id: OrchardId(orchard_id),
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        load_orchard_aerial_overlay_image(
            OrchardId(orchard_id),
            AerialOverlayId(overlay_id),
            &mut *storage,
        )
        .map_err(|error| match error {
            AerialOverlayImageLoadError::ImageNotFound => StatusCode::NOT_FOUND,
            AerialOverlayImageLoadError::ImageCouldNotBeRead => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Response::builder()
        .header(header::CONTENT_TYPE, image.media_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header("Referrer-Policy", "no-referrer")
        .body(Body::from(image.bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn map_configuration_json(configuration: MapConfiguration) -> Value {
    let aerial_overlays = configuration.aerial_overlays.into_iter().map(|overlay| {
        let coordinates = overlay
            .corners
            .map(|point| vec![point.longitude, point.latitude]);
        json!({
            "id": overlay.id.0,
            "name": overlay.name,
            "coordinates": coordinates,
        })
    });

    json!({
        "default_center": [
            configuration.default_center.longitude,
            configuration.default_center.latitude
        ],
        "aerial_overlays": aerial_overlays.collect::<Vec<_>>(),
    })
}

async fn list_trees_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = orchard_storage.lock().unwrap();
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id: OrchardId(orchard_id),
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        list_trees_for_orchard(OrchardId(orchard_id), &mut *storage)
            .map(|trees| Json(orchard_geojson(trees)))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

#[derive(Deserialize)]
struct ChangeTreeRequest {
    is_alive: Option<bool>,
    is_in_danger: Option<bool>,
}

#[derive(Deserialize)]
struct ReplaceHarvestWindowsRequest {
    reference_region: String,
    windows: Vec<HarvestWindowRequest>,
}

#[derive(Deserialize)]
struct HarvestWindowRequest {
    start: AnnualDate,
    end: AnnualDate,
    harvested_part: HarvestedPart,
}

async fn replace_identity_harvest_windows_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, plant_identity_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<ReplaceHarvestWindowsRequest>, JsonRejection>,
) -> StatusCode
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    replace_harvest_windows_handler(
        orchard_storage,
        OrchardId(orchard_id),
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(plant_identity_id)),
        headers,
        request,
    )
    .await
}

async fn replace_cultivar_harvest_windows_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, cultivar_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<ReplaceHarvestWindowsRequest>, JsonRejection>,
) -> StatusCode
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    replace_harvest_windows_handler(
        orchard_storage,
        OrchardId(orchard_id),
        HarvestScheduleOwner::PlantCultivar(PlantCultivarId(cultivar_id)),
        headers,
        request,
    )
    .await
}

async fn replace_harvest_windows_handler<U>(
    orchard_storage: Arc<Mutex<U>>,
    orchard_id: OrchardId,
    owner: HarvestScheduleOwner,
    headers: HeaderMap,
    request: Result<Json<ReplaceHarvestWindowsRequest>, JsonRejection>,
) -> StatusCode
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let Ok(session_token) = owner_session_token(&headers) else {
        return if headers.contains_key("x-orchard-share-token") {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::UNAUTHORIZED
        };
    };
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST;
    };
    let reference_region = request.reference_region;
    let windows = request
        .windows
        .into_iter()
        .map(|window| AnnualHarvestWindowChanged {
            start_month: window.start.month,
            start_day: window.start.day,
            end_month: window.end.month,
            end_day: window.end.day,
            harvested_part: window.harvested_part,
        })
        .collect();
    match tokio::task::spawn_blocking(move || {
        let mut storage = orchard_storage.lock().unwrap();
        authorize_owner_access(&mut *storage, orchard_id, session_token)?;
        replace_orchard_harvest_windows(
            OrchardHarvestWindowsReplaced {
                orchard_id,
                owner,
                reference_region,
                windows,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            PlantHarvestWindowsReplacementError::InvalidAnnualDate
            | PlantHarvestWindowsReplacementError::MissingReferenceRegion => {
                StatusCode::BAD_REQUEST
            }
            PlantHarvestWindowsReplacementError::OwnerNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    {
        Ok(Ok(())) => StatusCode::NO_CONTENT,
        Ok(Err(status)) => status,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn change_tree_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<ChangeTreeRequest>, JsonRejection>,
) -> StatusCode
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    if headers.contains_key("x-orchard-share-token") {
        return StatusCode::FORBIDDEN;
    }
    let Some(session_token) = cookie_value(&headers, "orchard_session") else {
        return StatusCode::UNAUTHORIZED;
    };
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST;
    };
    match tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        authorize_owner_access(&mut *storage, OrchardId(orchard_id), session_token)?;
        change_orchard_tree_condition(
            OrchardTreeConditionChanged {
                orchard_id: OrchardId(orchard_id),
                tree_id: TreeId(tree_id),
                is_alive: request.is_alive,
                is_in_danger: request.is_in_danger,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            TreeConditionChangeError::NoChangesRequested => StatusCode::BAD_REQUEST,
            TreeConditionChangeError::TreeNotFound => StatusCode::NOT_FOUND,
            TreeConditionChangeError::DeadTreeCannotBeInDanger => StatusCode::CONFLICT,
            TreeConditionChangeError::TreeCouldNotBeChanged => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    {
        Ok(Ok(())) => StatusCode::NO_CONTENT,
        Ok(Err(status)) => status,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn orchard_geojson(trees: Vec<OrchardTree>) -> Value {
    let features = trees.into_iter().map(|orchard_tree| {
        let tree_id = orchard_tree.id;
        let row_rank = orchard_tree.row_rank;
        let harvest_windows = orchard_tree
            .harvest_windows
            .iter()
            .map(|window| {
                json!({
                    "start": annual_date_string(window.start),
                    "end": annual_date_string(window.end),
                    "reference_region": window.reference_region,
                    "harvested_part": window.harvested_part,
                    "data_origin": window.data_origin,
                    "source_url": window.source_url,
                })
            })
            .collect::<Vec<_>>();
        let Tree {
            legacy_source,
            plant_identity_id,
            cultivar_id,
            identification_status,
            longitude,
            latitude,
            planted_on,
            row_name,
            roles,
            is_alive,
            is_in_danger,
            adult_height_meters,
            adult_width_meters,
            ..
        } = orchard_tree.tree;
        let plant_identity_name = orchard_tree.plant_identity.common_name.clone();
        let plant_identity_taxon_name =
            botanical_taxon_name(&orchard_tree.plant_identity.botanical_taxon);
        let plant_identity_botanical_name = botanical_name_with_cultivar(
            &orchard_tree.plant_identity,
            orchard_tree.plant_cultivar.as_ref(),
        );
        let plant_identity_cultivar = orchard_tree
            .plant_cultivar
            .as_ref()
            .map(|cultivar| cultivar.cultivar.clone());
        let (botanical_genera, botanical_species) =
            botanical_filter_values(&orchard_tree.plant_identity);
        let name = legacy_source
            .as_ref()
            .map(|source| source.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| orchard_tree.plant_identity.common_name.clone());
        let latin_name = legacy_source
            .as_ref()
            .map(|source| source.latin_name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| {
                botanical_name_with_cultivar(
                    &orchard_tree.plant_identity,
                    orchard_tree.plant_cultivar.as_ref(),
                )
            });

        json!({
            "type": "Feature",
            "id": tree_id.0,
            "geometry": {
                "type": "Point",
                "coordinates": [longitude, latitude]
            },
            "properties": {
                "name": name,
                "latin_name": latin_name,
                "plant_identity_id": plant_identity_id.0,
                "plant_cultivar_id": cultivar_id.map(|id| id.0),
                "plant_identity_name": plant_identity_name,
                "plant_identity_taxon_name": plant_identity_taxon_name,
                "plant_identity_botanical_name": plant_identity_botanical_name,
                "plant_identity_cultivar": plant_identity_cultivar,
                "identification_status": identification_status,
                "harvest_windows": harvest_windows,
                "botanical_genera": botanical_genera,
                "botanical_species": botanical_species,
                "planted_on": planted_on,
                "row_name": row_name,
                "row_rank": row_rank,
                "roles": roles,
                "is_alive": is_alive,
                "is_in_danger": is_in_danger,
                "adult_height": adult_height_meters,
                "adult_width": adult_width_meters
            }
        })
    });

    json!({
        "type": "FeatureCollection",
        "features": features.collect::<Vec<_>>()
    })
}

fn annual_date_string(date: AnnualDate) -> String {
    format!("{:02}-{:02}", date.month, date.day)
}

fn botanical_filter_values(plant_identity: &PlantIdentity) -> (Vec<String>, Vec<String>) {
    let taxa = match &plant_identity.botanical_taxon {
        BotanicalTaxon::Named(taxon) => vec![taxon],
        BotanicalTaxon::HybridFormula { parents } => parents.iter().collect(),
    };
    let mut genera = Vec::new();
    let mut species = Vec::new();

    for taxon in taxa {
        if !genera.contains(&taxon.genus) {
            genera.push(taxon.genus.clone());
        }
        if let Some(species_name) = &taxon.species {
            let hybrid_marker = if taxon.species_is_hybrid { "× " } else { "" };
            let full_name = format!("{} {hybrid_marker}{species_name}", taxon.genus);
            if !species.contains(&full_name) {
                species.push(full_name);
            }
        }
    }

    (genera, species)
}

fn botanical_name(plant_identity: &PlantIdentity) -> String {
    botanical_taxon_name(&plant_identity.botanical_taxon)
}

fn botanical_name_with_cultivar(
    plant_identity: &PlantIdentity,
    plant_cultivar: Option<&crate::hexagon::models::PlantCultivar>,
) -> String {
    let mut name = botanical_name(plant_identity);
    if let Some(cultivar) = plant_cultivar {
        let cultivar = &cultivar.cultivar;
        name.push_str(&format!(" ‘{cultivar}’"));
    }
    name
}

fn botanical_taxon_name(botanical_taxon: &BotanicalTaxon) -> String {
    match botanical_taxon {
        BotanicalTaxon::Named(taxon) => named_taxon(taxon),
        BotanicalTaxon::HybridFormula { parents } => {
            format!(
                "{} × {}",
                named_taxon(&parents[0]),
                named_taxon(&parents[1])
            )
        }
    }
}

fn named_taxon(taxon: &NamedTaxon) -> String {
    let mut parts = vec![taxon.genus.clone()];
    if let Some(species) = &taxon.species {
        if taxon.species_is_hybrid {
            parts.push("×".into());
        }
        parts.push(species.clone());
    }
    if let Some(infraspecific) = &taxon.infraspecific {
        parts.push(match infraspecific.rank {
            InfraspecificRank::Variety => "var.".into(),
            InfraspecificRank::Subspecies => "subsp.".into(),
        });
        parts.push(infraspecific.name.clone());
    }
    if taxon.is_aggregate {
        parts.push("agg.".into());
    }
    if let Some(cultivar_group) = &taxon.cultivar_group {
        parts.push(format!("{cultivar_group} Group"));
    }
    parts.join(" ")
}
