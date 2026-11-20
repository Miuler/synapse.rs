//! Punto de entrada del binario `synapse-server`.

use anyhow::Result;
use clap::Parser;
use synapse_server::Listeners;
use synapse_server::infrastructure::config::{Cli, Config};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.config.as_deref())?;
    config.validate()?;
    init_tracing(&config.log_format);

    let http = TcpListener::bind(&config.http_addr).await?;
    let grpc = TcpListener::bind(&config.grpc_addr).await?;
    synapse_server::run(config, Listeners { http, grpc }, shutdown_signal()).await
}

fn init_tracing(format: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if format == "json" {
        builder.json().init();
    } else {
        builder.init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("apagando");
}
