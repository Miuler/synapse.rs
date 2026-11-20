# Registro de cambios (`synapse-server`)

Todos los cambios notables de `synapse-server` serán documentados en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Sin publicar] (Unreleased)

### Añadido

- Contrato gRPC `synapse.sync.v1` (`proto/synapse/sync/v1/sync.proto`), generación con `tonic-prost-build` en `build.rs`, servidor `tonic` en `grpc_addr` con `tonic-health` (`SERVING`), `tonic-reflection`, límites `max_message_bytes`, `SyncController` (`UNIMPLEMENTED`) y mapeo centralizado de errores `google.rpc` con `tonic-types`.
- Configuración completa en capas (defaults → TOML → `SYNAPSE_*` con soporte de claves anidadas `__`) validada al arrancar, `RetentionPolicy` en `domain`, `config.example.toml`, separación `lib.rs`/`main.rs` con apagado ordenado (Ctrl+C y `SIGTERM`), arnés de pruebas en proceso (`TestServer`) y guardián de capas de arquitectura cebolla (`tests/integration/architecture.rs`).
