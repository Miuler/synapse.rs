# Contrato gRPC de Synapse Server (`synapse.sync.v1`)

## Generación de código

El código Rust (servidor, cliente y `FileDescriptorSet` para `tonic-reflection`) se genera automáticamente al compilar
`synapse-server` mediante [`build.rs`](../build.rs) y `tonic-prost-build`, usando el binario `protoc` instalado en el
sistema.

```bash
cd synapse-server
cargo build
```

Los tipos generados viven únicamente dentro de `src/infrastructure/entrypoint/controller/proto.rs` y nunca salen de la
capa de infraestructura.

## Convenciones

- **Autenticación**: todas las operaciones del servicio `synapse.sync.v1.Sync` exigen `authorization: Bearer <ID token de Google>`.
  Los servicios `grpc.health.v1.Health` y `grpc.reflection.v1.ServerReflection` no exigen token.
- **Hashes**: todo `content_hash`, `root_hash`, `base_root_hash`, `new_root_hash` y `chunk_hashes` es **BLAKE3 del contenido sin comprimir en hexadecimal minúscula de 64 caracteres**.
- **Streaming**:
  - `UploadArtifact` y `DownloadArtifact`: el primer mensaje del stream es siempre `header` (`ArtifactHeader`); los siguientes son `data` en trozos recomendados de **256 KiB**.
  - `PullState`: el primer mensaje es `root_hash` de la instantánea vigente; los siguientes son un `node` (`NodeMetadata`) cada uno.

## Tabla de errores (`google.rpc` con `tonic-types`)

| Situación | Código gRPC | Detalle `google.rpc` |
|---|---|---|
| Sin token, token inválido o caducado | `UNAUTHENTICATED` | — |
| Cuenta fuera de la lista permitida | `PERMISSION_DENIED` | — |
| JWKS caducado y Google inalcanzable | `UNAVAILABLE` | — |
| Bóveda inexistente o de otro tenant | `NOT_FOUND` | — (respuesta idéntica en ambos casos) |
| Ruta, hash o mensaje mal formado | `INVALID_ARGUMENT` | `BadRequest` con el campo |
| Delta incoherente | `INVALID_ARGUMENT` | `BadRequest` con la ruta |
| Hash del contenido subido no coincide | `INVALID_ARGUMENT` | `BadRequest` (`content_hash`) |
| Delta o manifiesto con contenido faltante | `FAILED_PRECONDITION` | `PreconditionFailure`, tipo `MISSING_CONTENT`, un hash por fila |
| Raíz declarada no coincide | `FAILED_PRECONDITION` | `ErrorInfo` razón `ROOT_MISMATCH` |
| `base_root_hash` no es el vigente | `ABORTED` | `ErrorInfo` razón `STALE_BASE`, metadato `current_root_hash` |
| Límite superado (tamaño, nodos) | `RESOURCE_EXHAUSTED` | — |
