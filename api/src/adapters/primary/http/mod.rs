use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;
use tokio::{net::TcpListener, task::JoinHandle};

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

pub fn router<U>(orchard_storage: Arc<Mutex<U>>) -> Router
where
    U: OrchardUnitOfWork + Send + 'static,
{
    Router::new()
        .route("/trees", post(create_tree_handler::<U>))
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

pub async fn start_http_server<U>(
    orchard_storage: U,
    address: SocketAddr,
) -> Result<RunningHttpServer, std::io::Error>
where
    U: OrchardUnitOfWork + Send + 'static,
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

async fn create_tree_handler<U>(
    State(orchard_storage): State<Arc<Mutex<U>>>,
    Json(request): Json<CreateTreeRequest>,
) -> Result<(StatusCode, Json<Tree>), StatusCode>
where
    U: OrchardUnitOfWork + Send + 'static,
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
