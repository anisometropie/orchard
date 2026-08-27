use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;

use crate::hexagon::models::{PlantIdentity, Tree};
use crate::hexagon::ports::OrchardUnitOfWork;
use crate::hexagon::use_cases::create_tree::{TreeCreationRequested, create_tree};

#[derive(Deserialize)]
pub struct CreateTreeRequest {
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identity: PlantIdentity,
    pub roles: Vec<String>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
}

pub fn router<U>(orchard_unit_of_work: Arc<Mutex<U>>) -> Router
where
    U: OrchardUnitOfWork + Send + 'static,
{
    Router::new()
        .route("/trees", post(create_tree_handler::<U>))
        .with_state(orchard_unit_of_work)
}

async fn create_tree_handler<U>(
    State(orchard_unit_of_work): State<Arc<Mutex<U>>>,
    Json(request): Json<CreateTreeRequest>,
) -> Result<(StatusCode, Json<Tree>), StatusCode>
where
    U: OrchardUnitOfWork + Send + 'static,
{
    create_tree(
        TreeCreationRequested {
            longitude: request.longitude,
            latitude: request.latitude,
            plant_identity: request.plant_identity,
            roles: request.roles,
            harvest_start_day: request.harvest_start_day,
            harvest_end_day: request.harvest_end_day,
        },
        &mut *orchard_unit_of_work.lock().unwrap(),
    )
    .map(|tree| (StatusCode::CREATED, Json(tree)))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
