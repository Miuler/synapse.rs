## Context

Ver proposal.md (Why). Hoy el repo tiene un solo binario Rust, `src-tauri/` (arquitectura cebolla: `domain`,
`application`, `infrastructure`), y el frontend Svelte en `src/`. El cliente ya mantiene un índice Tantivy local. No
existe código de sincronización ni un formato de árbol/manifiesto; este cambio define el lado servidor y el contrato, no
el cliente.

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

**Carpeta y build.** `synapse-server/` en la raíz, crate binario propio con arquitectura cebolla como `src-tauri`
(`domain` / `application` / `infrastructure`, con los controllers gRPC en `infrastructure/entrypoint/controller/`). La organización general del repo (mover lo actual a
`synapsers-desktop/`, crear `synapsers-app/`) se decide y ejecuta fuera de este cambio, como una tarea aparte, porque no
altera comportamiento. Este cambio solo exige que `synapse-server` sea un crate independiente que compile con
`cargo build` desde su carpeta y no dependa de Tauri; si más adelante el repo adopta un workspace de Cargo, el crate se
incorpora sin cambios. Alternativa descartada: un crate `synapse-sync-proto` compartido desde ya; se extrae cuando
exista el cliente de sincronización en `synapsers`, para no acoplar `src-tauri` antes de tiempo.

**Transporte: gRPC con `tonic` (HTTP/2) y contrato en `synapse-server/proto/sync.proto` (proto3).** Servicio `Sync`:

- `CheckState(StateRequest) → StateResponse`: compara el `root_hash` del cliente con el vigente.
- `PushDelta(DeltaManifest) → SyncAck`: nodos nuevos/modificados/eliminados, `base_root_hash` y `new_root_hash`.
- `PullState(PullRequest) → stream NodeMetadata`: manifiesto de la instantánea vigente, con filtro opcional por
  `node_type` (clientes ligeros).
- `UploadArtifact(stream ArtifactChunk) → UploadAck` y `DownloadArtifact(ArtifactRequest) → stream ArtifactChunk`: el
  primer mensaje del stream lleva el `content_hash` y el tamaño; los siguientes, los bytes.
- `FindMissing(HashList) → HashList`: añadido a la propuesta original para no subir contenido que el servidor ya tiene.
- `DeclareManifest(ChunkManifest) → ManifestAck`: declara un archivo grande como lista ordenada de bloques ya subidos
  (ver «Chunking»).
- Versiones: `ListVersions(VersionsRequest) → VersionList` y `GetVersion(VersionRequest) → VersionMetadata`, ver
  `sync-versioning`. El contenido de una versión se descarga con `DownloadArtifact` usando su `content_hash`.

Todas las operaciones del servicio `Sync` exigen autenticación. `grpc.health.v1` (`tonic-health`) y la reflexión
(`tonic-reflection`) quedan fuera del interceptor. gRPC y `axum` escuchan en direcciones separadas (`grpc_addr`,
`http_addr`). TLS lo termina el proxy inverso del despliegue; el binario habla h2c.

**Errores con detalle.** Los códigos de estado se acompañan de detalles estándar de `google.rpc` (crate `tonic-types`)
para que el cliente sepa qué hacer sin parsear texto:

| Situación                              | Código                | Detalle                                                          |
|----------------------------------------|-----------------------|------------------------------------------------------------------|
| Sin token, token inválido o caducado   | `UNAUTHENTICATED`     | —                                                                |
| Cuenta fuera de la lista permitida     | `PERMISSION_DENIED`   | —                                                                |
| JWKS caducado y Google inalcanzable    | `UNAVAILABLE`         | —                                                                |
| Bóveda inexistente o de otro tenant    | `NOT_FOUND`           | — (las dos respuestas son idénticas: no se filtra la existencia) |
| Ruta, hash o mensaje mal formado       | `INVALID_ARGUMENT`    | `BadRequest` con el campo                                        |
| Delta incoherente (ver «Árbol Merkle») | `INVALID_ARGUMENT`    | `BadRequest` con la ruta                                         |
| Hash del contenido subido no coincide  | `INVALID_ARGUMENT`    | `BadRequest` (`content_hash`)                                    |
| Delta o manifiesto con contenido falta | `FAILED_PRECONDITION` | `PreconditionFailure`, tipo `MISSING_CONTENT`, un hash por fila  |
| Raíz declarada no coincide             | `FAILED_PRECONDITION` | `ErrorInfo` razón `ROOT_MISMATCH`                                |
| `base_root_hash` no es el vigente      | `ABORTED`             | `ErrorInfo` razón `STALE_BASE`, metadato `current_root_hash`     |
| Límite superado (tamaño, nodos)        | `RESOURCE_EXHAUSTED`  | —                                                                |

Orden de comprobación en `PushDelta`: autenticación → propiedad de la bóveda → `base_root_hash` → forma de rutas y
hashes → existencia de contenidos → recálculo de la raíz. La primera que falla decide el código.

**Límites configurables.** Tamaño máximo de mensaje gRPC (por defecto 4 MiB), tamaño recomendado de trozo de streaming
256 KiB, tamaño máximo de artefacto (por defecto 2 GiB), nodos máximos por delta (por defecto 10 000).

`axum` solo para `/healthz` y gestión ligera. Alternativa descartada: REST con JSON; gRPC da contratos estrictos y
streaming de binarios grandes sin trabajo extra.

**Autenticación: Google.** El cliente obtiene un ID token de Google (OAuth/OIDC) y lo envía como
`authorization: Bearer <id_token>`. Una capa `tower` asíncrona delante del servicio `Sync` lo valida (el `Interceptor`
de `tonic` es síncrono y no puede descargar el JWKS ni consultar la base): firma RS256 contra el JWKS de Google (URL
configurable, cacheado y renovado según `Cache-Control`), `iss` igual a `accounts.google.com` o
`https://accounts.google.com`, `aud` igual al `client_id` configurado y `exp` vigente (tolerancia de reloj de 60 s). El
`tenant_id` se deriva del claim `sub` (nunca del email, que puede cambiar) y se inyecta en las extensiones del
`Request`.

Capas: la verificación del token es un servicio externo (contrato `IdTokenVerifier` en
`application/integration/service/`, implementación con `jsonwebtoken` en `infrastructure`); resolver o dar de alta el
tenant es un caso de uso. El tipo `TenantId` vive en `domain`; los controllers no lo construyen a partir de campos del
mensaje, solo lo leen de las extensiones mediante un tipo `AuthenticatedTenant` de `infrastructure` cuyo constructor es
privado del módulo de autenticación. Los repositorios solo aceptan `TenantId`, así ninguna consulta SQL usa un tenant
sin validar. Un test de arquitectura comprueba que `TenantId` solo se construye en el módulo de autenticación, en los
repositorios (al leer filas) y en pruebas.

- Alta automática: el primer token válido de un `sub` nuevo crea su tenant con la política de retención por defecto.
- Lista de cuentas permitidas opcional en la configuración (por `sub` o por email verificado) para despliegues que no
  deben aceptar a cualquier cuenta de Google.
- El token se valida al iniciar cada llamada; una subida por streaming ya iniciada no se corta si el token caduca a
  mitad.
- El flujo OAuth de escritorio (loopback + PKCE, refresco del token) es trabajo del cliente en `synapsers` y queda fuera
  de este cambio. El servidor solo valida.
- Alternativa descartada: tokens propios con CLI de administración; Google evita gestionar credenciales y emisión de
  tokens.

**Tenant.** Es un concepto exclusivo del servidor. En el cliente no existe como entidad: es solo la identidad con la que
se autentica y no aparece en el modelo local.

**Bóvedas.** La propuesta original guarda una sola instantánea por tenant. Synapse maneja varias bóvedas, así que la
clave de la instantánea es `(tenant_id, vault_id)`; un tenant con una bóveda se comporta igual que el diseño original.
El `vault_id` es un UUID v7 que genera el cliente al crear la bóveda; el servidor lo trata como un identificador opaco y
lo acepta en el primer `PushDelta` de esa bóveda. Dónde lo guarda el cliente dentro de la bóveda es decisión del
cliente.

- El `vault_id` es **único en todo el servidor** (índice único en `snapshots.vault_id`), no solo por tenant. Así un
  tenant que usa el `vault_id` de otro recibe `NOT_FOUND` en lugar de crear en silencio una bóveda paralela.
- Una bóveda nueva tiene la raíz vacía `EMPTY_ROOT` (ver «Árbol Merkle»). Su primer `PushDelta` lleva
  `base_root_hash = EMPTY_ROOT` y crea la fila de `snapshots`.
- `CheckState` y `PullState` sobre una bóveda que no existe o que es de otro tenant responden `NOT_FOUND` (la misma
  respuesta en ambos casos). El cliente que recibe `NOT_FOUND` de su propia bóveda nueva la sube con base `EMPTY_ROOT`.

**Hash: BLAKE3 en hex (64 caracteres).** Rápido y con buen soporte en Rust. El `content_hash` siempre es del contenido
sin comprimir, así la compresión no altera la identidad.

**Chunking: FastCDC** para archivos > 4 MiB (configurable), bloques de ~1 MiB promedio (mínimo 256 KiB, máximo
4 MiB). **Corta el cliente, no el servidor.** El flujo es:

1. El cliente corta el archivo, llama a `FindMissing` con los hashes de bloque y sube los que faltan con
   `UploadArtifact` (cada bloque es un artefacto normal del CAS).
2. Llama a `DeclareManifest` con el `content_hash` del archivo completo, su tamaño y la lista ordenada de hashes de
   bloque. El servidor comprueba que tiene todos los bloques (si no, `FAILED_PRECONDITION` con los faltantes),
   reensambla en streaming para verificar que BLAKE3 del total y el tamaño coinciden (si no, `INVALID_ARGUMENT`) y
   guarda una fila de `blobs` con `chunked = true` (sin objeto físico propio) y las filas de `file_chunks`.
3. El archivo se referencia en `PushDelta` por su `content_hash` como cualquier otro.

`DownloadArtifact` de un hash con `chunked = true` reensambla los bloques en orden. El servidor solo usa `fastcdc` en
las pruebas (dependencia de desarrollo, para hacer de cliente); el umbral del servidor solo decide si acepta subir un
archivo enorme entero (por encima de `max_artifact_bytes` lo rechaza con `RESOURCE_EXHAUSTED`).

**Árbol Merkle (calculado en el cliente, verificado en el servidor).** Regla normativa, documentada también en
`proto/MERKLE.md` junto a `sync.proto` y con vectores de prueba fijos que el futuro cliente reutilizará:

- **Rutas**: relativas a la raíz de la bóveda, separador `/`, UTF-8 en forma NFC, sin `/` inicial ni final, sin
  segmentos vacíos, `.` ni `..`, sin `\0` ni `\`, como máximo 1024 bytes. Distinguen mayúsculas. Una ruta inválida da
  `INVALID_ARGUMENT`.
- **Tipos**: `FILE`, `DIR`, `INDEX` (texto ASCII en mayúsculas dentro del cálculo).
- **Hash de hoja** (`FILE`, `INDEX`): el `content_hash`, BLAKE3 del contenido sin comprimir, en hex minúscula de 64
  caracteres.
- **Hash de directorio**: BLAKE3 de la concatenación, para cada hijo directo ordenado por los bytes UTF-8 de su nombre
  (no la ruta completa), de `nombre 0x00 tipo 0x00 hash_hex 0x0A`. Un directorio vacío vale BLAKE3 de la entrada vacía.
- **Raíz**: el hash del directorio raíz. `EMPTY_ROOT` = BLAKE3("") =
  `af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262`.
- **Directorios**: el delta solo trae hojas (`FILE`, `INDEX`) y, opcionalmente, nodos `DIR` sin hash para crear
  directorios vacíos. Los directorios intermedios se crean implícitamente. El servidor calcula y guarda el hash de cada
  `DIR` en `tree_nodes`; el cliente no los envía. Borrar una ruta `DIR` borra todo su subárbol. Un directorio que queda
  vacío tras un borrado se conserva hasta que se borre explícitamente.
- **Mover**: no hay operación propia en el árbol; es borrar la ruta vieja y crear la nueva. El nodo nuevo puede llevar
  `moved_from` (que debe figurar en las rutas borradas del mismo delta) para que el historial registre `MOVE` en lugar
  de `DELETE` + `CREATE`.
- **Delta incoherente** (`INVALID_ARGUMENT`): borrar una ruta que no existe, una hoja donde hay un `DIR` o al revés, la
  misma ruta creada y borrada en el mismo delta, `moved_from` que no existe o no se borra.
- **Versiones**: cada cambio de hoja genera una fila de `file_versions`; un upsert con el mismo hash y tamaño que ya
  tiene la ruta no es un cambio. Los `DIR` no generan versiones.
- **Concurrencia**: la transacción de `PushDelta` se abre con `BEGIN IMMEDIATE`, así dos deltas sobre la misma base se
  serializan y el segundo recibe `ABORTED`.

El servidor recalcula solo los ancestros de las rutas tocadas (no todo el árbol) y rechaza el delta si la raíz no
coincide: es el notario, no el autor.

**Persistencia: `sqlx` con SQLite en WAL.** En este cambio solo se implementa SQLite; PostgreSQL (con
`PARTITION BY HASH (tenant_id)` en las tablas grandes) queda para un cambio posterior. Las consultas se escriben en SQL
portable donde sea posible y se aíslan en `infrastructure/repository/sqlite/` para que añadir PostgreSQL sea otra
implementación de los mismos contratos. Al abrir SQLite se ejecuta siempre
`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;` y `busy_timeout`, con un pool de
conexiones acotado. Las consultas usan las macros `query!`/`query_as!` con la caché `.sqlx/` versionada; CI compila con
`SQLX_OFFLINE=true`. Todas las tablas llevan `tenant_id`:

- `tenants(tenant_id, google_sub, retention_policy, created_at)` — `google_sub` único; `tenant_id` UUID v7 propio del
  servidor (no el `sub`)
- `snapshots(tenant_id, vault_id, root_hash, updated_at)` — puntero a la instantánea vigente; clave
  `(tenant_id, vault_id)` y `vault_id` único
- `tree_nodes(tenant_id, vault_id, path, node_type FILE|DIR|INDEX, content_hash, size_bytes)` — estado plano vivo,
  clave `(tenant_id, vault_id, path)`. Sin `storage_uri`: la ubicación física se deriva del hash
- `file_versions(tenant_id, vault_id, path, version_id, node_type, content_hash, size_bytes, op, moved_from,
  created_at)` — historial, solo punteros. `version_id` UUID v7; `op` ∈ `CREATE|MODIFY|DELETE|MOVE`; en `DELETE`,
  `content_hash` es el último que tuvo la ruta
- `blobs(tenant_id, content_hash, size_bytes, stored_size, chunked, created_at, last_seen_at)` — `last_seen_at` se
  actualiza al subir o al aparecer como existente en `FindMissing`, y es lo que mira el periodo de gracia del GC
- `file_chunks(tenant_id, content_hash, idx, chunk_hash)`

Cada `PushDelta` se aplica en una sola transacción (`BEGIN … COMMIT`): comprueba `base_root_hash`, inserta
`file_versions`, actualiza `tree_nodes`, verifica la raíz y sobrescribe `snapshots.root_hash`.

**Contenidos: repository `ContentRepository`.** Los contenidos y su formato en el almacén son nuestros, así que por la
regla de propiedad es un repository (contrato en `domain/content/`), no un servicio externo. Una sola implementación,
`ContentRepositoryImpl` en `infrastructure/repository/object_store/`, que usa `object_store` con disco local
(`<data>/blobs/<tenant_id>/ab/cd/<hash>.zst`, donde `ab` y `cd` son los cuatro primeros caracteres del hash) o S3/R2
(sin coste de egress, misma clave) según la configuración. Siempre Zstd (nivel 3). El tenant va en la ruta física: la
deduplicación es por tenant, como dice la propuesta, y así el GC de un tenant nunca borra un objeto de otro ni
`FindMissing` revela qué contenido tienen otros tenants.

La subida escribe a un objeto temporal mientras calcula BLAKE3 y el tamaño, y solo al verificar lo mueve a su clave
definitiva e inserta la fila de `blobs` (`ON CONFLICT DO NOTHING`). Una subida cortada o con hash erróneo no deja ni
fila ni objeto definitivo.

**Índices.** Los segmentos de Tantivy (u otras estructuras serializadas) son nodos `INDEX` del árbol y artefactos opacos
en el CAS; el servidor no conoce su formato. Un cliente ligero hace `PullState` filtrado por `INDEX` y luego
`DownloadArtifact`.

**Historial.** La propuesta original declara el historial efímero (solo la última instantánea). Se adopta la evolución
posterior: el estado vivo sigue en `tree_nodes`/`snapshots` y se agrega `file_versions` con punteros, cuyo tamaño es
despreciable porque el contenido ya está deduplicado. Un plan sin historial equivale a retención «0 versiones extra».

**Retención y GC.** Tarea `tokio` periódica (intervalo configurable, 1 h por defecto): primero aplica la retención por
tenant sobre `file_versions` (respetando la versión vigente), luego marca y barre: contenidos referenciados por
`tree_nodes`, `file_versions` y `file_chunks` (un bloque está referenciado si lo está su archivo); se borran los no
referenciados con `last_seen_at` anterior al periodo de gracia (24 h por defecto). Primero se borra la fila de `blobs`
dentro de una transacción y después el objeto; como `PushDelta` comprueba la existencia de contenidos dentro de su
propia transacción, un delta nunca se confirma apuntando a un blob ya recolectado.

- La política se guarda como texto en `tenants.retention_policy`: `unlimited`, `versions:N` o `days:N`. Los tenants
  nuevos reciben `default_retention` de la configuración (por defecto `unlimited`, hasta fijar los planes). No hay API
  para cambiarla en este cambio.
- `versions:N` conserva por ruta las N versiones más recientes, **contando la vigente**. `days:N` conserva las
  versiones de los últimos N días. En ambas, la versión vigente de cada ruta viva se conserva siempre. Una ruta borrada
  no tiene versión vigente: todas sus versiones quedan sujetas a la política.

## Risks / Trade-offs

- [Raíz Merkle vieja en clientes con muchas ediciones] → el cliente reconcilia con `PullState` y reintenta; los deltas
  son pequeños.
- [GC borra un blob justo antes de que se confirme] → periodo de gracia y verificación de existencia dentro de la
  transacción del delta.
- [SQLite con muchos tenants] → misma capa `sqlx`; se cambia a PostgreSQL por configuración.
- [El formato de los segmentos cambia con la versión de Tantivy] → el servidor los trata como opacos; el cliente decide
  si puede usarlos y vuelve a subir si no.
- [SQLite con escrituras concurrentes] → WAL y pool acotado; PostgreSQL particionado cuando haga falta.
- [Contenido de usuarios en claro en el servidor] → TLS obligatorio en despliegue; cifrado de extremo a extremo queda
  como trabajo futuro.
- [Dependencia de Google para autenticar] → el JWKS se cachea para tolerar cortes breves; sin conexión a Google y con la
  caché vencida el servidor responde `UNAVAILABLE` en lugar de aceptar tokens sin verificar.
- [Cualquier cuenta de Google podría crear un tenant] → lista de cuentas permitidas configurable.

## Migration Plan

Servicio nuevo, sin datos previos. Migraciones `sqlx` versionadas dentro de `synapse-server/migrations/`. Despliegue
como binario único o imagen Docker; revertir es apagar el servicio.

**Estructura del crate.** `src/lib.rs` expone las capas y una función de arranque (`run`/`build_server`) que recibe la
configuración y los `TcpListener`; `src/main.rs` solo lee la configuración, inicia `tracing` y la llama. Así las
pruebas de integración levantan el servidor en el mismo proceso, en puertos efímeros, con SQLite en memoria y un
directorio temporal de blobs. El módulo `architecture` del binario de integración (el
equivalente al `tests/architecture.rs` de la arquitectura cebolla) vigila las dependencias entre capas; `blake3` figura en su lista de librerías permitidas del dominio
(estándar puro, usado en el cálculo Merkle).

**Configuración** (valores por defecto → TOML → `SYNAPSE_*`; se valida al arrancar): `grpc_addr`, `http_addr`,
`log_format`, `database_url`, `blob_backend` (`local` con `data_dir`, o `s3` con bucket, región, endpoint; las
credenciales solo por variables de entorno), `google_client_id`, `google_jwks_url`, `allowed_accounts` (lista de `sub`
o de emails verificados; vacía = cualquiera), `default_retention`, `chunk_threshold_bytes`, `max_artifact_bytes`,
`max_message_bytes`, `max_delta_nodes`, `gc_interval`, `gc_grace_period`.

## Open Questions

- Valores concretos de retención por plan comercial (el código acepta cualquiera; el valor por defecto es `unlimited`).
- PostgreSQL: se implementa en un cambio posterior cuando un despliegue lo necesite.
