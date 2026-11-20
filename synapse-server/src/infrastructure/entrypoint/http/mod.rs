//! Servidor HTTP con `axum`: salud y gestión ligera. Los casos de uso de negocio van por gRPC.

use std::future::Future;

use axum::{Router, http::StatusCode, routing::get};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

/// Rutas del servidor; se separa de `serve` para probarlas sin abrir un puerto.
pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .layer(TraceLayer::new_for_http())
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn serve(
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown)
        .await
}
