# Tasks: Integración de Agentes ACP (`add-acp-agent`)

> **Alcance:** **Claude** (`claude-acp`) desde el registro de ACP y **Antigravity** desde el CLI `agy` ya instalado
> (`agy acp`), sin descarga.

## 1. Backend Rust (Arquitectura Cebolla)

- [x] 1.1 Definir entidades (`AgentProfile`, `AgentSession`, `AgentMessage`, `PermissionRequest`) y el repository
  `AgentProfileRepository` en `src-tauri/src/domain/agent/`, y los contratos de servicios externos `AgentRunner`,
  `AgentCatalog` y `AgentInstaller` en `src-tauri/src/application/integration/service/`.
- [x] 1.2 Implementar los casos de uso (`…UseCase`, uno por intención, con su `Command`/`Query`) en
  `src-tauri/src/application/use_case/` (perfiles, instalación, ciclo de vida de sesión, prompts y permisos).
- [x] 1.3 Implementar `AgentRunnerImpl` (`stdio` JSON-RPC) en `src-tauri/src/infrastructure/integration/service/acp/`
  con emisión de eventos Tauri.
- [x] 1.4 Catálogo verificado desde el registro público de ACP (el mismo que usa Zed), instalador con verificación
  sha256 y persistencia de perfiles personalizados en `~/.config/synapsers/agents.json`; primer agente verificado:
  **Claude** (`claude-acp`).
- [x] 1.4b Agregar **Antigravity** como verificado lanzando `agy acp` del CLI instalado (PATH o `~/.local/bin`), con
  aviso de inicio de sesión cuando responda «Authentication required».
- [x] 1.5 Exponer comandos `#[tauri::command]` en un controller de
  `src-tauri/src/infrastructure/entrypoint/controller/tauri/` y registrarlos en
  `lib.rs`.
- [x] 1.6 Añadir pruebas unitarias y de integración (`cargo test --manifest-path src-tauri/Cargo.toml`).

## 2. Frontend Svelte 5 (FSD)

- [x] 2.1 Crear slice `entities/acp-agent/` (`model/` con Svelte 5 Runes y `api/` para IPC Tauri).
- [x] 2.2 Crear slice `features/acp-chat/` (`ui/` para historial, streaming, *tool calls*, permisos y estado vacío).
- [x] 2.3 Crear slice `widgets/acp-panel/` (`ui/` con cabecera, selector rápido de agente, botón `⚙` a Ajustes y toggle
  fijo/flotante).
- [x] 2.4 Integrar `widgets/acp-panel` en `pages/workspace/` acoplado por defecto a la derecha y soportando modo
  flotante.
- [x] 2.5 Añadir sección de gestión de agentes ACP en `widgets/settings-panel/`.

## 4. Contexto, visibilidad y particularidades de cada agente

- [x] 4.1 Inyectar el MCP de Synapse en cada sesión (`mcpServers` de `session/new`): HTTP en el puerto activo si el
  servidor MCP corre y el agente anuncia `mcpCapabilities.http`; si no, stdio con `synapsers --mcp --vault <bóveda>`.
  Prueba unitaria de la elección.
- [x] 4.2 Reconocer las capacidades de cada agente (`promptCapabilities`, `mcpCapabilities`, `authMethods`, modos y
  modelos de `session/new`) y exponer cambio de modelo (`session/set_model`) y de modo (`session/set_mode`) en la
  cabecera cuando el agente los ofrece.
- [x] 4.3 Mostrar lo que hace el agente: plan (`plan`), herramientas con archivos tocados y diff plegable, comandos
  disponibles (`available_commands_update`) con autocompletado `/` y uso de contexto (`usage_update`).
- [x] 4.4 Contexto en el prompt: `@` para mencionar notas (bloque `resource_link`), nota activa como etiqueta
  removible, enviar la selección del editor (bloque `resource` con rango de líneas, o texto citado si el agente no
  admite contexto incrustado) y arrastrar notas al chat.
- [x] 4.5 Probar con Claude (`claude-acp`, descargado del registro) y Antigravity (`agy acp`) reales: saludo, sesión
  con el MCP de Synapse y un prompt corto.
- [x] 4.6 e2e del agente falso cubriendo plan, diff, `@` mención y MCP inyectado.
- [x] 4.7 MCP obligatorio de la instancia abierta: al abrir una sesión se arranca el MCP de la app si estaba apagado;
  agentes con HTTP se conectan directo y los de solo stdio por `synapsers --mcp-proxy <url>` (puente a la misma
  instancia). Prueba unitaria del puente y e2e en la que el agente lista las herramientas por el puente.
- [x] 4.8 Identificar el MCP de la instancia como `synapse_instance` (distinto de un `synapse` global) e instruir al
  agente, con el primer mensaje, a usarlo y a crear/editar notas con sus herramientas en vez de escribir archivos.
  Verificado con Claude real (`write_note`).
- [x] 4.9 Modo feedback (Ajustes → Agentes de IA): herramienta MCP `report_feedback` que acumula en
  `~/.config/synapsers/feedback.md` lo que le faltó al agente. Solo con la feature de Cargo `develop`.
- [x] 4.10 Historial de conversaciones en `<bóveda>/.synapse/agent-sessions/<id>.json`: retomar
  (`session/load` si el agente lo admite, si no resumen como contexto), nueva conversación, abrir en
  crudo con la aplicación por defecto y borrar una o todas.
- [x] 4.11 Respuestas en markdown saneado, herramientas expandibles con `rawInput`/`rawOutput`,
  cabecera alineada con la barra de pestañas y panel acoplado compatible con el tema Islas.

## 3. Verificación

- [x] 3.1 Ejecutar `bun run check` (0 errores, 0 warnings).
- [x] 3.2 Ejecutar `bun run build`.
- [x] 3.3 Ejecutar `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 3.4 Actualizar `CHANGELOG.md` bajo `## [Sin publicar] (Unreleased)`.
