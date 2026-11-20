# P05 — Persistencia SQLite y almacén de contenidos

**Tareas OpenSpec:** 2.3, 2.4. **Depende de:** P01 y P04 (usa `ContentHash`, `VaultPath`, `NodeType`).

## Objetivo

### SQLite (2.3)

1. Dependencias: `sqlx` con `runtime-tokio`, `sqlite`, `migrate`, `macros`, `uuid`, `chrono` (sin TLS: no hay
   PostgreSQL en este cambio); `uuid` (v7), `chrono`.
2. Migraciones en `synapse-server/migrations/`, solo de subida (`sqlx migrate add <nombre>`, sin `-r`; nunca se edita una
   migración ya commiteada). Esquema exacto del `design.md` («Persistencia»), con:
   - `tenants`: `tenant_id TEXT PRIMARY KEY` (UUID v7), `google_sub TEXT NOT NULL UNIQUE`,
     `retention_policy TEXT NOT NULL`, `created_at`.
   - `snapshots`: PK `(tenant_id, vault_id)`, `vault_id` `UNIQUE`, FK a `tenants`.
   - `tree_nodes`: PK `(tenant_id, vault_id, path)`, `CHECK (node_type IN ('FILE','DIR','INDEX'))`, FK a `snapshots`;
     índice que permita listar hijos de un directorio y filtrar por `node_type`.
   - `file_versions`: PK `version_id`, índice `(tenant_id, vault_id, path, created_at DESC)`,
     `CHECK (op IN ('CREATE','MODIFY','DELETE','MOVE'))`, `moved_from` nulo.
   - `blobs`: PK `(tenant_id, content_hash)`, `chunked INTEGER NOT NULL CHECK (chunked IN (0,1))`, `created_at`,
     `last_seen_at` (índice para el GC).
   - `file_chunks`: PK `(tenant_id, content_hash, idx)`, índice `(tenant_id, chunk_hash)` (el GC pregunta «¿algún
     archivo usa este bloque?»), FK a `blobs` para el archivo y el bloque.
   - Fechas como texto ISO 8601 UTC o enteros de milisegundos: elige uno, úsalo en todo el esquema y dilo.
3. `infrastructure/repository/sqlite/pool.rs`: abre el pool con `SqliteConnectOptions` (`journal_mode(Wal)`,
   `synchronous(Normal)`, `foreign_keys(true)`, `busy_timeout(5s)`, `create_if_missing(true)`), tamaño acotado (p. ej.
   8; para `:memory:` usa 1 conexión o `sqlite::memory:` con caché compartida, y explícalo), y ejecuta
   `sqlx::migrate!()` al arrancar. El arranque (`run`) abre la base y la comprobación `/healthz` hace un `SELECT 1`.
4. Contratos de repository en `domain/` (traits) **solo con los métodos que usarán P06–P10**; no implementes casos de
   uso aquí. Mínimo: `TenantRepository` (buscar por `google_sub`, crear), `SnapshotRepository` (buscar bóveda por
   `vault_id` con su dueño), `BlobRepository` (existe, insertar, `touch_last_seen`, listar faltantes de una lista).
   Los métodos de `tree_nodes`/`file_versions` llegan en P09–P10. Todos reciben `&TenantId` salvo la búsqueda de
   tenant por `sub` y la de dueño de bóveda.
   - Transacciones: define cómo un caso de uso abre una «unidad de trabajo» sin conocer `sqlx` (p. ej. un trait
     `UnitOfWork` con `begin()` que devuelve un objeto con los repositorios transaccionales y `commit()`), y
     documéntalo: P09 lo necesita para `PushDelta`.
5. Consultas con `query!`/`query_as!`; genera y versiona `.sqlx/` (ver `00_contexto.md`).

### Almacén de contenidos (2.4)

1. Contrato `ContentRepository` en `domain/content/content_repository.rs` con escritura y lectura en **streaming**
   (no `Vec<u8>` enteros): p. ej. `begin_write(tenant, expected: ContentHash, size) -> PendingWrite` con
   `write_chunk`, `finish() -> Result<StoredContent, ContentError>` (verifica BLAKE3 y tamaño; si no coinciden, borra el
   temporal y devuelve `HashMismatch`), `abort()`; `open_read(tenant, hash) -> stream de bytes descomprimidos`;
   `exists`; `delete`. Los tipos de stream no deben arrastrar `object_store` al dominio (usa `futures::Stream` de
   `Bytes` o un trait propio; si eliges `bytes`/`futures`, añádelos a las permitidas del test de arquitectura con
   comentario).
2. `ContentRepositoryImpl` en `infrastructure/repository/object_store/` con `object_store::local::LocalFileSystem`:
   - Clave definitiva `<tenant_id>/ab/cd/<hash>.zst`; temporal `tmp/<uuid>`; compresión Zstd nivel 3 en streaming
     (crate `zstd` o `async-compression` con `zstd` y `tokio`: elige uno y justifícalo); BLAKE3 incremental sobre los
     bytes **sin comprimir**.
   - Al verificar: `put`/multipart a la clave definitiva (o `rename` si el backend lo permite) y borrar el temporal.
     Si la clave definitiva ya existe, no la reescribas (idempotente).
   - `stored_size` = bytes comprimidos, para la fila de `blobs`.
   - Al arrancar, limpia temporales con más de 1 h (subidas cortadas por un reinicio).

## Pruebas exigidas

- `migraciones::migra_base_vacia_y_aplica_pragmas` (archivo temporal, no memoria: WAL no aplica en memoria): lee
  `journal_mode`, `synchronous`, `foreign_keys` y las tablas creadas.
- `migraciones::vault_id_es_unico_entre_tenants` (insertar la misma bóveda para dos tenants falla).
- Repositorios: crear/buscar tenant; `google_sub` duplicado falla; faltantes de una lista; `touch_last_seen`.
- Contenidos: escritura y lectura devuelven los mismos bytes (incluido un archivo de 0 bytes y uno de 10 MB);
  escritura repetida idempotente (un solo objeto); hash erróneo no deja objeto ni temporal; tamaño erróneo igual;
  el mismo hash en dos tenants son dos objetos separados y borrar uno no afecta al otro; `proptest` de ida y vuelta con
  contenidos aleatorios pequeños.

## Fuera de alcance

S3/R2 (es P12), casos de uso, gRPC.

## Hecho cuando

Verificación en verde, también con `SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings`; `.sqlx/` versionado;
2.3 y 2.4 marcadas; CHANGELOG.
