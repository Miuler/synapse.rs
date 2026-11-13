# Design: Integración de Agentes ACP en Synapse

## 1. Arquitectura Backend (Rust - Arquitectura Cebolla)

- **`domain/acp/`**:
    - Entidades: `AcpAgentProfile` (id, nombre, origen verificado/personalizado, comando, args, env), `AcpSession`,
      `AcpMessage`, `AcpPermissionRequest`.
    - Puertos (traits): `AcpClientPort` (iniciar, detener, enviar prompt, responder permiso), `AcpAgentRegistryPort`
      (listar catálogo, verificar instalación, guardar perfiles personalizados), `AcpInstallerPort` (descargar e
      instalar binarios verificados).
- **`application/use_cases/acp/`**:
    - Casos de uso: `StartAcpSession`, `SendAcpPrompt`, `RespondAcpPermission`, `StopAcpSession`, `InstallAcpAgent`,
      `ManageAcpProfiles`.
- **`infrastructure/`**:
    - `infrastructure/service/acp/`: Implementación de transporte `stdio` usando `tokio::process::Command` con
      lectura/escritura asíncrona línea por línea (JSON-RPC) y emisión de eventos Tauri (`acp://stream`,
      `acp://tool-call`, `acp://permission-request`, `acp://status`).
    - `infrastructure/service/acp/installer.rs`: Descargador de binarios verificados (Antigravity, Claude) hacia el
      directorio de datos de la aplicación.
    - `infrastructure/entrypoint/tauri/acp_commands.rs`: Comandos `#[tauri::command]` expuestos al frontend.

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
