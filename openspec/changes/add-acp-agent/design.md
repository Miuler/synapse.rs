# Design: Integración de Agentes ACP en Synapse

## 1. Arquitectura Backend (Rust - Arquitectura Cebolla)

Siguiendo `docs/arquitectura_cebolla.md`: el agente es un proceso externo cuyo protocolo dicta un tercero, así que es un
**servicio externo**; los perfiles los definimos nosotros, así que son un **repository**. El nombre del protocolo (ACP) y
de las herramientas solo aparece en `infrastructure`; las capas internas hablan de agentes.

- **`domain/agent/`**:
    - Entidades: `AgentProfile` (id, nombre, origen verificado/personalizado, comando, args, env), `AgentSession`,
      `AgentMessage`, `PermissionRequest`.
    - Repository: `AgentProfileRepository` (guardar, listar y borrar perfiles personalizados).
- **`application/use_case/`**: un caso de uso por intención, con un único `execute` y su `Command`/`Query` en
  `application/dto/`: `StartAgentSessionUseCase`, `SendAgentPromptUseCase`, `RespondPermissionUseCase`,
  `StopAgentSessionUseCase`, `InstallAgentUseCase`, `ListAgentsUseCase`, `SaveAgentProfileUseCase`,
  `DeleteAgentProfileUseCase`.
- **`application/integration/service/`** (contratos de servicios externos, nombrados por capacidad):
    - `AgentRunner`: iniciar y detener un agente, enviar un prompt y responder un permiso.
    - `AgentCatalog`: listar el catálogo verificado y comprobar si un agente está instalado.
    - `AgentInstaller`: descargar e instalar binarios verificados.
- **`infrastructure/`**:
    - `infrastructure/integration/service/acp/agent_runner_impl.rs`: transporte `stdio` con `tokio::process::Command`,
      lectura/escritura asíncrona línea por línea (JSON-RPC) y emisión de eventos Tauri (`acp://stream`,
      `acp://tool-call`, `acp://permission-request`, `acp://status`).
    - `infrastructure/integration/service/acp/agent_catalog_impl.rs` y `agent_installer_impl.rs`: catálogo verificado
      (Antigravity, Claude) y descarga hacia el directorio de datos de la aplicación.
    - `infrastructure/repository/jsonc/agent_profile_repository_impl.rs`: perfiles personalizados en la configuración.
    - `infrastructure/entrypoint/controller/tauri/agent_controller.rs`: comandos `#[tauri::command]` que convierten la
      entrada en `Command`/`Query` y llaman al caso de uso.
    - `infrastructure/config/`: ensamblado de los casos de uso con sus implementaciones.

## 2. Arquitectura Frontend (Svelte 5 + FSD)

- **`entities/acp-agent/`**:
    - `model/`: Store reactivo con Runes (`$state`, `$derived`) para la lista de agentes, agente activo, estado de
      conexión, mensajes en streaming y solicitudes de permiso pendientes.
    - `api/`: Wrappers tipados de `invoke` y `listen` de Tauri.
- **`features/acp-chat/`**:
    - `ui/`: Vista de chat, renderizado de streaming, tarjetas de *tool calls* y botones de aprobar/rechazar permisos,
      además del estado vacío cuando no hay agente configurado.
- **`widgets/acp-panel/`**:
    - `ui/`: Contenedor del panel con cabecera (selector rápido de agente, botón de modo fijo/flotante, botón `⚙` hacia
      Ajustes).
    - Integración en `pages/workspace/`: Por defecto acoplado como panel independiente a la derecha del editor; en modo
      flotante se renderiza como panel posicionable sobre el workspace.
