# Tasks: Integración de Agentes ACP (`add-acp-agent`)

## 1. Backend Rust (Arquitectura Cebolla)

- [ ] 1.1 Definir entidades y puertos en `src-tauri/src/domain/acp/` (`AcpAgentProfile`, `AcpSession`, `AcpClientPort`,
  `AcpAgentRegistryPort`, `AcpInstallerPort`).
- [ ] 1.2 Implementar casos de uso en `src-tauri/src/application/use_cases/acp/` (gestión de perfiles,
  instalación/descarga, ciclo de vida de sesión, prompts y permisos).
- [ ] 1.3 Implementar adaptador `stdio` JSON-RPC en `src-tauri/src/infrastructure/service/acp/` con emisión de eventos
  Tauri.
- [ ] 1.4 Implementar descargador del catálogo verificado (**Antigravity** y **Claude**) y persistencia de perfiles
  personalizados.
- [ ] 1.5 Exponer comandos `#[tauri::command]` en `src-tauri/src/infrastructure/entrypoint/tauri/` y registrarlos en
  `lib.rs`.
- [ ] 1.6 Añadir pruebas unitarias y de integración (`cargo test --manifest-path src-tauri/Cargo.toml`).

## 2. Frontend Svelte 5 (FSD)

- [ ] 2.1 Crear slice `entities/acp-agent/` (`model/` con Svelte 5 Runes y `api/` para IPC Tauri).
- [ ] 2.2 Crear slice `features/acp-chat/` (`ui/` para historial, streaming, *tool calls*, permisos y estado vacío).
- [ ] 2.3 Crear slice `widgets/acp-panel/` (`ui/` con cabecera, selector rápido de agente, botón `⚙` a Ajustes y toggle
  fijo/flotante).
- [ ] 2.4 Integrar `widgets/acp-panel` en `pages/workspace/` acoplado por defecto a la derecha y soportando modo
  flotante.
- [ ] 2.5 Añadir sección de gestión de agentes ACP en `widgets/settings-panel/`.

## 3. Verificación

- [ ] 3.1 Ejecutar `bun run check` (0 errores, 0 warnings).
- [ ] 3.2 Ejecutar `bun run build`.
- [ ] 3.3 Ejecutar `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] 3.4 Actualizar `CHANGELOG.md` bajo `## [Sin publicar] (Unreleased)`.
