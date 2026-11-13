## Context

Ver proposal.md (Why). Hoy el repo tiene un solo binario Rust, `src-tauri/` (arquitectura cebolla: `domain`, `application`, `infrastructure`), y el frontend Svelte en `src/`. El cliente ya mantiene un índice Tantivy local. No existe código de sincronización ni un formato de árbol/manifiesto; este cambio define el lado servidor y el contrato, no el cliente.

## Goals / Non-Goals

**Goals:**
- Servidor que solo recibe y guarda: blobs, deltas del árbol e información de índice predigerida.
- Consumo de recursos bajo: un proceso, SQLite y disco local bastan para un tenant pequeño.
- Mismo modelo de datos con PostgreSQL y S3/R2 para producción, sin cambiar el contrato.
- Clientes ligeros (móviles) que solo descargan los índices para buscar en memoria.

**Non-Goals:**
- Cliente de sincronización en la app de escritorio, descarga a otros dispositivos y fusión de conflictos.
- Búsqueda en el servidor: los índices solo se almacenan y se sirven.
- Cifrado de extremo a extremo, facturación y panel web.

## Decisions

**Carpeta y build.** `synapse-server/` en la raíz, crate binario propio con arquitectura cebolla como `src-tauri` (`domain` / `application` / `infrastructure` / `entrypoint`). Se agrega un `Cargo.toml` de workspace en la raíz con `src-tauri` y `synapse-server` como miembros para compartir `Cargo.lock` y caché de `target/`. Alternativa descartada: un crate `synapse-sync-proto` compartido desde ya; se extrae cuando exista el cliente, para no acoplar `src-tauri` antes de tiempo.

**Transporte: gRPC con `tonic` (HTTP/2) y contrato en `synapse-server/proto/sync.proto` (proto3).** Servicio `Sync`:
- `CheckState(StateRequest) → StateResponse`: compara el `root_hash` del cliente con el vigente.
- `PushDelta(DeltaManifest) → SyncAck`: nodos nuevos/modificados/eliminados, `base_root_hash` y `new_root_hash`.
- `PullState(PullRequest) → stream NodeMetadata`: manifiesto de la instantánea vigente, con filtro opcional por `node_type` (clientes ligeros).
- `UploadArtifact(stream ArtifactChunk) → UploadAck` y `DownloadArtifact(ArtifactRequest) → stream ArtifactChunk`: el primer mensaje del stream lleva el `content_hash` y el tamaño; los siguientes, los bytes.
- `FindMissing(HashList) → HashList`: añadido a la propuesta original para no subir contenido que el servidor ya tiene.
- Versiones: `ListVersions` y `GetVersion`, ver `sync-versioning`.

`axum` solo para `/healthz` y gestión ligera. Alternativa descartada: REST con JSON; gRPC da contratos estrictos y streaming de binarios grandes sin trabajo extra.

**Autenticación.** Un interceptor de `tonic` valida el token (`authorization: Bearer`), resuelve el `tenant_id` y lo inyecta en las extensiones del `Request`. Los repositorios reciben un `TenantId` que solo puede construir el interceptor, así ninguna consulta SQL usa un tenant sin validar.

**Bóvedas.** La propuesta original guarda una sola instantánea por tenant. Synapse maneja varias bóvedas, así que la clave de la instantánea es `(tenant_id, vault_id)`; un tenant con una bóveda se comporta igual que el diseño original.

**Hash: BLAKE3 en hex (64 caracteres).** Rápido y con buen soporte en Rust. El `content_hash` siempre es del contenido sin comprimir, así la compresión no altera la identidad.

**Chunking: FastCDC** (crate `fastcdc`) para archivos > 4 MiB (configurable), bloques de ~1 MiB promedio. El manifiesto es la lista ordenada de hashes de bloque; se guarda como fila en `file_chunks`.

**Árbol Merkle (calculado en el cliente).** El cliente calcula hashes por archivo y por directorio y el `root_hash`. El servidor guarda los nodos `DIR` que envía, recalcula la raíz con la misma regla (BLAKE3 de los hijos ordenados `nombre\0tipo\0hash`) y rechaza el delta si no coincide: es el notario, no el autor. La regla exacta se documenta junto a `sync.proto` para que cliente y servidor la compartan.

**Persistencia: `sqlx` con SQLite en WAL por defecto y PostgreSQL para escala.** Al abrir SQLite se ejecuta siempre `PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;`, con un pool de conexiones acotado. En PostgreSQL las tablas grandes se particionan con `PARTITION BY HASH (tenant_id)`. Todas las tablas llevan `tenant_id`:
- `tenants(tenant_id, token_hash, retention_policy)`
- `snapshots(tenant_id, vault_id, root_hash, updated_at)` — puntero a la instantánea vigente
- `tree_nodes(tenant_id, vault_id, path, node_type FILE|DIR|INDEX, content_hash, size_bytes, storage_uri)` — estado plano vivo
- `file_versions(tenant_id, vault_id, path, version_id, node_type, content_hash, size_bytes, op, created_at)` — historial, solo punteros
- `blobs(tenant_id, content_hash, size_bytes, stored_size, chunked, created_at)` y `file_chunks(tenant_id, content_hash, idx, chunk_hash)`

Cada `PushDelta` se aplica en una sola transacción (`BEGIN … COMMIT`): comprueba `base_root_hash`, inserta `file_versions`, actualiza `tree_nodes`, verifica la raíz y sobrescribe `snapshots.root_hash`.

**Blobs: trait `BlobStore`** con implementaciones `LocalFs` (`<data>/blobs/ab/cd/<hash>.zst`) y `S3` (R2 sin coste de egress). Siempre Zstd.

**Índices.** Los segmentos de Tantivy (u otras estructuras serializadas) son nodos `INDEX` del árbol y artefactos opacos en el CAS; el servidor no conoce su formato. Un cliente ligero hace `PullState` filtrado por `INDEX` y luego `DownloadArtifact`.

**Historial.** La propuesta original declara el historial efímero (solo la última instantánea). Se adopta la evolución posterior: el estado vivo sigue en `tree_nodes`/`snapshots` y se agrega `file_versions` con punteros, cuyo tamaño es despreciable porque el contenido ya está deduplicado. Un plan sin historial equivale a retención «0 versiones extra».

**Retención y GC.** Tarea `tokio` periódica: primero aplica la retención por tenant sobre `file_versions` (respetando la versión vigente), luego marca y barre: contenidos referenciados por `tree_nodes`, `file_versions` y `file_chunks`; se borran los no referenciados con `created_at` anterior al periodo de gracia (24 h por defecto).

## Risks / Trade-offs

- [Raíz Merkle vieja en clientes con muchas ediciones] → el cliente reconcilia con `GET /tree` y reintenta; los deltas son pequeños.
- [GC borra un blob justo antes de que se confirme] → periodo de gracia y verificación de existencia dentro de la transacción del delta.
- [SQLite con muchos tenants] → misma capa `sqlx`; se cambia a PostgreSQL por configuración.
- [El formato de los segmentos cambia con la versión de Tantivy] → el servidor los trata como opacos; el cliente decide si puede usarlos y vuelve a subir si no.
- [SQLite con escrituras concurrentes] → WAL y pool acotado; PostgreSQL particionado cuando haga falta.
- [Contenido de usuarios en claro en el servidor] → TLS obligatorio en despliegue; cifrado de extremo a extremo queda como trabajo futuro.

## Migration Plan

Servicio nuevo, sin datos previos. Migraciones `sqlx` versionadas dentro de `synapse-server/migrations/`. Despliegue como binario único o imagen Docker; revertir es apagar el servicio.

## Open Questions

- Cómo se emiten los tokens de tenant (CLI de administración vs. panel) y su rotación.
- Valores concretos de retención por plan comercial.
