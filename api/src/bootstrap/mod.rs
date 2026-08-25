use std::sync::{Arc, Mutex};

use tokio::{net::TcpListener, task::JoinHandle};

use crate::adapters::primary::http::router;
use crate::hexagon::ports::TreeRepository;

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

pub async fn start_http_server<R>(tree_repository: R) -> RunningHttpServer
where
    R: TreeRepository + Send + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("the orchard HTTP server should bind an available local port");
    let address = listener
        .local_addr()
        .expect("the orchard HTTP server should report its local address");
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router(Arc::new(Mutex::new(tree_repository))))
            .await
            .expect("the orchard HTTP server should run");
    });

    RunningHttpServer {
        url: format!("http://{address}"),
        server_task,
    }
}
