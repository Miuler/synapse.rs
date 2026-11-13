# Proposal: Integración de Agentes ACP por `stdio` en Synapse

## Why

Los usuarios de Synapse quieren interactuar con agentes de IA (como Antigravity o Claude) directamente desde su espacio
de trabajo sin tener que instalar ni gestionar binarios manualmente por fuera de la aplicación. Al adoptar el protocolo
**ACP (Agent Client Protocol)** sobre `stdio`, Synapse actúa como cliente/interfaz unificada capaz de lanzar, gestionar
y comunicarse con múltiples agentes compatibles.

## What Changes

- **Registro y descarga automática de agentes ACP**:
    - Catálogo verificado integrado con soporte inicial para **Antigravity** y **Claude**, con descarga e instalación
      automática en el directorio gestionado de Synapse.
    - Soporte opcional para registrar agentes personalizados indicando comando, argumentos y variables de entorno.
- **Cliente ACP por `stdio` en Backend Rust (Arquitectura Cebolla)**:
    - Gestión del ciclo de vida del proceso hijo (`spawn`, `restart`, `stop`) y comunicación JSON-RPC sobre `stdin`/
      `stdout`.
    - Streaming de respuestas, manejo de llamadas a herramientas (*tool calls*) y solicitudes de permisos hacia el
      frontend mediante eventos Tauri.
- **Interfaz de Agente ACP en Frontend Svelte 5 (FSD)**:
    - Panel independiente ubicado por defecto a la **derecha** del workspace (fijo/acoplado, similar al `vault-explorer`
      a la izquierda), con opción de **alternar a modo flotante**.
    - Selector rápido en la cabecera del panel para cambiar de agente en cualquier momento.
    - Estado vacío guiado cuando no hay agentes configurados y acceso directo a la configuración completa en Ajustes de
      Synapse.

## Capabilities

- `acp-agent`: Gestión de catálogo/descarga de agentes ACP, transporte `stdio` JSON-RPC, sesiones de chat, permisos y
  panel derecho acoplado/flotante.

## Impact

- **Backend (`src-tauri/src/`)**: Nuevos módulos en `domain/`, `application/` e `infrastructure/` siguiendo Arquitectura
  Cebolla, más comandos Tauri en `infrastructure/entrypoint/tauri/`.
- **Frontend (`src/`)**: Nuevas slices en `entities/`, `features/` y `widgets/` siguiendo Feature-Sliced Design (FSD) y
  Svelte 5 Runes.
