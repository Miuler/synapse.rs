# P03 — Contrato gRPC y servidor tonic

**Tareas OpenSpec:** 2.1. **Depende de:** P01.

## Objetivo

1. `synapse-server/proto/synapse/sync/v1/sync.proto` con el contrato de abajo. Es la propuesta de referencia: respeta
   nombres, números de campo y semántica. Si al implementar ves que falta o sobra algo, **pregunta antes** (es el
   contrato que respetará el futuro cliente).
2. Generación en `build.rs`. Comprueba en docs.rs qué crates usa la versión actual de `tonic`: desde `tonic` 0.14 la
   generación con `prost` está en `tonic-prost-build` (y el runtime en `tonic-prost`), no en `tonic-build`. Genera
   servidor y cliente (el cliente lo usan las pruebas). Genera también el `FileDescriptorSet` para
   `tonic-reflection`. Usa el `protoc` del sistema (en CI se instala `protobuf-compiler`).
   - Si la API generada lo requiere, añade `prost-types` para `google.protobuf.Timestamp`.
   - Los tipos generados se incluyen en `infrastructure/entrypoint/controller/proto.rs` (`tonic::include_proto!`),
     con `#[allow(clippy::all, clippy::pedantic)]` solo sobre ese módulo.
3. Servidor gRPC en `grpc_addr` dentro de `run` (P01), junto al HTTP: `tonic-health` (servicio `synapse.sync.v1.Sync`
   en `SERVING`), `tonic-reflection`, y el servicio `Sync` con un controller `SyncController` cuyos métodos devuelven
   `Status::unimplemented` por ahora. Límites de mensaje desde `max_message_bytes`
   (`max_decoding_message_size`/`max_encoding_message_size`). Apagado ordenado común a HTTP y gRPC.
4. Módulo de errores del controller (`infrastructure/entrypoint/controller/status.rs`): una función que traduce el
   error de aplicación a `tonic::Status` siguiendo la tabla del `design.md`, con detalles `google.rpc` mediante el
   crate `tonic-types` (`ErrorDetails`, `BadRequest`, `PreconditionFailure`, `ErrorInfo`). Hoy solo tendrá las
   variantes que existan; deja la estructura para que P05–P10 añadan las suyas **ahí y en ningún otro sitio**.
5. `synapse-server/proto/README.md`: cómo se genera, convenciones (hex minúscula de 64 caracteres para todo hash,
   primer mensaje de los streams = cabecera) y la tabla de errores.

## Contrato de referencia

```proto
syntax = "proto3";

package synapse.sync.v1;

import "google/protobuf/timestamp.proto";

// Todas las operaciones exigen `authorization: Bearer <ID token de Google>`.
// Todos los hashes son BLAKE3 del contenido sin comprimir, en hex minúscula de 64 caracteres.
service Sync {
  rpc CheckState(CheckStateRequest) returns (CheckStateResponse);
  rpc FindMissing(FindMissingRequest) returns (FindMissingResponse);
  // Primer mensaje: `header`; los siguientes: `data` (trozos de ~256 KiB).
  rpc UploadArtifact(stream UploadArtifactRequest) returns (UploadArtifactResponse);
  // Primer mensaje de la respuesta: `header`; los siguientes: `data`.
  rpc DownloadArtifact(DownloadArtifactRequest) returns (stream DownloadArtifactResponse);
  rpc DeclareManifest(DeclareManifestRequest) returns (DeclareManifestResponse);
  rpc PushDelta(PushDeltaRequest) returns (PushDeltaResponse);
  // Primer mensaje: `root_hash` de la instantánea; los siguientes: un `node` cada uno.
  rpc PullState(PullStateRequest) returns (stream PullStateResponse);
  rpc ListVersions(ListVersionsRequest) returns (ListVersionsResponse);
  rpc GetVersion(GetVersionRequest) returns (GetVersionResponse);
}

enum NodeType {
  NODE_TYPE_UNSPECIFIED = 0;
  NODE_TYPE_FILE = 1;
  NODE_TYPE_DIR = 2;
  NODE_TYPE_INDEX = 3;
}

enum VersionOp {
  VERSION_OP_UNSPECIFIED = 0;
  VERSION_OP_CREATE = 1;
  VERSION_OP_MODIFY = 2;
  VERSION_OP_DELETE = 3;
  VERSION_OP_MOVE = 4;
}

message CheckStateRequest {
  string vault_id = 1;  // UUID v7 generado por el cliente
  string root_hash = 2; // raíz que conoce el cliente
}

message CheckStateResponse {
  bool in_sync = 1;
  string current_root_hash = 2;
}

message FindMissingRequest {
  repeated string content_hashes = 1; // como máximo 10 000
}

message FindMissingResponse {
  repeated string missing_hashes = 1; // subconjunto de la petición, mismo orden
}

message ArtifactHeader {
  string content_hash = 1;
  uint64 size_bytes = 2; // tamaño sin comprimir
}

message UploadArtifactRequest {
  oneof payload {
    ArtifactHeader header = 1;
    bytes data = 2;
  }
}

message UploadArtifactResponse {
  string content_hash = 1;
  bool already_present = 2;
}

message DownloadArtifactRequest {
  string content_hash = 1;
}

message DownloadArtifactResponse {
  oneof payload {
    ArtifactHeader header = 1;
    bytes data = 2;
  }
}

message DeclareManifestRequest {
  string content_hash = 1;           // hash del archivo completo
  uint64 size_bytes = 2;             // tamaño del archivo completo
  repeated string chunk_hashes = 3;  // en orden; cada bloque ya subido con UploadArtifact
}

message DeclareManifestResponse {
  string content_hash = 1;
}

message NodeUpsert {
  string path = 1;
  NodeType node_type = 2;            // FILE o INDEX; DIR solo para crear un directorio vacío
  string content_hash = 3;           // vacío en DIR
  uint64 size_bytes = 4;             // 0 en DIR
  optional string moved_from = 5;    // ruta de origen si es un movimiento
}

message PushDeltaRequest {
  string vault_id = 1;
  string base_root_hash = 2;         // EMPTY_ROOT en la primera subida
  string new_root_hash = 3;
  repeated NodeUpsert upserts = 4;
  repeated string deleted_paths = 5; // FILE, INDEX o DIR (borra el subárbol)
}

message PushDeltaResponse {
  string root_hash = 1;
}

message PullStateRequest {
  string vault_id = 1;
  repeated NodeType node_types = 2;  // vacío = todos
}

message NodeMetadata {
  string path = 1;
  NodeType node_type = 2;
  string content_hash = 3;           // en DIR, el hash calculado por el servidor
  uint64 size_bytes = 4;
}

message PullStateResponse {
  oneof payload {
    string root_hash = 1;
    NodeMetadata node = 2;
  }
}

message VersionMetadata {
  string version_id = 1;             // UUID v7
  string path = 2;
  NodeType node_type = 3;
  VersionOp op = 4;
  string content_hash = 5;           // en DELETE, el último que tuvo la ruta
  uint64 size_bytes = 6;
  optional string moved_from = 7;
  google.protobuf.Timestamp created_at = 8;
}

message ListVersionsRequest {
  string vault_id = 1;
  string path = 2;
  uint32 page_size = 3;              // 0 = 100; máximo 1000
  string page_token = 4;
}

message ListVersionsResponse {
  repeated VersionMetadata versions = 1; // de la más reciente a la más antigua
  string next_page_token = 2;            // vacío = no hay más
}

message GetVersionRequest {
  string vault_id = 1;
  string version_id = 2;
}

message GetVersionResponse {
  VersionMetadata version = 1;           // el contenido se descarga con DownloadArtifact(content_hash)
}
```

## Pruebas exigidas

- Integración `grpc::salud_grpc_responde_serving` con el cliente de `tonic-health`.
- Integración `grpc::metodo_sin_implementar_responde_unimplemented` (con el cliente generado).
- Unitaria del mapeo de errores: cada variante existente produce el código esperado.

## Fuera de alcance

Autenticación (P06) y la lógica de cada método.

## Hecho cuando

`cargo build` genera el código, verificación en verde, `grpcurl -plaintext localhost:50051 list` mostraría el servicio
(no hace falta ejecutarlo si `grpcurl` no está instalado), 2.1 marcada, CHANGELOG.
