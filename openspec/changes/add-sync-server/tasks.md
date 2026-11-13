## 1. Estructura del proyecto

- [ ] 1.1 Crear `synapse-server/` (crate binario, capas `domain`/`application`/`infrastructure`/`entrypoint`) y un `Cargo.toml` de workspace en la raíz con `src-tauri` y `synapse-server`; verificar con `cargo build -p synapse-server` y `cargo build -p synapsers`
- [ ] 1.2 Configuración por archivo y variables de entorno (direcciones gRPC y HTTP, base de datos, backend de blobs, umbral de chunking, periodo de gracia) y `GET /healthz` con `axum`; prueba de integración que arranca el servidor y recibe 200
- [ ] 1.3 Comprobar que `ci/release.py` y el workflow de release de escritorio no empaquetan `synapse-server`, y agregar un job de CI que compile y pruebe el servidor

## 2. Contrato, persistencia y almacenamiento

- [ ] 2.1 Escribir `proto/sync.proto` (`CheckState`, `PushDelta`, `PullState`, `UploadArtifact`, `DownloadArtifact`, `FindMissing`, `ListVersions`, `GetVersion`) y generarlo con `tonic-build`; verificar con `cargo build -p synapse-server`. Documentar la regla de cálculo del Merkle Tree junto al proto
- [ ] 2.2 Migraciones `sqlx` para `tenants`, `snapshots`, `tree_nodes`, `file_versions`, `blobs`, `file_chunks`; apertura de SQLite con `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`; prueba que migra una base vacía y lee los PRAGMA aplicados
- [ ] 2.3 Trait `BlobStore` con implementación `LocalFs` (Zstd, rutas por prefijo de hash); pruebas de escritura idempotente y lectura
- [ ] 2.4 Implementación `S3` del `BlobStore` detrás de una feature; prueba contra MinIO o marcada `#[ignore]` si no hay

## 3. Sincronización (`sync-ingest`)

- [ ] 3.1 Interceptor de `tonic` que valida el token e inyecta el `TenantId` en las extensiones; repositorios que solo aceptan `TenantId`; pruebas: sin token → `UNAUTHENTICATED`, bóveda ajena → `NOT_FOUND`
- [ ] 3.2 `CheckState`; pruebas de cliente al día y desactualizado
- [ ] 3.3 `FindMissing`, `UploadArtifact` y `DownloadArtifact` por streaming con verificación BLAKE3; pruebas: hash incorrecto → `INVALID_ARGUMENT`, subida repetida no duplica, contenido igual en dos rutas se guarda una vez, descarga devuelve los bytes originales
- [ ] 3.4 Chunking FastCDC para archivos grandes (manifiesto de bloques); prueba de que editar unos bytes de un archivo de 50 MB solo sube los bloques cambiados y de que un manifiesto con bloques faltantes se rechaza
- [ ] 3.5 `PushDelta` transaccional con `base_root_hash`, recálculo de la raíz y actualización de `snapshots`; pruebas: delta válido, artefacto faltante, raíz inconsistente → `FAILED_PRECONDITION`, base vieja → `ABORTED` sin cambios
- [ ] 3.6 `PullState` por streaming con filtro por `node_type`; prueba de reconciliación tras `ABORTED`
- [ ] 3.7 Documentar el uso del servicio (autenticación, flujo de sincronización, códigos de error) en `synapse-server/README.md`

## 4. Versiones, retención y GC (`sync-versioning`)

- [ ] 4.1 Registrar `file_versions` en cada `PushDelta`; pruebas de historial tras dos ediciones y tras borrar
- [ ] 4.2 `ListVersions` y `GetVersion` (reensamblando bloques); prueba de bytes idénticos para una versión antigua
- [ ] 4.3 Políticas de retención por tenant (N versiones, N días, ilimitada) que nunca borran la versión vigente; pruebas de los escenarios de la spec
- [ ] 4.4 Recolector de basura con periodo de gracia como tarea periódica; pruebas: borra lo no referenciado, conserva artefactos compartidos y subidas sin confirmar

## 5. Índices (`sync-index-ingest`)

- [ ] 5.1 Nodos `INDEX` en `PushDelta` y en `PullState` filtrado; prueba: un cliente que solo pide `INDEX` recibe los segmentos de la última instantánea y no los archivos
- [ ] 5.2 Prueba de deduplicación de un segmento sin cambios entre dos instantáneas y de su recolección cuando se reemplaza y sale de la retención

## 6. Integración

- [ ] 6.1 Prueba de extremo a extremo con un cliente gRPC de prueba en Rust: subir una bóveda con un segmento de Tantivy, editar, mover y borrar archivos, aplicar retención y GC, y verificar instantánea, historial y almacenamiento final
- [ ] 6.2 Entrada en `CHANGELOG.md` («[Sin publicar]») y sección en `ARCHITECTURE.md` describiendo el servidor
