## Why

Synapse hoy es solo una aplicación de escritorio (Tauri): las bóvedas viven en el disco del usuario y no hay forma de sincronizarlas ni de conservar su historial fuera de la máquina. Queremos un servidor propio, separado de la app y muy ligero, que **solo reciba** lo que el cliente ya preparó: los archivos y la información de índice predigerida que el cliente extrae de Tantivy. Sobre esa base el historial de versiones sale casi gratis, porque el almacenamiento se direcciona por contenido (`content_hash`), y la retención de versiones se vuelve el modelo de negocio.

## What Changes

- Nueva carpeta `synapse-server/` en la raíz del repo: un binario Rust independiente, sin dependencia de Tauri ni del frontend, que no se empaqueta en el instalador de escritorio.
- Servicio gRPC (`tonic`, `sync.proto`): `CheckState`, `PushDelta`, `PullState`, `UploadArtifact` y `DownloadArtifact`. Cliente pesado / servidor ligero: el cliente calcula los hashes y el Merkle Tree; el servidor actúa como notario de integridad, guarda la instantánea actual y gestiona la transferencia diferencial.
- Sincronización diferencial: si el `root_hash` del cliente coincide con el del servidor no se transfiere nada; si difiere, el cliente envía solo los nodos modificados o eliminados y sube por *streaming* los contenidos que faltan.
- Almacenamiento direccionado por contenido (CAS) con deduplicación: un mismo contenido se guarda una sola vez aunque aparezca en varias rutas, versiones o bóvedas del mismo tenant. Los archivos grandes se dividen en bloques definidos por contenido para subir y guardar solo los que cambian.
- Estado vivo del árbol (`tree_nodes`) separado del historial (`file_versions`): cada cambio confirmado deja una fila de versión que apunta a su `content_hash`.
- Los índices precomputados por el cliente (segmentos de Tantivy u otras estructuras serializadas) son artefactos binarios de primera clase dentro del árbol (nodos `INDEX`). Un cliente ligero (móvil) puede descargar solo los índices de la última instantánea para buscar en memoria sin bajar los archivos fuente. El servidor no indexa ni parsea notas.
- Políticas de retención por plan (últimas N versiones / N días / ilimitada) y un recolector de basura que borra físicamente los blobs que ya nadie referencia.
- Fuera de alcance en este cambio: el cliente de sincronización dentro de la app de escritorio, la fusión de conflictos, búsqueda en el servidor, interfaz web, facturación y cobro.

## Capabilities

### New Capabilities
- `sync-ingest`: comprobación y consulta del estado, transferencia por streaming de artefactos direccionados por contenido y confirmación atómica de deltas del árbol de una bóveda.
- `sync-versioning`: historial de versiones por archivo, retención por plan y recolección de basura de blobs sin referencias.
- `sync-index-ingest`: índices precomputados por el cliente como artefactos del árbol y su descarga selectiva por clientes ligeros.

### Modified Capabilities
<!-- Ninguna: no existen specs previas y la app de escritorio no cambia en este cambio. -->

## Impact

- Código nuevo: `synapse-server/` (crate propio). `src-tauri/` y `src/` no cambian.
- Build: el repo pasa a tener dos binarios Rust; conviene un workspace de Cargo o, como mínimo, que CI compile y pruebe `synapse-server` por separado. El release de escritorio (`ci/release.py`) no debe incluirlo.
- Dependencias nuevas solo en el servidor: `tonic`/`prost` (gRPC), `axum` para salud y gestión, `sqlx` con SQLite en WAL (PostgreSQL particionado opcional), compresión Zstd, chunking por contenido (FastCDC) y cliente S3 compatible (R2) como backend de blobs opcional.
- Contrato nuevo cliente↔servidor (`sync.proto`) que el futuro cliente de escritorio deberá respetar.
