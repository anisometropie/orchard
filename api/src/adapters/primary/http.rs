use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;

use crate::hexagon::models::Tree;
use crate::hexagon::ports::TreeRepository;
use crate::hexagon::use_cases::create_tree::{TreeCreationRequested, create_tree};

#[derive(Deserialize)]
pub struct CreateTreeRequest {
    pub longitude: f64,
    pub latitude: f64,
    pub name: String,
    pub latin_name: Option<String>,
    pub roles: Vec<String>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
}

pub fn router<R>(tree_repository: Arc<Mutex<R>>) -> Router
where
    R: TreeRepository + Send + 'static,
{
    Router::new()
        .route("/trees", post(create_tree_handler::<R>))
        .with_state(tree_repository)
}

async fn create_tree_handler<R>(
    State(tree_repository): State<Arc<Mutex<R>>>,
    Json(request): Json<CreateTreeRequest>,
) -> (StatusCode, Json<Tree>)
where
    R: TreeRepository + Send + 'static,
{
    let tree = create_tree(
        TreeCreationRequested {
            longitude: request.longitude,
            latitude: request.latitude,
            name: request.name,
            latin_name: request.latin_name,
            roles: request.roles,
            harvest_start_day: request.harvest_start_day,
            harvest_end_day: request.harvest_end_day,
        },
        &mut *tree_repository.lock().unwrap(),
    );
    (StatusCode::CREATED, Json(tree))
}
