# Stack y reglas para proyectos Rust

Este documento complementa [Arquitectura Cebolla](arquitectura_cebolla.md). Allí está **cómo
se organiza** el código; aquí, **con qué se construye** un proyecto Rust y las reglas de uso.
Igual que aquel, es **normativo para agentes de IA**: no son sugerencias. Si un caso no encaja,
se discute y se cambia este documento antes de desviarse en el código.

---

## 1. Base del lenguaje

- **Edición 2024** y la última versión estable de Rust (`rust-toolchain.toml` fijado por proyecto).
- **Asíncrono siempre con `tokio`** (runtime multihilo). No se mezclan runtimes (`async-std`,
  `smol`). El código bloqueante (CPU intensiva, I/O síncrona de librerías) va en
  `tokio::task::spawn_blocking` o en `rayon`, nunca directo dentro de una tarea async.
- `unsafe` prohibido salvo justificación escrita en un comentario `// SAFETY:` y revisión.
- Nada de `unwrap()`/`expect()` fuera de pruebas, `main` de arranque o invariantes
  imposibles documentadas con un `expect("motivo")`.

## 2. Comunicación entre servicios

| Uso                                                   | Librería                           | Regla                                                                                                                |
|-------------------------------------------------------|------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| API entre servicios y clientes propios                | **gRPC con `tonic` + `prost`**     | **Siempre gRPC por encima de REST.** Contrato en `.proto` (proto3), generado con `tonic-build` en `build.rs`         |
| Salud, métricas, webhooks de terceros, gestión ligera | **`axum`**                         | Solo cuando el que llama no puede hablar gRPC. No se exponen casos de uso de negocio por REST si ya existen por gRPC |
| Salud y descubrimiento gRPC                           | `tonic-health`, `tonic-reflection` | Activos en todo servidor gRPC                                                                                        |
| Cliente HTTP saliente                                 | **`reqwest`** con `rustls`         | Sin OpenSSL. Timeouts explícitos siempre                                                                             |

Reglas gRPC:

- Los `.proto` viven en `proto/` del crate (o en un crate `*-proto` compartido cuando haya
  más de un consumidor) y se versionan por paquete (`synapse.sync.v1`).
- Los tipos generados por `prost` son de **infraestructura**: se traducen a tipos de dominio en
  el adaptador de entrada; nunca llegan a `domain` ni a `application`.
- Errores: se mapean a `tonic::Status` con el código correcto (`INVALID_ARGUMENT`,
  `NOT_FOUND`, `ABORTED`, `UNAUTHENTICATED`…), en un único lugar del adaptador.
- Autenticación y contexto (tenant, usuario) en **interceptores/capas `tower`**, inyectados en
  las extensiones del `Request`.
- Streaming para binarios grandes; nunca un `bytes` gigante en un mensaje unario.

## 3. Persistencia

- **SQL: `sqlx`** (async, consultas verificadas en compilación con `query!`/`query_as!` y
  `cargo sqlx prepare` en CI). Sin ORM (Diesel, SeaORM).
- **SQLite** para proyectos ligeros o locales, siempre con
  `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON` y pool acotado. **PostgreSQL** para servidores con
  concurrencia de escritura.
- Migraciones con `sqlx migrate`, versionadas en `migrations/` del crate. Nunca se edita una
  migración publicada: se agrega otra.
- Toda escritura que toque más de una tabla va en una transacción explícita.
- Los repositorios son puertos (traits) en `domain`/`application` e implementaciones en
  `infrastructure`.
- Almacenamiento de objetos: crate **`object_store`** (S3, R2, disco local) detrás de un puerto propio.

## 4. Serialización

- **`serde`** con `derive` para todo. JSON con `serde_json`; JSONC/JSON con comentarios con
  `serde_json_lenient`; YAML con `serde_yaml` solo para archivos que el usuario edita.
- Formatos binarios internos (cachés, índices en disco): **`postcard`**.
- Los DTO serializables viven en la capa que los necesita (infraestructura o entrada); el
  dominio no depende de formatos de transporte.
- Campos con `#[serde(default)]` cuando se agregan a un formato ya publicado, para no romper
  datos existentes.

## 5. Errores

- **`thiserror`** para errores tipados en `domain`, `application` e `infrastructure`. Cada capa
  define los suyos y convierte los de la capa interior con `#[from]`.
- **`anyhow`** solo en binarios (`main`, CLI, pruebas), nunca en librerías ni en la API de una capa.
- Los mensajes de error para el usuario se escriben en español; los nombres de tipos y
  variantes, en inglés.

## 6. Logs y telemetría

- **Logs: `tracing`** + **`tracing-subscriber`** (`EnvFilter` vía `RUST_LOG`, formato `fmt`
  en desarrollo y JSON en producción). No se usa `println!` ni el crate `log` en código nuevo;
  las dependencias que emiten `log` se puentean con `tracing-log`.
- Instrumentar con `#[tracing::instrument(skip(...))]` en casos de uso y adaptadores; nunca se
  registran secretos, tokens ni contenido de archivos del usuario (usar `skip` y campos de tamaño o hash).
- **Telemetría: OpenTelemetry** — `opentelemetry`, `opentelemetry_sdk`,
  `opentelemetry-otlp` (exportador OTLP por gRPC) y `tracing-opentelemetry` para que las spans
  de `tracing` sean trazas. La misma configuración exporta **métricas** (`opentelemetry` meters).
- Propagación de contexto W3C (`traceparent`) en gRPC y HTTP mediante capas `tower`
  (`tower-http` para `axum`).
- La telemetría se activa por configuración; sin colector configurado el servicio funciona igual.
- En la app de escritorio (Tauri) se mantiene `tauri-plugin-log` como salida, alimentado desde `tracing`.

## 7. Configuración y CLI

- **`figment`** para cargar configuración en capas: valores por defecto → archivo (TOML o
  JSONC) → variables de entorno con prefijo del proyecto. Se valida al arrancar y falla rápido.
- **`clap`** (derive) para argumentos de línea de comandos.
- Secretos solo por variables de entorno o gestor de secretos; nunca en archivos del repo.

## 8. Utilidades estándar

| Necesidad               | Librería                                         |
|-------------------------|--------------------------------------------------|
| Identificadores         | `uuid` v7 (ordenables por tiempo)                |
| Fechas                  | `chrono` (con `serde` y soporte `sqlx`)          |
| Hash de contenido       | `blake3`; `sha2` solo por compatibilidad externa |
| Compresión              | `zstd`                                           |
| Bytes y buffers         | `bytes`                                          |
| Concurrencia en memoria | `dashmap`, `tokio::sync`                         |
| Paralelismo de CPU      | `rayon`                                          |
| Strings compactos       | `compact_str`                                    |
| Middleware              | `tower`, `tower-http`                            |

Agregar una dependencia nueva que duplique una de esta lista requiere justificarlo en el PR.

## 9. Pruebas

- Unitarias junto al código (`#[cfg(test)]`); de integración en `tests/`.
- Async con `#[tokio::test]`. Bases de datos reales en pruebas (SQLite en memoria o
  `testcontainers` para PostgreSQL/MinIO), nunca mocks de SQL.
- Snapshots con `insta` cuando la salida es grande; propiedades con `proptest` para
  algoritmos (hashing, chunking, parsers).
- Los servidores gRPC se prueban levantándolos en un puerto efímero y llamándolos con el
  cliente generado.
- Linker: si `ld.lld` se cuelga enlazando muchos binarios de prueba, limitar con `cargo test -j 2`.

## 10. Calidad y CI

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test` obligatorios en CI.
- `cargo deny` (licencias y avisos de seguridad) y `cargo machete` (dependencias sin usar).
- Workspace de Cargo cuando el repo tenga más de un crate, con versiones de dependencias
  centralizadas en `[workspace.dependencies]`.
- Documentación pública con `///` en español; nombres de código en inglés.
