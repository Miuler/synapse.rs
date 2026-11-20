use synapse_server::infrastructure::entrypoint::controller::proto::CheckStateRequest;
use synapse_server::infrastructure::entrypoint::controller::proto::sync_client::SyncClient;
use tonic::Code;
use tonic::transport::Endpoint;
use tonic_health::pb::HealthCheckRequest;
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::health_client::HealthClient;

use crate::support::TestServer;

#[tokio::test]
async fn salud_grpc_responde_serving() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let channel = Endpoint::from_shared(server.grpc_endpoint())?
        .connect()
        .await?;
    let mut client = HealthClient::new(channel);

    let respuesta = client
        .check(HealthCheckRequest {
            service: "synapse.sync.v1.Sync".into(),
        })
        .await?
        .into_inner();

    assert_eq!(respuesta.status, ServingStatus::Serving as i32);
    Ok(())
}

#[tokio::test]
async fn metodo_sin_implementar_responde_unimplemented() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let mut client = SyncClient::connect(server.grpc_endpoint()).await?;

    let err = client
        .check_state(CheckStateRequest {
            vault_id: "01936b3e-0000-7000-8000-000000000000".into(),
            root_hash: "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262".into(),
        })
        .await
        .err()
        .ok_or("se esperaba un error UNIMPLEMENTED")?;

    assert_eq!(err.code(), Code::Unimplemented);
    Ok(())
}
