use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use crate::hexagon::models::{
    BotanicalTaxon, InfraspecificRank, NamedTaxon, OrchardTree, PlantIdentity, Tree, TreeId,
};
use crate::hexagon::ports::OrchardStorage;
use crate::hexagon::use_cases::change_tree_condition::{
    TreeConditionChangeError, TreeConditionChanged, change_tree_condition,
};
use crate::hexagon::use_cases::create_tree::{TreeCreationRequested, create_tree};
use crate::hexagon::use_cases::list_orchard_trees::list_orchard_trees;

pub async fn start_http_server<U>(
    orchard_storage: U,
    address: SocketAddr,
) -> Result<RunningHttpServer, std::io::Error>
where
    U: OrchardStorage + Send + 'static,
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
    U: OrchardStorage + Send + 'static,
{
    Router::new()
        .route("/trees", post(create_tree_handler::<U>))
        .route("/trees/{tree_id}", patch(change_tree_handler::<U>))
        .route("/trees.geojson", get(list_trees_handler::<U>))
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
pub struct CreateTreeRequest {
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identity: PlantIdentity,
    pub roles: Vec<String>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
}

#[derive(Deserialize)]
struct ChangeTreeRequest {
    is_alive: Option<bool>,
    is_in_danger: Option<bool>,
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
    Json(request): Json<CreateTreeRequest>,
) -> Result<(StatusCode, Json<Tree>), StatusCode>
where
    U: OrchardStorage + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        create_tree(
            TreeCreationRequested {
                longitude: request.longitude,
                latitude: request.latitude,
                plant_identity: request.plant_identity,
                roles: request.roles,
                harvest_start_day: request.harvest_start_day,
                harvest_end_day: request.harvest_end_day,
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
        let Tree {
            legacy_source,
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
        let name = legacy_source
            .as_ref()
            .map(|source| source.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| orchard_tree.plant_identity.common_name.clone());
        let latin_name = legacy_source
            .as_ref()
            .map(|source| source.latin_name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| botanical_name(&orchard_tree.plant_identity));

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

fn botanical_name(plant_identity: &PlantIdentity) -> String {
    let mut name = match &plant_identity.botanical_taxon {
        BotanicalTaxon::Named(taxon) => named_taxon(taxon),
        BotanicalTaxon::HybridFormula { parents } => {
            format!(
                "{} × {}",
                named_taxon(&parents[0]),
                named_taxon(&parents[1])
            )
        }
    };
    if let Some(cultivar) = &plant_identity.cultivar {
        name.push_str(&format!(" ‘{cultivar}’"));
    }
    name
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
