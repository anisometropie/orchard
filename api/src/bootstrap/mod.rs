use std::sync::{Arc, Mutex};

use tokio::{net::TcpListener, task::JoinHandle};

use crate::adapters::primary::geojson_legacy_orchard_import::GeoJsonLegacyOrchardImportError;
use crate::adapters::primary::http::router;
use crate::adapters::primary::import_legacy_geojson_file;
use crate::adapters::primary::legacy_orchard_import_cli::LegacyOrchardImportCommand;
use crate::adapters::secondary::PostgresOrchardStorage;
use crate::hexagon::ports::OrchardUnitOfWork;

#[derive(Debug)]
pub enum LegacyOrchardImportCommandError {
    CouldNotConnectToDatabase,
    CouldNotImportOrchard(GeoJsonLegacyOrchardImportError),
}

pub fn import_legacy_orchard(
    command: LegacyOrchardImportCommand,
) -> Result<usize, LegacyOrchardImportCommandError> {
    let mut orchard_unit_of_work = PostgresOrchardStorage::connect(&command.database_url)
        .map_err(|_| LegacyOrchardImportCommandError::CouldNotConnectToDatabase)?;
    import_legacy_geojson_file(&command.geojson_path, &mut orchard_unit_of_work)
        .map_err(LegacyOrchardImportCommandError::CouldNotImportOrchard)
}

pub struct RunningHttpServer {
    url: String,
    server_task: JoinHandle<()>,
}

impl RunningHttpServer {
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for RunningHttpServer {
    fn drop(&mut self) {
        self.server_task.abort();
    }
}

pub async fn start_http_server<U>(orchard_unit_of_work: U) -> RunningHttpServer
where
    U: OrchardUnitOfWork + Send + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("the orchard HTTP server should bind an available local port");
    let address = listener
        .local_addr()
        .expect("the orchard HTTP server should report its local address");
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router(Arc::new(Mutex::new(orchard_unit_of_work))))
            .await
            .expect("the orchard HTTP server should run");
    });

    RunningHttpServer {
        url: format!("http://{address}"),
        server_task,
    }
}
