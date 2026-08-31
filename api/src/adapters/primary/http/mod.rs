use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::Response,
    routing::{get, patch, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use crate::hexagon::models::{
    AerialOverlayId, AnnualDate, BotanicalTaxon, HarvestScheduleOwner, HarvestedPart,
    InfraspecificRank, MapConfiguration, NamedTaxon, OrchardTree, PlantCultivarId,
    PlantIdentification, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use crate::hexagon::ports::{MapConfigurationStorage, OrchardStorage};
use crate::hexagon::use_cases::change_tree_condition::{
    TreeConditionChangeError, TreeConditionChanged, change_tree_condition,
};
use crate::hexagon::use_cases::create_tree::{TreeCreationRequested, create_tree};
use crate::hexagon::use_cases::list_orchard_trees::list_orchard_trees;
use crate::hexagon::use_cases::load_aerial_overlay_image::{
    AerialOverlayImageLoadError, load_aerial_overlay_image,
};
use crate::hexagon::use_cases::load_map_configuration::{
    MapConfigurationLoadError, load_map_configuration,
};
use crate::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, PlantHarvestWindowsReplaced, PlantHarvestWindowsReplacementError,
    replace_plant_harvest_windows,
};

pub async fn start_http_server<U>(
    orchard_storage: U,
    address: SocketAddr,
) -> Result<RunningHttpServer, std::io::Error>
where
    U: OrchardStorage + MapConfigurationStorage + Send + 'static,
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
    U: OrchardStorage + MapConfigurationStorage + Send + 'static,
{
    Router::new()
        .route("/trees", post(create_tree_handler::<U>))
        .route("/trees/{tree_id}", patch(change_tree_handler::<U>))
        .route(
            "/plant-identities/{plant_identity_id}/harvest-windows",
            put(replace_identity_harvest_windows_handler::<U>),
        )
        .route(
            "/plant-cultivars/{cultivar_id}/harvest-windows",
            put(replace_cultivar_harvest_windows_handler::<U>),
        )
        .route("/trees.geojson", get(list_trees_handler::<U>))
        .route("/map-config", get(map_configuration_handler::<U>))
        .route(
            "/aerial-overlays/{overlay_id}/image",
            get(aerial_overlay_image_handler::<U>),
        )
        .with_state(orchard_storage)
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
) -> Result<Json<Value>, StatusCode>
where
    U: MapConfigurationStorage + Send + 'static,
{
    tokio::task::spawn_blocking(move || load_map_configuration(&mut *storage.lock().unwrap()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|configuration| Json(map_configuration_json(configuration)))
        .map_err(|error| match error {
            MapConfigurationLoadError::ConfigurationNotFound => StatusCode::NOT_FOUND,
            MapConfigurationLoadError::ConfigurationCouldNotBeRead => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

async fn aerial_overlay_image_handler<U>(
    State(storage): State<Arc<Mutex<U>>>,
    Path(overlay_id): Path<u64>,
) -> Result<Response, StatusCode>
where
    U: MapConfigurationStorage + Send + 'static,
{
    let image = tokio::task::spawn_blocking(move || {
        load_aerial_overlay_image(AerialOverlayId(overlay_id), &mut *storage.lock().unwrap())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|error| match error {
        AerialOverlayImageLoadError::ImageNotFound => StatusCode::NOT_FOUND,
        AerialOverlayImageLoadError::ImageCouldNotBeRead => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    Response::builder()
        .header(header::CONTENT_TYPE, image.media_type)
        .header(header::CACHE_CONTROL, "no-store")
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
) -> Result<Json<Value>, StatusCode>
where
    U: OrchardStorage + Send + 'static,
{
    tokio::task::spawn_blocking(move || list_orchard_trees(&mut *orchard_storage.lock().unwrap()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|trees| Json(orchard_geojson(trees)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTreeRequest {
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identity: PlantIdentification,
    pub roles: Vec<String>,
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
    Path(plant_identity_id): Path<u64>,
    Json(request): Json<ReplaceHarvestWindowsRequest>,
) -> StatusCode
where
    U: OrchardStorage + Send + 'static,
{
    replace_harvest_windows_handler(
        orchard_storage,
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(plant_identity_id)),
        request,
    )
    .await
}

async fn replace_cultivar_harvest_windows_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Path(cultivar_id): Path<u64>,
    Json(request): Json<ReplaceHarvestWindowsRequest>,
) -> StatusCode
where
    U: OrchardStorage + Send + 'static,
{
    replace_harvest_windows_handler(
        orchard_storage,
        HarvestScheduleOwner::PlantCultivar(PlantCultivarId(cultivar_id)),
        request,
    )
    .await
}

async fn replace_harvest_windows_handler<U>(
    orchard_storage: Arc<Mutex<U>>,
    owner: HarvestScheduleOwner,
    request: ReplaceHarvestWindowsRequest,
) -> StatusCode
where
    U: OrchardStorage + Send + 'static,
{
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
        replace_plant_harvest_windows(
            PlantHarvestWindowsReplaced {
                owner,
                reference_region,
                windows,
            },
            &mut *orchard_storage.lock().unwrap(),
        )
    })
    .await
    {
        Ok(Ok(())) => StatusCode::NO_CONTENT,
        Ok(Err(
            PlantHarvestWindowsReplacementError::InvalidAnnualDate
            | PlantHarvestWindowsReplacementError::MissingReferenceRegion,
        )) => StatusCode::BAD_REQUEST,
        Ok(Err(PlantHarvestWindowsReplacementError::OwnerNotFound)) => StatusCode::NOT_FOUND,
        Ok(Err(_)) | Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn change_tree_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Path(tree_id): Path<u64>,
    Json(request): Json<ChangeTreeRequest>,
) -> StatusCode
where
    U: OrchardStorage + Send + 'static,
{
    match tokio::task::spawn_blocking(move || {
        change_tree_condition(
            TreeConditionChanged {
                tree_id: TreeId(tree_id),
                is_alive: request.is_alive,
                is_in_danger: request.is_in_danger,
            },
            &mut *orchard_storage.lock().unwrap(),
        )
    })
    .await
    {
        Ok(Ok(())) => StatusCode::NO_CONTENT,
        Ok(Err(TreeConditionChangeError::NoChangesRequested)) => StatusCode::BAD_REQUEST,
        Ok(Err(TreeConditionChangeError::TreeNotFound)) => StatusCode::NOT_FOUND,
        Ok(Err(TreeConditionChangeError::DeadTreeCannotBeInDanger)) => StatusCode::CONFLICT,
        Ok(Err(TreeConditionChangeError::TreeCouldNotBeChanged)) | Err(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn create_tree_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    request: Result<Json<CreateTreeRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Tree>), StatusCode>
where
    U: OrchardStorage + Send + 'static,
{
    let Json(request) = request.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        create_tree(
            TreeCreationRequested {
                longitude: request.longitude,
                latitude: request.latitude,
                plant_identification: request.plant_identity,
                roles: request.roles,
            },
            &mut *orchard_storage.lock().unwrap(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|tree| (StatusCode::CREATED, Json(tree)))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn orchard_geojson(trees: Vec<OrchardTree>) -> Value {
    let features = trees.into_iter().map(|orchard_tree| {
        let tree_id = orchard_tree.id;
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
