---
schema_version: 9
id: syna-ng5v
title: 'MCP: colaboración entre bóvedas (instancia correcta, conversaciones de bóvedas cerradas, despertar bóveda)'
priority: P2
status: open
deps: []
tags:
- mcp
- agente
owner: null
created_at: 2026-11-22T06:08:26.709148897Z
acceptance:
- 'is_this/instancia: el agente usa el MCP de su ventana y is_this es correcto, con pruebas'
- list_conversations puede listar conversaciones de bóvedas cerradas respetando el permiso
- open_vault con confirmación del usuario y send_to_conversation enrutado a la bóveda despertada
---

# Tarea: mejorar la colaboración entre bóvedas por MCP en Synapse (synapse.rs)

Repositorio: synapse.rs (Tauri 2 + Svelte 5, backend Rust con arquitectura cebolla:
domain / application / infrastructure; usa ese vocabulario). Responde y documenta en español.

## Contexto
Synapse abre «bóvedas» (carpetas de notas). Cada ventana levanta un servidor MCP propio y lo
inyecta a los agentes ACP de esa ventana con el nombre `synapse_instance`
(`src-tauri/src/infrastructure/integration/service/acp/agent_runner_impl.rs`, constante
`SYNAPSE_TOOLS`). Las herramientas están en
`src-tauri/src/infrastructure/entrypoint/mcp/tool_handler.rs`:
`list_vaults` (`execute_list_vaults`, devuelve `name`, `path`, `open`, `is_this`, `allowed`),
`list_conversations` (`execute_list_conversations`) y `send_to_conversation`.
Las conversaciones se guardan por bóveda en `<bóveda>/.synapse/agent-sessions/*.json`
(`agent_transcript_repository_impl.rs`).

## Feedback de un agente que lo usó
1. Con `list_vaults` vio las bóvedas y pudo leerlas en solo lectura pasando `vault: "..."`.
2. No pudo hablar con el agente de una bóveda cerrada (`open: false`): no hay forma de
   despertarla ni de que `send_to_conversation` la levante.
3. `list_conversations` solo lista las de ventanas abiertas; no sabe si una bóveda cerrada
   tenía conversaciones previas.
4. El contexto le indicaba usar `synapse_instance`, pero el MCP que tenía registrado se llamaba
   `synapse` y daba `is_this: true` en otra ventana (MisNotas) en vez de en la suya (BCP-APIs).
   La bóveda BCP-APIs se había creado hacía poco.

## Qué hacer (en este orden)
### A. Investigar y corregir el fallo de la instancia (punto 4)
- Reproduce con dos ventanas abiertas sobre bóvedas distintas.
- Comprueba qué MCP recibe cada agente: el inyectado (`synapse_instance`, de su ventana) o uno
  global del usuario llamado `synapse` (por ejemplo, de la configuración del propio agente,
  apuntando al puerto de otra ventana).
- Corrige lo que corresponda, por ejemplo:
  - que el agente use siempre el de su ventana;
  - que las instrucciones inyectadas digan cómo reconocerlo;
  - que `is_this` se calcule bien (rutas canónicas, bóveda recién creada).
- Añade pruebas.

### B. Listar conversaciones de bóvedas cerradas (punto 3)
- `list_conversations` (o un parámetro `vault`/`include_closed`) debe poder leer
  `.synapse/agent-sessions` de una bóveda cerrada, en solo lectura, con su título, agente, fecha
  y si está archivada.
- Respeta el permiso por bóveda que ya existe (`allowed` / `read_vault_access`).

### C. Despertar una bóveda cerrada (punto 2)
- Herramienta `open_vault` (o `wake_vault`) que pida permiso al usuario con el diálogo de
  confirmación MCP existente (`McpConfirm`) y abra la bóveda en una ventana nueva.
- Después, `send_to_conversation(to=…)` debe poder enrutar el mensaje a una conversación de esa
  bóveda (nueva o existente) cuando la ventana esté lista.
- Propón el diseño antes de implementarlo si hay dudas (¿ventana visible u oculta?,
  ¿qué agente usar?).

## Reglas
- No hagas commit ni push sin que el usuario lo apruebe. Al commitear, incluye la línea en
  `CHANGELOG.md`, sección «[Sin publicar]», en el mismo commit, y commitea solo tus archivos
  (hay otras sesiones trabajando en la misma copia).
- Pruebas Rust: `cd src-tauri && cargo test --lib`. E2E: siempre en la pantalla embebida,
  `bun run test:e2e:embebed:run`; para archivos concretos,
  `GDK_BACKEND=x11 DISPLAY=:100 bun e2e/quick.ts <nombre>`. Si la pantalla `:100` no existe,
  abre en segundo plano `bun run test:e2e:embebed:x11`.
  - El agente ACP falso de las e2e está en `e2e/fixtures/fake-acp-agent.ts`.
  - Toda bóveda de prueba necesita la carpeta `.synapse`.
- No modifiques las bóvedas reales del usuario. No mates procesos `synapsers` con patrones amplios.
- Al terminar, resume qué cambió, cómo se probó y qué queda pendiente.