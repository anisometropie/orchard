use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use crate::hexagon::models::{
    AerialOverlayId, AnnualDate, BotanicalTaxon, GeoPoint, HarvestDate, HarvestRunId,
    HarvestRunTarget, HarvestScheduleOwner, HarvestTreeOutcome, HarvestedPart, InfraspecificRank,
    MapConfiguration, NamedTaxon, OrchardId, OrchardSharePermission, OrchardSharePermissions,
    OrchardShareTokenId, OrchardTree, PlantCultivarId, PlantIdentity, PlantIdentityId, Tree,
    TreeId, TreePhotoId, TreePhotoVariant, WateringRunId, WateringRunTarget,
};
use crate::hexagon::ports::{
    AccessControl, MapConfigurationStorage, OrchardStorage, TreePhotoStorage,
};
use crate::hexagon::use_cases::add_tree_photo::{
    TreePhotoAddError, TreePhotoAdded, add_tree_photo,
};
use crate::hexagon::use_cases::authorize_orchard_harvester::{
    OrchardHarvestingAccessError, OrchardHarvestingAccessRequested, OrchardHarvestingCredential,
    authorize_orchard_harvester,
};
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};
use crate::hexagon::use_cases::authorize_orchard_photographer::{
    OrchardPhotographyAccessError, OrchardPhotographyAccessRequested, OrchardPhotographyCredential,
    authorize_orchard_photographer,
};
use crate::hexagon::use_cases::authorize_orchard_reader::{
    OrchardReadAccessError, OrchardReadAccessRequested, OrchardReadCredential,
    authorize_orchard_reader,
};
use crate::hexagon::use_cases::authorize_orchard_waterer::{
    OrchardWateringAccessError, OrchardWateringAccessRequested, OrchardWateringCredential,
    authorize_orchard_waterer,
};
use crate::hexagon::use_cases::cancel_harvest_run::{
    HarvestRunCancellationError, HarvestRunCancellationRequested, cancel_harvest_run,
};
use crate::hexagon::use_cases::cancel_watering_run::{
    WateringRunCancellationError, WateringRunCancellationRequested, cancel_watering_run,
};
use crate::hexagon::use_cases::change_orchard_share_permissions::{
    OrchardSharePermissionsChangeError, OrchardSharePermissionsChanged,
    change_orchard_share_permissions,
};
use crate::hexagon::use_cases::change_tree_condition::{
    OrchardTreeConditionChanged, TreeConditionChangeError, change_orchard_tree_condition,
};
use crate::hexagon::use_cases::defer_harvest_tree::{
    HarvestTreeDeferralError, HarvestTreeDeferred, defer_harvest_tree,
};
use crate::hexagon::use_cases::delete_tree_photo::{
    TreePhotoDeleteError, TreePhotoDeletionRequested, delete_tree_photo,
};
use crate::hexagon::use_cases::list_harvest_candidates::{
    HarvestCandidatesError, HarvestCandidatesRequested, list_harvest_candidates,
};
use crate::hexagon::use_cases::list_orchard_run_history::{
    HarvestRunHistory, OrchardRunHistory, OrchardRunHistoryError, OrchardRunHistoryRequested,
    WateringRunHistory, list_orchard_run_history,
};
use crate::hexagon::use_cases::list_orchard_shares::{
    OrchardSharesListError, OrchardSharesRequested, list_orchard_shares,
};
use crate::hexagon::use_cases::list_orchard_trees::list_trees_for_orchard;
use crate::hexagon::use_cases::list_tree_photos::{
    TreePhotosListError, TreePhotosRequested, list_tree_photos,
};
use crate::hexagon::use_cases::list_watering_runs::list_watering_runs;
use crate::hexagon::use_cases::load_active_harvest_run::{
    ActiveHarvestRunError, load_active_harvest_run,
};
use crate::hexagon::use_cases::load_active_watering_run::{
    ActiveWateringRunError, load_active_watering_run,
};
use crate::hexagon::use_cases::load_aerial_overlay_image::{
    AerialOverlayImageLoadError, load_orchard_aerial_overlay_image,
};
use crate::hexagon::use_cases::load_latest_tree_photo::{
    LatestTreePhotoLoadError, LatestTreePhotoRequested, load_latest_tree_photo,
};
use crate::hexagon::use_cases::load_map_configuration::{
    MapConfigurationLoadError, load_orchard_map_configuration,
};
use crate::hexagon::use_cases::load_tree_photo::{
    TreePhotoLoadError, TreePhotoRequested, load_tree_photo,
};
use crate::hexagon::use_cases::load_watering_run::{WateringRunLoadError, load_watering_run};
use crate::hexagon::use_cases::log_in_user::{UserLoginError, UserLoginRequested, log_in_user};
use crate::hexagon::use_cases::log_out_user::log_out_user;
use crate::hexagon::use_cases::mark_watering_tree_dead::{
    WateringTreeMarkedDead, WateringTreeMarkedDeadError, mark_watering_tree_dead,
};
use crate::hexagon::use_cases::move_tree::{
    OrchardTreeMoveConfirmed, OrchardTreeMoveError, move_orchard_tree,
};
use crate::hexagon::use_cases::order_orchard_row::{
    OrchardRowOrderError, OrchardRowOrderRequested, RowOrder, order_orchard_row,
};
use crate::hexagon::use_cases::pause_watering_run::{
    WateringRunPauseError, WateringRunPauseRequested, pause_watering_run,
};
use crate::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, TreeHarvestedEverythingError, record_tree_harvested,
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
use crate::hexagon::use_cases::resume_watering_run::{
    WateringRunResumeError, WateringRunResumeRequested, resume_watering_run,
};
use crate::hexagon::use_cases::revoke_orchard_share::{
    OrchardShareRevokeError, OrchardShareRevoked, revoke_orchard_share,
};
use crate::hexagon::use_cases::share_orchard::{
    OrchardShareError, OrchardShareLinkRequested, share_orchard,
};
use crate::hexagon::use_cases::start_danger_watering_run::{
    DangerWateringRunStartError, DangerWateringRunStartRequested, start_danger_watering_run,
};
use crate::hexagon::use_cases::start_harvest_run::{
    HarvestProgress, HarvestRunStartError, HarvestRunStartRequested, start_harvest_run,
};
use crate::hexagon::use_cases::start_watering_run::{
    WateringProgress, WateringRunStartError, WateringRunStartRequested, start_watering_run,
};
use crate::hexagon::use_cases::undo_last_harvest_tree_action::{
    LastHarvestTreeActionUndoError, LastHarvestTreeActionUndone, undo_last_harvest_tree_action,
};

mod tree_photo_image;

use self::tree_photo_image::{TreePhotoPreparationError, prepare_tree_photo};

pub async fn start_http_server<U>(
    orchard_storage: U,
    address: SocketAddr,
) -> Result<RunningHttpServer, std::io::Error>
where
    U: AccessControl + OrchardStorage + MapConfigurationStorage + TreePhotoStorage + Send + 'static,
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
    U: AccessControl + OrchardStorage + MapConfigurationStorage + TreePhotoStorage + Send + 'static,
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
            "/orchards/{orchard_id}/trees/{tree_id}/position",
            put(move_tree_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}/photos",
            get(list_tree_photos_handler::<U>)
                .post(add_tree_photo_handler::<U>)
                .layer(DefaultBodyLimit::max(55 * 1024 * 1024)),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}/photos/latest",
            get(latest_tree_photo_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}/photos/latest/thumbnail",
            get(latest_tree_photo_thumbnail_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}/photos/{photo_id}",
            get(tree_photo_handler::<U>).delete(delete_tree_photo_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/trees/{tree_id}/photos/{photo_id}/thumbnail",
            get(tree_photo_thumbnail_handler::<U>),
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
            "/orchards/{orchard_id}/share/harvest-watering",
            post(share_orchard_for_harvest_and_watering_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/share-access",
            get(share_access_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/share-tokens",
            get(list_share_tokens_handler::<U>).post(create_share_token_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/share-tokens/{share_id}",
            patch(change_share_token_permissions_handler::<U>)
                .delete(revoke_share_token_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/run-history",
            get(run_history_handler::<U>),
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
            get(list_watering_runs_handler::<U>).post(start_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}",
            get(watering_run_handler::<U>).delete(cancel_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}/pause",
            post(pause_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}/resume",
            post(resume_watering_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}/watered",
            post(record_tree_watered_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/watering-runs/{watering_run_id}/dead",
            post(mark_watering_tree_dead_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-run",
            get(active_harvest_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-candidates",
            get(harvest_candidates_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-runs",
            post(start_harvest_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-runs/{harvest_run_id}",
            delete(cancel_harvest_run_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-runs/{harvest_run_id}/harvested",
            post(record_tree_harvested_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-runs/{harvest_run_id}/deferred",
            post(defer_harvest_tree_handler::<U>),
        )
        .route(
            "/orchards/{orchard_id}/harvest-runs/{harvest_run_id}/previous",
            post(undo_last_harvest_tree_action_handler::<U>),
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
        OrchardSharePermission::View.into(),
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
        OrchardSharePermission::Watering.into(),
    )
    .await
}

async fn share_orchard_for_harvest_and_watering_handler<U>(
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
        OrchardSharePermission::HarvestAndWatering.into(),
    )
    .await
}

async fn create_share_link<U>(
    access_control: Arc<Mutex<U>>,
    orchard_id: OrchardId,
    headers: HeaderMap,
    permissions: OrchardSharePermissions,
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
                permissions,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|created| Json(json!({ "share_token": created.token, "id": created.id.0 })))
    .map_err(|error| match error {
        OrchardShareError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardShareError::OrchardNotOwned => StatusCode::NOT_FOUND,
        OrchardShareError::ShareLinkCouldNotBeCreated => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SharePermissionsRequest {
    permissions: OrchardSharePermissions,
}

async fn create_share_token_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
    request: Result<Json<SharePermissionsRequest>, JsonRejection>,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    create_share_link(
        access_control,
        OrchardId(orchard_id),
        headers,
        request.permissions,
    )
    .await
}

async fn list_share_tokens_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    tokio::task::spawn_blocking(move || {
        list_orchard_shares(
            OrchardSharesRequested {
                orchard_id: OrchardId(orchard_id),
                session_token,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|shares| Json(json!({ "shares": shares })))
    .map_err(|error| match error {
        OrchardSharesListError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardSharesListError::OrchardNotOwned => StatusCode::NOT_FOUND,
        OrchardSharesListError::SharesCouldNotBeListed => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

async fn change_share_token_permissions_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path((orchard_id, share_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<SharePermissionsRequest>, JsonRejection>,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        change_orchard_share_permissions(
            OrchardSharePermissionsChanged {
                orchard_id: OrchardId(orchard_id),
                share_id: OrchardShareTokenId(share_id),
                permissions: request.permissions,
                session_token,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|()| StatusCode::NO_CONTENT)
    .map_err(|error| match error {
        OrchardSharePermissionsChangeError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardSharePermissionsChangeError::OrchardNotOwned
        | OrchardSharePermissionsChangeError::ShareNotFound => StatusCode::NOT_FOUND,
        OrchardSharePermissionsChangeError::PermissionsCouldNotBeChanged => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })
}

async fn revoke_share_token_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path((orchard_id, share_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    tokio::task::spawn_blocking(move || {
        revoke_orchard_share(
            OrchardShareRevoked {
                orchard_id: OrchardId(orchard_id),
                share_id: OrchardShareTokenId(share_id),
                session_token,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|()| StatusCode::NO_CONTENT)
    .map_err(|error| match error {
        OrchardShareRevokeError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardShareRevokeError::OrchardNotOwned | OrchardShareRevokeError::ShareNotFound => {
            StatusCode::NOT_FOUND
        }
        OrchardShareRevokeError::ShareCouldNotBeRevoked => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

async fn share_access_handler<U>(
    State(access_control): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id: OrchardId(orchard_id),
                credential,
            },
            &mut *access_control.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|access| {
        let permissions = match access {
            crate::hexagon::use_cases::authorize_orchard_reader::OrchardReadAccess::Editable => {
                OrchardSharePermissions {
                    harvest: true,
                    water: true,
                    add_photos: true,
                }
            }
            crate::hexagon::use_cases::authorize_orchard_reader::OrchardReadAccess::ReadOnly(
                permissions,
            ) => permissions,
        };
        Json(json!({ "permissions": permissions }))
    })
    .map_err(|error| match error {
        OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
        OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

async fn run_history_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    tokio::task::spawn_blocking(move || {
        list_orchard_run_history(
            OrchardRunHistoryRequested {
                orchard_id: OrchardId(orchard_id),
                session_token,
            },
            &mut *storage.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|history| Json(run_history_json(history)))
    .map_err(|error| match error {
        OrchardRunHistoryError::SessionNotFound => StatusCode::UNAUTHORIZED,
        OrchardRunHistoryError::OrchardNotOwned => StatusCode::NOT_FOUND,
        OrchardRunHistoryError::HistoryCouldNotBeRead => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

fn run_history_json(history: OrchardRunHistory) -> Value {
    json!({
        "watering_runs": history
            .watering_runs
            .into_iter()
            .map(watering_run_history_json)
            .collect::<Vec<_>>(),
        "harvest_runs": history
            .harvest_runs
            .into_iter()
            .map(harvest_run_history_json)
            .collect::<Vec<_>>(),
    })
}

fn watering_run_history_json(run: WateringRunHistory) -> Value {
    let watered_tree_count = run
        .trees
        .iter()
        .filter(|tree| tree.watered_at_unix_seconds.is_some())
        .count();
    let skipped_tree_count = run
        .trees
        .iter()
        .filter(|tree| tree.skipped_at_unix_seconds.is_some())
        .count();
    json!({
        "run_id": run.run_id.0,
        "target_label": run.target_label,
        "carry_capacity": run.carry_capacity,
        "started_at_unix_seconds": run.started_at_unix_seconds,
        "completed_at_unix_seconds": run.completed_at_unix_seconds,
        "watered_tree_count": watered_tree_count,
        "skipped_tree_count": skipped_tree_count,
        "handled_tree_count": watered_tree_count + skipped_tree_count,
        "total_tree_count": run.trees.len(),
        "trees": run.trees.into_iter().map(|tree| json!({
            "tree_id": tree.tree_id.0,
            "name": tree.name,
            "watered_at_unix_seconds": tree.watered_at_unix_seconds,
            "skipped_at_unix_seconds": tree.skipped_at_unix_seconds,
        })).collect::<Vec<_>>(),
    })
}

fn harvest_run_history_json(run: HarvestRunHistory) -> Value {
    let handled_tree_count = run.trees.len();
    json!({
        "run_id": run.run_id.0,
        "target_label": run.target_label,
        "harvested_parts": run.harvested_parts.into_iter()
            .map(HarvestedPart::as_str)
            .collect::<Vec<_>>(),
        "started_on": run.started_on.to_string(),
        "completed_at_unix_seconds": run.completed_at_unix_seconds,
        "handled_tree_count": handled_tree_count,
        "total_tree_count": run.total_tree_count,
        "trees": run.trees.into_iter().map(|tree| json!({
            "tree_id": tree.tree_id.0,
            "name": tree.name,
            "harvested_parts": tree.harvested_parts.into_iter()
                .map(HarvestedPart::as_str)
                .collect::<Vec<_>>(),
            "window_start": tree.period.start.to_string(),
            "window_end": tree.period.end.to_string(),
            "outcome": harvest_history_outcome_json(tree.outcome),
        })).collect::<Vec<_>>(),
    })
}

fn harvest_history_outcome_json(outcome: HarvestTreeOutcome) -> Value {
    match outcome {
        HarvestTreeOutcome::HarvestedEverything { harvested_on } => json!({
            "kind": "harvested_everything",
            "recorded_on": harvested_on.to_string(),
        }),
        HarvestTreeOutcome::DoneForWindow { recorded_on } => json!({
            "kind": "done_for_window",
            "recorded_on": recorded_on.to_string(),
        }),
        HarvestTreeOutcome::Deferred {
            deferred_on,
            retry_on,
        } => json!({
            "kind": "deferred",
            "recorded_on": deferred_on.to_string(),
            "retry_on": retry_on.to_string(),
        }),
    }
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
    carry_capacity: Option<u32>,
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
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        let progress = match (
            request.row_name,
            request.target,
            request.water_source,
            request.carry_capacity,
        ) {
            (Some(row_name), None, None, None) => match start_watering_run(
                WateringRunStartRequested {
                    orchard_id,
                    row_name,
                },
                &mut *storage,
            ) {
                Ok(progress) => progress,
                Err(error) => return Ok(watering_start_error_response(error)),
            },
            (None, Some(RequestedWateringTarget::Danger), Some(water_source), carry_capacity) => {
                match start_danger_watering_run(
                    DangerWateringRunStartRequested {
                        orchard_id,
                        water_source: GeoPoint {
                            longitude: water_source.longitude,
                            latitude: water_source.latitude,
                        },
                        carry_capacity: carry_capacity.unwrap_or(2),
                    },
                    &mut *storage,
                ) {
                    Ok(progress) => progress,
                    Err(error) => return Ok(danger_watering_start_error_response(error)),
                }
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        Ok(Json(watering_progress_json(progress)).into_response())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

fn watering_start_error_response(error: WateringRunStartError) -> Response {
    match error {
        WateringRunStartError::RowNotFound => StatusCode::NOT_FOUND.into_response(),
        WateringRunStartError::RowNotOrdered => watering_conflict_response(
            "watering_row_not_ordered",
            "Order every tree included in watering in this row before starting.",
        ),
        WateringRunStartError::HarvestRunIsActive => {
            watering_conflict_response("harvest_run_active", "A harvest tour is already active.")
        }
        WateringRunStartError::WateringRunCouldNotBeStarted => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn danger_watering_start_error_response(error: DangerWateringRunStartError) -> Response {
    match error {
        DangerWateringRunStartError::InvalidWaterSource
        | DangerWateringRunStartError::InvalidCarryCapacity => {
            StatusCode::BAD_REQUEST.into_response()
        }
        DangerWateringRunStartError::NoDangerTrees => StatusCode::NOT_FOUND.into_response(),
        DangerWateringRunStartError::HarvestRunIsActive => {
            watering_conflict_response("harvest_run_active", "A harvest tour is already active.")
        }
        DangerWateringRunStartError::WateringRunCouldNotBeStarted => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn watering_conflict_response(code: &str, message: &str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}

async fn list_watering_runs_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        list_watering_runs(orchard_id, &mut *storage)
            .map(|runs| Json(json!({"runs": runs.into_iter().map(watering_progress_json).collect::<Vec<_>>()})))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        load_watering_run(orchard_id, WateringRunId(run_id), &mut *storage)
            .map(|progress| Json(watering_progress_json(progress)))
            .map_err(|error| match error {
                WateringRunLoadError::WateringRunNotFound => StatusCode::NOT_FOUND,
                WateringRunLoadError::WateringRunCouldNotBeLoaded => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn pause_watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        pause_watering_run(
            WateringRunPauseRequested {
                orchard_id,
                watering_run_id: WateringRunId(run_id),
            },
            &mut *storage,
        )
        .map(|progress| Json(watering_progress_json(progress)))
        .map_err(|error| match error {
            WateringRunPauseError::WateringRunNotFound => StatusCode::NOT_FOUND,
            WateringRunPauseError::WateringRunAlreadyCompleted => StatusCode::CONFLICT,
            WateringRunPauseError::WateringRunCouldNotBePaused => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn resume_watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, run_id)): Path<(u64, u64)>,
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
        Ok(
            match resume_watering_run(
                WateringRunResumeRequested {
                    orchard_id,
                    watering_run_id: WateringRunId(run_id),
                },
                &mut *storage,
            ) {
                Ok(progress) => Json(watering_progress_json(progress)).into_response(),
                Err(WateringRunResumeError::WateringRunNotFound) => {
                    StatusCode::NOT_FOUND.into_response()
                }
                Err(WateringRunResumeError::WateringRunAlreadyCompleted) => {
                    StatusCode::CONFLICT.into_response()
                }
                Err(WateringRunResumeError::AnotherWateringRunIsActive) => {
                    watering_conflict_response(
                        "watering_run_active",
                        "Another watering run for this target is already active.",
                    )
                }
                Err(WateringRunResumeError::HarvestRunIsActive) => watering_conflict_response(
                    "harvest_run_active",
                    "A harvest tour is already active.",
                ),
                Err(WateringRunResumeError::WateringRunCouldNotBeResumed) => {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            },
        )
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
            TreeWateredError::WateringRunAlreadyCompleted
            | TreeWateredError::WateringRunIsPaused
            | TreeWateredError::TreeIsNotNext => StatusCode::CONFLICT,
            TreeWateredError::TreeCouldNotBeRecorded => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkWateringTreeDeadRequest {
    tree_id: u64,
}

async fn mark_watering_tree_dead_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, watering_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<MarkWateringTreeDeadRequest>, JsonRejection>,
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
        mark_watering_tree_dead(
            WateringTreeMarkedDead {
                orchard_id,
                watering_run_id: WateringRunId(watering_run_id),
                tree_id: TreeId(request.tree_id),
            },
            &mut *storage,
        )
        .map(|progress| Json(watering_progress_json(progress)))
        .map_err(|error| match error {
            WateringTreeMarkedDeadError::WateringRunNotFound => StatusCode::NOT_FOUND,
            WateringTreeMarkedDeadError::WateringRunAlreadyCompleted
            | WateringTreeMarkedDeadError::WateringRunIsPaused
            | WateringTreeMarkedDeadError::TreeIsNotNext => StatusCode::CONFLICT,
            WateringTreeMarkedDeadError::TreeCouldNotBeMarkedDead => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn cancel_watering_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, watering_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_watering_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_watering_access(&mut *storage, orchard_id, credential)?;
        cancel_watering_run(
            WateringRunCancellationRequested {
                orchard_id,
                watering_run_id: WateringRunId(watering_run_id),
            },
            &mut *storage,
        )
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|error| match error {
            WateringRunCancellationError::WateringRunNotFound => StatusCode::NOT_FOUND,
            WateringRunCancellationError::WateringRunAlreadyCompleted => StatusCode::CONFLICT,
            WateringRunCancellationError::WateringRunCouldNotBeCancelled => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
        "paused": progress.paused,
        "target": target,
        "target_label": progress.target.label(),
        "row_name": row_name,
        "water_source": progress.water_source.map(|source| json!({
            "longitude": source.longitude,
            "latitude": source.latitude,
        })),
        "carry_capacity": progress.carry_capacity,
        "route": progress.route.into_iter().map(watering_tree_json).collect::<Vec<_>>(),
        "watered_tree_count": progress.watered_tree_count,
        "skipped_tree_count": progress.skipped_tree_count,
        "handled_tree_count": progress.handled_tree_count,
        "watered_tree_ids": progress.watered_tree_ids.into_iter().map(|id| id.0).collect::<Vec<_>>(),
        "skipped_tree_ids": progress.skipped_tree_ids.into_iter().map(|id| id.0).collect::<Vec<_>>(),
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

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestedHarvestTarget {
    All,
    Species,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartHarvestRunRequest {
    target: RequestedHarvestTarget,
    plant_identity_id: Option<u64>,
    #[serde(default = "default_harvested_parts")]
    harvested_parts: Vec<HarvestedPart>,
    on_date: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HarvestCandidatesQuery {
    on_date: String,
    #[serde(default = "default_harvested_parts_query")]
    harvested_parts: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveHarvestRunQuery {
    on_date: String,
}

async fn harvest_candidates_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    Query(query): Query<HarvestCandidatesQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    let harvested_parts =
        parse_harvested_parts(&query.harvested_parts).ok_or(StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_harvesting_access(&mut *storage, orchard_id, credential)?;
        list_harvest_candidates(
            HarvestCandidatesRequested {
                orchard_id,
                harvested_parts,
                action_date: query.on_date,
            },
            &mut *storage,
        )
        .map(|candidates| {
            Json(json!({
                "candidates": candidates.into_iter().map(|candidate| json!({
                    "tree_id": candidate.tree_id.0,
                    "plant_identity_id": candidate.plant_identity_id.0,
                })).collect::<Vec<_>>(),
            }))
        })
        .map_err(|error| match error {
            HarvestCandidatesError::InvalidActionDate => StatusCode::BAD_REQUEST,
            HarvestCandidatesError::NoHarvestPartsSelected => StatusCode::BAD_REQUEST,
            HarvestCandidatesError::HarvestCandidatesCouldNotBeListed => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn start_harvest_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    headers: HeaderMap,
    request: Result<Json<StartHarvestRunRequest>, JsonRejection>,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    let target = match (request.target, request.plant_identity_id) {
        (RequestedHarvestTarget::All, None) => HarvestRunTarget::All,
        (RequestedHarvestTarget::Species, Some(plant_identity_id)) if plant_identity_id > 0 => {
            HarvestRunTarget::Species(PlantIdentityId(plant_identity_id))
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_harvesting_access(&mut *storage, orchard_id, credential)?;
        start_harvest_run(
            HarvestRunStartRequested {
                orchard_id,
                target,
                harvested_parts: request.harvested_parts,
                action_date: request.on_date,
            },
            &mut *storage,
        )
        .map(|progress| Json(harvest_progress_json(progress)))
        .map_err(|error| match error {
            HarvestRunStartError::InvalidActionDate
            | HarvestRunStartError::NoHarvestPartsSelected => StatusCode::BAD_REQUEST,
            HarvestRunStartError::NoTreesCurrentlyAvailable => StatusCode::NOT_FOUND,
            HarvestRunStartError::AnotherHarvestRunIsActive
            | HarvestRunStartError::WateringRunIsActive => StatusCode::CONFLICT,
            HarvestRunStartError::HarvestRunCouldNotBeStarted => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn active_harvest_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(orchard_id): Path<u64>,
    Query(query): Query<ActiveHarvestRunQuery>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_harvesting_access(&mut *storage, orchard_id, credential)?;
        load_active_harvest_run(orchard_id, &query.on_date, &mut *storage)
            .map(|progress| match progress {
                Some(progress) => Json(harvest_progress_json(progress)).into_response(),
                None => StatusCode::NO_CONTENT.into_response(),
            })
            .map_err(|error| match error {
                ActiveHarvestRunError::InvalidActionDate => StatusCode::BAD_REQUEST,
                ActiveHarvestRunError::HarvestRunCouldNotBeLoaded => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordHarvestTreeRequest {
    tree_id: u64,
    action_date: String,
    extend_window: Option<bool>,
}

async fn record_tree_harvested_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, harvest_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<RecordHarvestTreeRequest>, JsonRejection>,
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    if request.extend_window.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let response = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        if let Err(status) = authorize_harvesting_access(&mut *storage, orchard_id, credential) {
            return status.into_response();
        }
        match record_tree_harvested(
            TreeHarvestedEverything {
                orchard_id,
                harvest_run_id: HarvestRunId(harvest_run_id),
                tree_id: TreeId(request.tree_id),
                action_date: request.action_date,
            },
            &mut *storage,
        ) {
            Ok(progress) => Json(harvest_progress_json(progress)).into_response(),
            Err(error) => match error {
                TreeHarvestedEverythingError::InvalidActionDate => {
                    StatusCode::BAD_REQUEST.into_response()
                }
                TreeHarvestedEverythingError::HarvestRunNotFound => {
                    StatusCode::NOT_FOUND.into_response()
                }
                TreeHarvestedEverythingError::HarvestRunAlreadyCompleted
                | TreeHarvestedEverythingError::TreeIsNotCurrent => {
                    StatusCode::CONFLICT.into_response()
                }
                TreeHarvestedEverythingError::ActionDateOutsideRunPeriod => {
                    harvest_conflict_response(
                        "harvest_action_date_outside_run_period",
                        "The action date is outside the harvest period captured by this tour.",
                    )
                }
                TreeHarvestedEverythingError::HarvestWindowChanged => harvest_conflict_response(
                    "harvest_window_changed",
                    "The live harvest window changed after this tour started.",
                ),
                TreeHarvestedEverythingError::TreeCouldNotBeRecorded => {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            },
        }
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

async fn cancel_harvest_run_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, harvest_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_harvesting_access(&mut *storage, orchard_id, credential)?;
        cancel_harvest_run(
            HarvestRunCancellationRequested {
                orchard_id,
                harvest_run_id: HarvestRunId(harvest_run_id),
            },
            &mut *storage,
        )
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|error| match error {
            HarvestRunCancellationError::HarvestRunNotFound => StatusCode::NOT_FOUND,
            HarvestRunCancellationError::HarvestRunAlreadyCompleted => StatusCode::CONFLICT,
            HarvestRunCancellationError::HarvestRunCouldNotBeCancelled => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn defer_harvest_tree_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, harvest_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<RecordHarvestTreeRequest>, JsonRejection>,
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    let action_date = request.action_date.clone();
    let response = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        if let Err(status) = authorize_harvesting_access(&mut *storage, orchard_id, credential) {
            return status.into_response();
        }
        match defer_harvest_tree(
            HarvestTreeDeferred {
                orchard_id,
                harvest_run_id: HarvestRunId(harvest_run_id),
                tree_id: TreeId(request.tree_id),
                action_date,
                extend_window: request.extend_window,
            },
            &mut *storage,
        ) {
            Ok(progress) => Json(harvest_progress_json(progress)).into_response(),
            Err(error) => match error {
                HarvestTreeDeferralError::InvalidActionDate => {
                    StatusCode::BAD_REQUEST.into_response()
                }
                HarvestTreeDeferralError::HarvestRunNotFound => {
                    StatusCode::NOT_FOUND.into_response()
                }
                HarvestTreeDeferralError::HarvestRunAlreadyCompleted
                | HarvestTreeDeferralError::TreeIsNotCurrent => {
                    StatusCode::CONFLICT.into_response()
                }
                HarvestTreeDeferralError::ActionDateOutsideRunPeriod => harvest_conflict_response(
                    "harvest_action_date_outside_run_period",
                    "The action date is outside the harvest period captured by this tour.",
                ),
                HarvestTreeDeferralError::HarvestWindowChanged => harvest_conflict_response(
                    "harvest_window_changed",
                    "The live harvest window changed after this tour started.",
                ),
                HarvestTreeDeferralError::WindowExtensionRequired(proposal) => {
                    let retry_on = HarvestDate::parse_iso(&request.action_date)
                        .and_then(|date| date.add_days(7));
                    (
                        StatusCode::CONFLICT,
                        Json(json!({
                            "code": "harvest_window_extension_required",
                            "current_end": proposal.current_end.to_string(),
                            "proposed_end": proposal.proposed_end.to_string(),
                            "retry_on": retry_on.map(|date| date.to_string()),
                            "schedule_label": "species/cultivar harvest",
                        })),
                    )
                        .into_response()
                }
                HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended
                | HarvestTreeDeferralError::TreeCouldNotBeDeferred => {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            },
        }
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UndoHarvestTreeActionRequest {
    action_date: String,
}

async fn undo_last_harvest_tree_action_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, harvest_run_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<UndoHarvestTreeActionRequest>, JsonRejection>,
) -> Result<Response, StatusCode>
where
    U: AccessControl + OrchardStorage + Send + 'static,
{
    let credential = orchard_harvesting_credential(&headers)?;
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    let response = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        if let Err(status) = authorize_harvesting_access(&mut *storage, orchard_id, credential) {
            return status.into_response();
        }
        match undo_last_harvest_tree_action(
            LastHarvestTreeActionUndone {
                orchard_id,
                harvest_run_id: HarvestRunId(harvest_run_id),
                action_date: request.action_date,
            },
            &mut *storage,
        ) {
            Ok(progress) => Json(harvest_progress_json(progress)).into_response(),
            Err(error) => match error {
                LastHarvestTreeActionUndoError::InvalidActionDate => {
                    StatusCode::BAD_REQUEST.into_response()
                }
                LastHarvestTreeActionUndoError::HarvestRunNotFound => {
                    StatusCode::NOT_FOUND.into_response()
                }
                LastHarvestTreeActionUndoError::NoHarvestTreeActionToUndo => {
                    harvest_conflict_response(
                        "no_harvest_tree_action_to_undo",
                        "There is no previous harvest-tree action to undo.",
                    )
                }
                LastHarvestTreeActionUndoError::HarvestTreeActionCouldNotBeUndone => {
                    harvest_conflict_response(
                        "harvest_tree_action_could_not_be_undone",
                        "The previous harvest-tree action could not be safely undone.",
                    )
                }
            },
        }
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

fn harvest_conflict_response(code: &str, message: &str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}

fn harvest_progress_json(progress: HarvestProgress) -> Value {
    let (target, plant_identity_id, target_label) = match progress.target {
        HarvestRunTarget::All => ("all", None, "All currently available".to_owned()),
        HarvestRunTarget::Species(plant_identity_id) => (
            "species",
            Some(plant_identity_id.0),
            "Selected species".to_owned(),
        ),
    };
    json!({
        "run_id": progress.run_id.0,
        "target": target,
        "target_label": target_label,
        "plant_identity_id": plant_identity_id,
        "harvested_parts": progress.harvested_parts.into_iter()
            .map(HarvestedPart::as_str)
            .collect::<Vec<_>>(),
        "route": progress.route.into_iter().map(harvest_tree_json).collect::<Vec<_>>(),
        "handled_tree_count": progress.handled_tree_count,
        "harvested_tree_count": progress.harvested_tree_count,
        "done_for_window_tree_count": progress.done_for_window_tree_count,
        "deferred_tree_count": progress.deferred_tree_count,
        "total_tree_count": progress.total_tree_count,
        "next_tree": progress.current_tree.map(harvest_tree_json),
    })
}

fn harvest_tree_json(tree: crate::hexagon::use_cases::start_harvest_run::HarvestTree) -> Value {
    json!({
        "id": tree.id.0,
        "name": tree.name,
        "plant_identity_id": tree.plant_identity_id.0,
        "harvested_parts": tree.harvested_parts.into_iter()
            .map(HarvestedPart::as_str)
            .collect::<Vec<_>>(),
        "longitude": tree.longitude,
        "latitude": tree.latitude,
        "route_rank": tree.route_rank,
        "window_start": tree.period.start.to_string(),
        "window_end": tree.period.end.to_string(),
    })
}

fn parse_harvested_parts(value: &str) -> Option<Vec<HarvestedPart>> {
    let mut parts = value
        .split(',')
        .map(|part| match part {
            "cone" => Some(HarvestedPart::Cone),
            "flower" => Some(HarvestedPart::Flower),
            "fruit" => Some(HarvestedPart::Fruit),
            "leaf" => Some(HarvestedPart::Leaf),
            "nut" => Some(HarvestedPart::Nut),
            "pod" => Some(HarvestedPart::Pod),
            "seed" => Some(HarvestedPart::Seed),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    parts.sort_unstable();
    parts.dedup();
    (!parts.is_empty()).then_some(parts)
}

fn default_harvested_parts() -> Vec<HarvestedPart> {
    vec![HarvestedPart::Fruit]
}

fn default_harvested_parts_query() -> String {
    HarvestedPart::Fruit.as_str().into()
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

fn orchard_harvesting_credential(
    headers: &HeaderMap,
) -> Result<OrchardHarvestingCredential, StatusCode> {
    if let Some(share_token) = headers
        .get("x-orchard-share-token")
        .and_then(|value| value.to_str().ok())
    {
        return Ok(OrchardHarvestingCredential::ShareToken(
            share_token.to_owned(),
        ));
    }
    cookie_value(headers, "orchard_session")
        .map(OrchardHarvestingCredential::OwnerSession)
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn orchard_photography_credential(
    headers: &HeaderMap,
) -> Result<OrchardPhotographyCredential, StatusCode> {
    if let Some(share_token) = headers
        .get("x-orchard-share-token")
        .and_then(|value| value.to_str().ok())
    {
        return Ok(OrchardPhotographyCredential::ShareToken(
            share_token.to_owned(),
        ));
    }
    cookie_value(headers, "orchard_session")
        .map(OrchardPhotographyCredential::OwnerSession)
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

fn authorize_harvesting_access(
    access_control: &mut impl AccessControl,
    orchard_id: OrchardId,
    credential: OrchardHarvestingCredential,
) -> Result<(), StatusCode> {
    authorize_orchard_harvester(
        OrchardHarvestingAccessRequested {
            orchard_id,
            credential,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardHarvestingAccessError::AccessNotFound => StatusCode::NOT_FOUND,
        OrchardHarvestingAccessError::PermissionDenied => StatusCode::FORBIDDEN,
        OrchardHarvestingAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

fn authorize_photography_access(
    access_control: &mut impl AccessControl,
    orchard_id: OrchardId,
    credential: OrchardPhotographyCredential,
) -> Result<(), StatusCode> {
    authorize_orchard_photographer(
        OrchardPhotographyAccessRequested {
            orchard_id,
            credential,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardPhotographyAccessError::AccessNotFound => StatusCode::NOT_FOUND,
        OrchardPhotographyAccessError::PermissionDenied => StatusCode::FORBIDDEN,
        OrchardPhotographyAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
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
    is_excluded_from_watering: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveTreeRequest {
    longitude: f64,
    latitude: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceTreePhotoRequest {
    image_base64: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedTreePhotoRequest {
    full_webp_base64: String,
    thumbnail_webp_base64: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AddTreePhotoRequest {
    Source(SourceTreePhotoRequest),
    Prepared(PreparedTreePhotoRequest),
}

enum DecodedTreePhotoRequest {
    Source(Vec<u8>),
    Prepared {
        full_webp: Vec<u8>,
        thumbnail_webp: Vec<u8>,
    },
}

async fn list_tree_photos_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id,
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        list_tree_photos(
            TreePhotosRequested {
                orchard_id,
                tree_id: TreeId(tree_id),
            },
            &mut *storage,
        )
        .map(|photos| {
            Json(json!({
                "photos": photos.into_iter().map(|photo| json!({
                    "id": photo.id.0,
                    "created_at_unix_seconds": photo.created_at_unix_seconds,
                })).collect::<Vec<_>>()
            }))
        })
        .map_err(|error| match error {
            TreePhotosListError::TreeNotFound => StatusCode::NOT_FOUND,
            TreePhotosListError::PhotosCouldNotBeListed => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn add_tree_photo_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<AddTreePhotoRequest>, JsonRejection>,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    let credential = orchard_photography_credential(&headers)?;
    let Json(request) = request.map_err(|rejection| rejection.status())?;
    let request = match request {
        AddTreePhotoRequest::Source(request) => DecodedTreePhotoRequest::Source(
            STANDARD
                .decode(request.image_base64)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        ),
        AddTreePhotoRequest::Prepared(request) => DecodedTreePhotoRequest::Prepared {
            full_webp: STANDARD
                .decode(request.full_webp_base64)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
            thumbnail_webp: STANDARD
                .decode(request.thumbnail_webp_base64)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        },
    };

    tokio::task::spawn_blocking(move || {
        let orchard_id = OrchardId(orchard_id);
        {
            let mut storage = storage.lock().unwrap();
            authorize_photography_access(&mut *storage, orchard_id, credential)?;
        }
        let (full_webp, thumbnail_webp) = match request {
            DecodedTreePhotoRequest::Source(source) => {
                let prepared = prepare_tree_photo(&source).map_err(|error| match error {
                    TreePhotoPreparationError::ImageTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    TreePhotoPreparationError::InvalidImage
                    | TreePhotoPreparationError::CouldNotEncode => StatusCode::BAD_REQUEST,
                })?;
                (prepared.full_webp, prepared.thumbnail_webp)
            }
            DecodedTreePhotoRequest::Prepared {
                full_webp,
                thumbnail_webp,
            } => (full_webp, thumbnail_webp),
        };
        let mut storage = storage.lock().unwrap();
        add_tree_photo(
            TreePhotoAdded {
                orchard_id,
                tree_id: TreeId(tree_id),
                full_webp,
                thumbnail_webp,
            },
            &mut *storage,
        )
        .map(|()| StatusCode::CREATED)
        .map_err(|error| match error {
            TreePhotoAddError::InvalidWebp => StatusCode::BAD_REQUEST,
            TreePhotoAddError::PhotoTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            TreePhotoAddError::TreeNotFound => StatusCode::NOT_FOUND,
            TreePhotoAddError::PhotoCouldNotBeSaved => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn latest_tree_photo_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    tree_photo_response(
        storage,
        OrchardId(orchard_id),
        TreeId(tree_id),
        headers,
        TreePhotoVariant::Full,
    )
    .await
}

async fn latest_tree_photo_thumbnail_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    tree_photo_response(
        storage,
        OrchardId(orchard_id),
        TreeId(tree_id),
        headers,
        TreePhotoVariant::Thumbnail,
    )
    .await
}

async fn tree_photo_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id, photo_id)): Path<(u64, u64, u64)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    specific_tree_photo_response(
        storage,
        OrchardId(orchard_id),
        TreeId(tree_id),
        TreePhotoId(photo_id),
        headers,
        TreePhotoVariant::Full,
    )
    .await
}

async fn tree_photo_thumbnail_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id, photo_id)): Path<(u64, u64, u64)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    specific_tree_photo_response(
        storage,
        OrchardId(orchard_id),
        TreeId(tree_id),
        TreePhotoId(photo_id),
        headers,
        TreePhotoVariant::Thumbnail,
    )
    .await
}

async fn delete_tree_photo_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id, photo_id)): Path<(u64, u64, u64)>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    let session_token = owner_session_token(&headers)?;
    tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        let orchard_id = OrchardId(orchard_id);
        authorize_owner_access(&mut *storage, orchard_id, session_token)?;
        delete_tree_photo(
            TreePhotoDeletionRequested {
                orchard_id,
                tree_id: TreeId(tree_id),
                photo_id: TreePhotoId(photo_id),
            },
            &mut *storage,
        )
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|error| match error {
            TreePhotoDeleteError::PhotoNotFound => StatusCode::NOT_FOUND,
            TreePhotoDeleteError::PhotoCouldNotBeDeleted => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn tree_photo_response<U>(
    storage: Arc<Mutex<U>>,
    orchard_id: OrchardId,
    tree_id: TreeId,
    headers: HeaderMap,
    variant: TreePhotoVariant,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    let bytes = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id,
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        load_latest_tree_photo(
            LatestTreePhotoRequested {
                orchard_id,
                tree_id,
                variant,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            LatestTreePhotoLoadError::PhotoNotFound => StatusCode::NOT_FOUND,
            LatestTreePhotoLoadError::PhotoCouldNotBeRead => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Response::builder()
        .header(header::CONTENT_TYPE, "image/webp")
        .header(header::CACHE_CONTROL, "no-store")
        .header("Referrer-Policy", "no-referrer")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn specific_tree_photo_response<U>(
    storage: Arc<Mutex<U>>,
    orchard_id: OrchardId,
    tree_id: TreeId,
    photo_id: TreePhotoId,
    headers: HeaderMap,
    variant: TreePhotoVariant,
) -> Result<Response, StatusCode>
where
    U: AccessControl + TreePhotoStorage + Send + 'static,
{
    let credential = orchard_read_credential(&headers)?;
    let bytes = tokio::task::spawn_blocking(move || {
        let mut storage = storage.lock().unwrap();
        authorize_orchard_reader(
            OrchardReadAccessRequested {
                orchard_id,
                credential,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardReadAccessError::AccessNotFound => StatusCode::NOT_FOUND,
            OrchardReadAccessError::AccessCouldNotBeChecked => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        load_tree_photo(
            TreePhotoRequested {
                orchard_id,
                tree_id,
                photo_id,
                variant,
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            TreePhotoLoadError::PhotoNotFound => StatusCode::NOT_FOUND,
            TreePhotoLoadError::PhotoCouldNotBeRead => StatusCode::INTERNAL_SERVER_ERROR,
        })
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Response::builder()
        .header(header::CONTENT_TYPE, "image/webp")
        .header(header::CACHE_CONTROL, "no-store")
        .header("Referrer-Policy", "no-referrer")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
                is_excluded_from_watering: request.is_excluded_from_watering,
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

async fn move_tree_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path((orchard_id, tree_id)): Path<(u64, u64)>,
    headers: HeaderMap,
    request: Result<Json<MoveTreeRequest>, JsonRejection>,
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
        move_orchard_tree(
            OrchardTreeMoveConfirmed {
                orchard_id: OrchardId(orchard_id),
                tree_id: TreeId(tree_id),
                position: GeoPoint {
                    longitude: request.longitude,
                    latitude: request.latitude,
                },
            },
            &mut *storage,
        )
        .map_err(|error| match error {
            OrchardTreeMoveError::InvalidPosition => StatusCode::BAD_REQUEST,
            OrchardTreeMoveError::TreeNotFound => StatusCode::NOT_FOUND,
            OrchardTreeMoveError::TreeCouldNotBeMoved => StatusCode::INTERNAL_SERVER_ERROR,
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
        let has_photo = orchard_tree.has_photo;
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
            is_excluded_from_watering,
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
                "has_photo": has_photo,
                "roles": roles,
                "is_alive": is_alive,
                "is_in_danger": is_in_danger,
                "is_excluded_from_watering": is_excluded_from_watering,
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
