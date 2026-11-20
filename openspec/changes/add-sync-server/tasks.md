## 1. Estructura del proyecto

- [x] 1.1 Crear `synapse-server/` (crate binario independiente, capas `domain`/`application`/`infrastructure`
  según la arquitectura cebolla, sin dependencia de Tauri); verificar con `cargo build` dentro de `synapse-server/`. La
  reorganización de carpetas del repo es una tarea aparte
- [x] 1.2 Configuración por archivo y variables de entorno con todas las claves de «Configuración» del `design.md`
  (direcciones gRPC y HTTP, base de datos, backend de blobs, Google, límites, chunking, GC), validada al arrancar, y
  `GET /healthz` con `axum`; prueba de integración que arranca el servidor y recibe 200. Hoy solo existen `http_addr` y
  `log_format`
- [x] 1.3 Separar `src/lib.rs` (capas y función de arranque que recibe configuración y listeners) de `src/main.rs`
  (solo lectura de configuración, `tracing` y llamada); prueba de integración en el mismo proceso con puertos efímeros;
  módulo `architecture` en las pruebas de integración que vigile las dependencias entre capas
- [ ] 1.4 Comprobar que `ci/release.py` y el workflow de release de escritorio no empaquetan `synapse-server`; agregar
  un job de CI que compile y pruebe el servidor (`protoc` instalado, `SQLX_OFFLINE=true`) y cubrirlo en Dependabot,
  `cargo audit` y `cargo deny`

## 2. Contrato, persistencia y almacenamiento

- [x] 2.1 Escribir `proto/sync.proto` (paquete `synapse.sync.v1`: `CheckState`, `PushDelta`, `PullState`,
  `UploadArtifact`, `DownloadArtifact`, `FindMissing`, `DeclareManifest`, `ListVersions`, `GetVersion`) y generarlo en
  `build.rs`; servidor gRPC en `grpc_addr` con `tonic-health` y `tonic-reflection`, métodos aún `UNIMPLEMENTED`; prueba
  que llama a la salud gRPC
- [ ] 2.2 Regla del Merkle Tree en `domain` (validación de rutas, hash de directorio, raíz, `EMPTY_ROOT`) y
  `proto/MERKLE.md` con la regla y vectores de prueba; pruebas unitarias con los vectores y `proptest` (el orden de
  inserción no cambia la raíz; cambiar un byte cambia la raíz)
- [ ] 2.3 Migraciones `sqlx` para `tenants`, `snapshots`, `tree_nodes`, `file_versions`, `blobs`, `file_chunks` según el
  `design.md`; apertura de SQLite con `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout` y pool
  acotado; caché `.sqlx/` versionada; prueba que migra una base vacía y lee los PRAGMA aplicados
- [ ] 2.4 Repository `ContentRepository` (contrato en `domain/content/`) y `ContentRepositoryImpl` con `object_store`
  sobre disco local (Zstd, `<tenant>/ab/cd/<hash>.zst`, escritura temporal y movimiento al verificar); pruebas de
  escritura idempotente, lectura, aislamiento entre tenants y de que una escritura con hash erróneo no deja nada

## 3. Sincronización (`sync-ingest`)

- [ ] 3.1 Capa `tower` de autenticación con el contrato `IdTokenVerifier` (implementación con `jsonwebtoken`, JWKS
  cacheado según `Cache-Control`, `iss`, `aud`, `exp`), caso de uso que resuelve o da de alta el tenant (lista de
  cuentas permitidas opcional) e inyección de `AuthenticatedTenant` en las extensiones; pruebas con un JWKS de prueba
  servido en local: sin token o caducado → `UNAUTHENTICATED`, audiencia distinta → `UNAUTHENTICATED`, cuenta no
  permitida → `PERMISSION_DENIED` sin crear tenant, primer acceso crea el tenant, JWKS inaccesible y caducado →
  `UNAVAILABLE`, la salud gRPC no exige token
- [ ] 3.2 `CheckState`; pruebas de cliente al día, desactualizado, bóveda desconocida y bóveda de otro tenant (ambas
  `NOT_FOUND`)
- [ ] 3.3 `FindMissing`, `UploadArtifact` y `DownloadArtifact` por streaming con verificación BLAKE3 y actualización de
  `last_seen_at`; pruebas: hash incorrecto → `INVALID_ARGUMENT`, subida repetida no duplica, contenido igual en dos rutas
  se guarda una vez, descarga devuelve los bytes originales, un tenant no descarga ni ve en `FindMissing` el contenido de
  otro, artefacto por encima del límite → `RESOURCE_EXHAUSTED`
- [ ] 3.4 `DeclareManifest` y descarga reensamblada de archivos en bloques; pruebas (con `fastcdc` como dependencia de
  desarrollo haciendo de cliente): editar unos bytes de un archivo de 50 MB solo sube los bloques cambiados, un
  manifiesto con bloques faltantes se rechaza indicándolos, un manifiesto cuyo total no da el hash declarado →
  `INVALID_ARGUMENT`
- [ ] 3.5 `PushDelta` transaccional: propiedad de la bóveda, `base_root_hash`, rutas, existencia de contenidos,
  directorios implícitos, borrado de subárboles, recálculo de la raíz y actualización de `snapshots`, con errores y
  detalles de la tabla del `design.md`; pruebas: primera subida sobre `EMPTY_ROOT`, delta válido, ruta inválida,
  artefacto faltante y raíz inconsistente → `FAILED_PRECONDITION`, base vieja → `ABORTED` con la raíz vigente y sin
  cambios, bóveda de otro tenant → `NOT_FOUND`, borrado de un directorio
- [ ] 3.6 `PullState` por streaming con filtro por `node_type`; prueba de reconciliación tras `ABORTED`
- [ ] 3.7 Documentar el uso del servicio (configuración, autenticación, flujo de sincronización, chunking, códigos de
  error) en `synapse-server/README.md`

## 4. Versiones, retención y GC (`sync-versioning`)

- [ ] 4.1 Registrar `file_versions` (`CREATE`, `MODIFY`, `DELETE`, `MOVE`) en la misma transacción de cada `PushDelta`;
  pruebas de historial tras dos ediciones, tras borrar y tras mover
- [ ] 4.2 `ListVersions` y `GetVersion`; prueba de bytes idénticos al descargar una versión antigua (también en bloques)
- [ ] 4.3 Políticas de retención por tenant (`versions:N` contando la vigente, `days:N`, `unlimited`) que nunca borran
  la versión vigente; pruebas de los escenarios de la spec
- [ ] 4.4 Recolector de basura con periodo de gracia sobre `last_seen_at` como tarea periódica; pruebas: borra lo no
  referenciado, conserva artefactos compartidos, bloques de archivos vivos y subidas sin confirmar

## 5. Índices (`sync-index-ingest`)

- [ ] 5.1 Nodos `INDEX` en `PushDelta` y en `PullState` filtrado; prueba: un cliente que solo pide `INDEX` recibe los
  segmentos de la última instantánea y no los archivos
- [ ] 5.2 Prueba de deduplicación de un segmento sin cambios entre dos instantáneas y de su recolección cuando se
  reemplaza y sale de la retención

## 6. Integración

- [ ] 6.1 Prueba de extremo a extremo con un cliente gRPC de prueba en Rust: subir una bóveda con un segmento de
  Tantivy, editar, mover y borrar archivos, aplicar retención y GC, y verificar instantánea, historial y almacenamiento
  final
- [ ] 6.2 Backend S3/R2 de `ContentRepositoryImpl` por configuración (feature `aws` de `object_store`); prueba contra
  MinIO con `testcontainers`, marcada `#[ignore]` si no hay Docker
- [ ] 6.3 Entrada en `CHANGELOG.md` («[Sin publicar]») y sección en `ARCHITECTURE.md` describiendo el servidor
