//! Synapse Server: servidor de sincronización ligero. Solo recibe lo que el cliente ya preparó.
//!
//! Capas (arquitectura cebolla, ver `docs/arquitectura_cebolla.md`): `domain` ← `application` ←
//! `infrastructure`.

pub mod application;
pub mod domain;
pub mod infrastructure;

use std::future::Future;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::infrastructure::config::Config;
use crate::infrastructure::entrypoint::controller::sync_controller;
use crate::infrastructure::entrypoint::http;

/// Listeners ya enlazados que recibe el servidor (facilita puertos efímeros en pruebas).
pub struct Listeners {
    /// Listener HTTP para salud y gestión ligera.
    pub http: TcpListener,
    /// Listener gRPC para el servicio `Sync`, salud gRPC y reflexión.
    pub grpc: TcpListener,
}

/// Arranca los servicios HTTP y gRPC sobre `listeners` con apagado ordenado común.
pub async fn run(
    config: Config,
    listeners: Listeners,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    config.validate()?;
    tracing::info!(addr = %listeners.http.local_addr()?, "servidor HTTP en marcha");
    tracing::info!(addr = %listeners.grpc.local_addr()?, "servidor gRPC en marcha");

    let (shutdown_tx, mut http_rx) = watch::channel(false);
    let mut grpc_rx = shutdown_tx.subscribe();

    let shutdown_task = tokio::spawn(async move {
        shutdown.await;
        let _ = shutdown_tx.send(true);
    });

    let http_fut = async move {
        http::serve(listeners.http, async move {
            let _ = http_rx.wait_for(|&v| v).await;
        })
        .await
        .map_err(anyhow::Error::from)
    };

    let grpc_fut = sync_controller::serve(listeners.grpc, config.max_message_bytes, async move {
        let _ = grpc_rx.wait_for(|&v| v).await;
    });

    let result = tokio::try_join!(http_fut, grpc_fut);
    shutdown_task.abort();
    result?;
    Ok(())
}
