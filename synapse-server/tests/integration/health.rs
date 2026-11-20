use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::support::TestServer;

struct ServidorSubproceso(Child);

impl Drop for ServidorSubproceso {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn healthz_responde_200() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let respuesta = reqwest::get(server.http_url("/healthz")).await?;
    assert_eq!(respuesta.status(), 200);
    Ok(())
}

/// Escenario de la spec `sync-ingest`: «Arranque sin la app de escritorio».
#[tokio::test]
async fn arranca_el_binario_y_responde_salud() -> Result<(), Box<dyn std::error::Error>> {
    let puerto_http = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let puerto_grpc = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let _servidor = ServidorSubproceso(
        Command::new(env!("CARGO_BIN_EXE_synapse-server"))
            .env("SYNAPSE_HTTP_ADDR", format!("127.0.0.1:{puerto_http}"))
            .env("SYNAPSE_GRPC_ADDR", format!("127.0.0.1:{puerto_grpc}"))
            .env("SYNAPSE_GOOGLE_CLIENT_ID", "test-client")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );

    let url = format!("http://127.0.0.1:{puerto_http}/healthz");
    let limite = Instant::now() + Duration::from_secs(10);
    loop {
        match reqwest::get(&url).await {
            Ok(respuesta) => {
                assert_eq!(respuesta.status(), 200);
                return Ok(());
            }
            Err(e) if Instant::now() > limite => return Err(e.into()),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}
