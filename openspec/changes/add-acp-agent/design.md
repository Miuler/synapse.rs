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
      y descarga hacia el directorio de datos de la aplicación (ver «Catálogo e instalación»).
    - `infrastructure/repository/jsonc/agent_profile_repository_impl.rs`: perfiles personalizados en la configuración.
    - `infrastructure/entrypoint/controller/tauri/agent_controller.rs`: comandos `#[tauri::command]` que convierten la
      entrada en `Command`/`Query` y llaman al caso de uso.
    - `infrastructure/config/`: ensamblado de los casos de uso con sus implementaciones.

### Catálogo e instalación (decisión)

- **Fuente:** el registro público de ACP, el mismo que usa Zed:
  `https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`. Synapse solo ofrece como verificados los ids
  de una lista blanca (`VERIFIED_AGENTS`); se agregan de a uno tras probarlos. Caché en memoria de 1 h y copia en disco
  para trabajar sin red; si el registro falla, los agentes personalizados se siguen listando.
- **Instalación:** se usa la distribución `binary` de la plataforma (`linux-x86_64`, `darwin-aarch64`…): se descarga el
  archivo, se **verifica su `sha256`** del registro, se descomprime (`.zip`, `.tar.gz` o binario suelto) en
  `<datos>/synapsers/agents/<id>/<versión>/` y se guarda cómo lanzarlo. Sin binario para la plataforma se recurre a la
  distribución `npx` (requiere Node).
- **Perfiles personalizados:** `~/.config/synapsers/agents.json`.
- **Agentes verificados:** **Claude** (`claude-acp`) desde el registro. **Antigravity** se usa desde su CLI oficial ya
  instalado: el subcomando `agy acp`, buscado en el PATH y en `~/.local/bin` (las apps gráficas no siempre heredan el
  PATH), sin descarga; no se usa el paquete `antigravity-acp` del registro, que pasa un `--uid=` sin documentar. Si falta
  el CLI se muestra su comando oficial de instalación. Usa las credenciales que ya guardó `agy`; si responde
  «Authentication required» (`-32000`), se pide al usuario iniciar sesión desde el CLI.

### MCP de la instancia (decisión)

- Cada sesión recibe el MCP **de la ventana de Synapse abierta**, con el nombre `synapse_instance` para no confundirse
  con un `synapse` configurado globalmente en el agente. Se arranca si estaba apagado.
- Transporte: HTTP directo si el agente anuncia `mcpCapabilities.http`; si no, `synapsers --mcp-proxy <url>`, un puente
  stdio→HTTP hacia la misma instancia (todos comparten índice y estado con la ventana).
- ACP no tiene «system prompt»: las instrucciones de sesión viajan como un bloque de texto con el primer mensaje. Piden
  usar `synapse_instance` y crear/editar/mover/borrar notas con sus herramientas, no con escritura directa de archivos.
- Los agentes no están aislados: usan su configuración global (credenciales, ajustes y otros MCP); Synapse solo fija la
  carpeta de trabajo, el MCP de la instancia y las variables de los agentes personalizados.
- Modo feedback (solo con la feature de Cargo `develop`): herramienta MCP `report_feedback` → `~/.config/synapsers/feedback.md`.

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
