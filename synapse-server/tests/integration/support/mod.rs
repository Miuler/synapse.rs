//! Arnés compartido para pruebas de integración en el mismo proceso.
#![allow(clippy::expect_used)]

use std::net::SocketAddr;

use synapse_server::Listeners;
use synapse_server::infrastructure::config::{BlobBackend, Config};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Instancia de `synapse-server` ejecutándose en el mismo proceso sobre puertos efímeros.
/// Se apaga ordenadamente al soltarse (`Drop`).
pub struct TestServer {
    pub http_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
    _data_dir: TempDir,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: JoinHandle<anyhow::Result<()>>,
}

impl TestServer {
    /// Arranca el servidor con configuración de prueba y directorio temporal aislado.
    pub async fn start() -> Self {
        let data_dir = tempfile::tempdir().expect("crea directorio temporal de prueba");
        let http_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("enlaza puerto HTTP efímero");
        let http_addr = http_listener
            .local_addr()
            .expect("obtiene dirección HTTP efímera");

        let grpc_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("enlaza puerto gRPC efímero");
        let grpc_addr = grpc_listener
            .local_addr()
            .expect("obtiene dirección gRPC efímera");

        let config = Config {
            http_addr: http_addr.to_string(),
            grpc_addr: grpc_addr.to_string(),
            database_url: "sqlite::memory:".into(),
            blob_backend: BlobBackend::Local {
                data_dir: data_dir.path().to_path_buf(),
            },
            google_client_id: "test-client".into(),
            ..Config::default()
        };

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            synapse_server::run(
                config,
                Listeners {
                    http: http_listener,
                    grpc: grpc_listener,
                },
                async {
                    let _ = shutdown_rx.await;
                },
            )
            .await
        });

        Self {
            http_addr,
            grpc_addr,
            _data_dir: data_dir,
            shutdown_tx: Some(shutdown_tx),
            handle,
        }
    }

    /// URL base HTTP del servidor de prueba.
    pub fn http_url(&self, path: &str) -> String {
        format!("http://{}{}", self.http_addr, path)
    }

    /// Endpoint gRPC del servidor de prueba.
    pub fn grpc_endpoint(&self) -> String {
        format!("http://{}", self.grpc_addr)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.handle.abort();
    }
}
