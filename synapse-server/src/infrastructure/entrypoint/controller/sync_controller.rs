//! Controller gRPC del servicio `synapse.sync.v1.Sync` y arranque del servidor `tonic`.

use std::future::Future;
use std::pin::Pin;

use tokio::net::TcpListener;
use tokio_stream::Stream;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

use crate::infrastructure::entrypoint::controller::proto;
use crate::infrastructure::entrypoint::controller::proto::sync_server::{Sync, SyncServer};

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

/// Implementación gRPC del servicio `Sync`.
#[derive(Debug, Default)]
pub struct SyncController;

#[tonic::async_trait]
impl Sync for SyncController {
    async fn check_state(
        &self,
        _request: Request<proto::CheckStateRequest>,
    ) -> Result<Response<proto::CheckStateResponse>, Status> {
        Err(Status::unimplemented("CheckState aún no implementado"))
    }

    async fn find_missing(
        &self,
        _request: Request<proto::FindMissingRequest>,
    ) -> Result<Response<proto::FindMissingResponse>, Status> {
        Err(Status::unimplemented("FindMissing aún no implementado"))
    }

    async fn upload_artifact(
        &self,
        _request: Request<Streaming<proto::UploadArtifactRequest>>,
    ) -> Result<Response<proto::UploadArtifactResponse>, Status> {
        Err(Status::unimplemented("UploadArtifact aún no implementado"))
    }

    type DownloadArtifactStream = BoxStream<proto::DownloadArtifactResponse>;

    async fn download_artifact(
        &self,
        _request: Request<proto::DownloadArtifactRequest>,
    ) -> Result<Response<Self::DownloadArtifactStream>, Status> {
        Err(Status::unimplemented(
            "DownloadArtifact aún no implementado",
        ))
    }

    async fn declare_manifest(
        &self,
        _request: Request<proto::DeclareManifestRequest>,
    ) -> Result<Response<proto::DeclareManifestResponse>, Status> {
        Err(Status::unimplemented("DeclareManifest aún no implementado"))
    }

    async fn push_delta(
        &self,
        _request: Request<proto::PushDeltaRequest>,
    ) -> Result<Response<proto::PushDeltaResponse>, Status> {
        Err(Status::unimplemented("PushDelta aún no implementado"))
    }

    type PullStateStream = BoxStream<proto::PullStateResponse>;

    async fn pull_state(
        &self,
        _request: Request<proto::PullStateRequest>,
    ) -> Result<Response<Self::PullStateStream>, Status> {
        Err(Status::unimplemented("PullState aún no implementado"))
    }

    async fn list_versions(
        &self,
        _request: Request<proto::ListVersionsRequest>,
    ) -> Result<Response<proto::ListVersionsResponse>, Status> {
        Err(Status::unimplemented("ListVersions aún no implementado"))
    }

    async fn get_version(
        &self,
        _request: Request<proto::GetVersionRequest>,
    ) -> Result<Response<proto::GetVersionResponse>, Status> {
        Err(Status::unimplemented("GetVersion aún no implementado"))
    }
}

/// Arranca el servidor gRPC (`Sync`, `grpc.health.v1` y `tonic-reflection`) sobre `listener`.
pub async fn serve(
    listener: TcpListener,
    max_message_bytes: usize,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<SyncServer<SyncController>>()
        .await;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let sync_service = SyncServer::new(SyncController)
        .max_decoding_message_size(max_message_bytes)
        .max_encoding_message_size(max_message_bytes);

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(sync_service)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await?;

    Ok(())
}
