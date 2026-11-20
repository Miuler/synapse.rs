# P01 — Base del crate: configuración completa, lib + bin y guardián de capas

**Tareas OpenSpec:** 1.1 (comprobar y marcar), 1.2, 1.3. **Depende de:** nada.

## Estado de partida (ya hecho, no lo rehagas)

`synapse-server/` es un crate independiente con `Cargo.toml`/`Cargo.lock` propios, edición 2024,
`rust-toolchain.toml`, `.cargo/config.toml` con `mold`, `rustfmt.toml`, lints (`unsafe_code = forbid`,
`unwrap_used`/`expect_used` en `warn`), perfiles. Código: `src/main.rs` con `mod` de las tres capas,
`infrastructure/config.rs` (defaults → TOML → `SYNAPSE_*`, solo `http_addr` y `log_format`),
`infrastructure/entrypoint/http/` con `GET /healthz`, una prueba unitaria y una de integración que lanza el binario
como subproceso. Compruébalo con la verificación obligatoria antes de cambiar nada.

## Objetivo

1. **Configuración completa (1.2).** `Config` con todas las claves de la sección «Configuración» del `design.md`:
   `grpc_addr` (defecto `127.0.0.1:50051`), `http_addr`, `log_format`, `database_url` (defecto
   `sqlite://data/synapse.db?mode=rwc`), `blob_backend` (enum etiquetado: `local { data_dir }` por defecto `data/`,
   `s3 { bucket, region, endpoint }` — las credenciales S3 **no** van en `Config`, las lee `object_store` del entorno),
   `google_client_id` (obligatorio salvo en pruebas), `google_jwks_url` (defecto
   `https://www.googleapis.com/oauth2/v3/certs`), `allowed_accounts: Vec<String>` (vacía = cualquiera),
   `default_retention` (defecto `unlimited`), `chunk_threshold_bytes` (4 MiB), `max_artifact_bytes` (2 GiB),
   `max_message_bytes` (4 MiB), `max_delta_nodes` (10 000), `gc_interval` (1 h), `gc_grace_period` (24 h).
   - Duraciones en TOML como texto legible (`"1h"`, `"24h"`); elige `humantime-serde` o equivalente y dilo en el
     cierre (no está en la tabla del stack: es la dependencia permitida de esta petición).
   - Variables anidadas: `SYNAPSE_BLOB_BACKEND__…` con el separador de `figment` (`Env::prefixed("SYNAPSE_").split("__")`).
   - `Config::validate()` falla rápido y con mensaje claro: direcciones que no parsean, `google_client_id` vacío,
     política de retención con formato inválido (`unlimited` | `versions:N` con N ≥ 1 | `days:N` con N ≥ 1), límites a
     cero, `chunk_threshold_bytes > max_artifact_bytes`.
   - La política de retención se parsea a un tipo de `domain` (`RetentionPolicy`), no se queda como `String`.
   - Mueve `config.rs` a `infrastructure/config/` (módulo con `settings.rs` y, desde P03, el ensamblado).
   - Incluye un `synapse-server/config.example.toml` comentado con todas las claves.
2. **lib + bin (1.3).** `src/lib.rs` declara las capas y expone una función de arranque, por ejemplo
   `pub async fn run(config: Config, listeners: Listeners, shutdown: impl Future<Output = ()>) -> anyhow::Result<()>`,
   donde `Listeners` lleva el `TcpListener` HTTP (y desde P03 el gRPC). `main.rs` queda en: parsear CLI, cargar y
   validar configuración, iniciar `tracing`, abrir listeners, llamar a `run`. Apagado ordenado con Ctrl+C y SIGTERM.
3. **Arnés de pruebas en proceso.** `tests/integration/support/mod.rs` con un `TestServer` que arranca `run` en puertos
   efímeros con configuración de prueba (directorio temporal, `google_client_id = "test-client"`) y se apaga al
   soltarse. Reescribe `health.rs` para usarlo. Deja **una** prueba que sigue lanzando el binario real como subproceso
   (`arranca_el_binario_y_responde_salud`) para cubrir el escenario «Arranque sin la app de escritorio».
4. **Guardián de capas.** Módulo `tests/integration/architecture.rs` que recorre `src/domain` y `src/application`
   (con `std::fs` y búsqueda de texto de `use`/rutas `crate::`/nombres de crate) y falla si aparecen importaciones
   prohibidas (lista del `00_contexto.md`), con lista explícita de permitidas y un comentario por excepción.

## Pruebas exigidas

- Unitarias de `Config`: defaults; TOML sobrescribe defaults; `SYNAPSE_*` sobrescribe TOML (usa `figment::Jail`);
  cada caso inválido de `validate()` da error con el nombre de la clave; parseo de `RetentionPolicy`.
- Integración: `healthz_responde_200` (en proceso) y `arranca_el_binario_y_responde_salud` (subproceso).
- `architecture::dominio_no_conoce_el_exterior` y `architecture::aplicacion_solo_conoce_el_dominio` (pasan hoy, con
  capas vacías; lo importante es que fallen si alguien añade `use sqlx` en `domain`: compruébalo a mano una vez y
  cuéntalo en el cierre).

## Fuera de alcance

gRPC, base de datos, almacenamiento (solo sus claves de configuración). CI (es P02).

## Hecho cuando

Verificación obligatoria en verde, `cargo run` arranca con la configuración por defecto (más
`SYNAPSE_GOOGLE_CLIENT_ID=x`) y responde `/healthz`, 1.1–1.3 marcadas en `tasks.md`, CHANGELOG actualizado.
