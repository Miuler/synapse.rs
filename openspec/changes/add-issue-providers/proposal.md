# Proposal

## Why

Los tableros de tareas (`.board.md`) solo se sincronizan con Azure DevOps, con una única credencial global y una
sintaxis propia en el frontmatter. Quien trabaja con varios gestores (o con varias cuentas del mismo) no puede
tener un tablero por cada uno, y desde otras notas no hay forma de ver cómo va un tablero o una tarea sin abrirlo.

## What Changes

- Una capa de abstracción de **proveedores de issues** con un modelo común (clave, título, tipo, estado, asignado,
  comentarios, histórico) y las capacidades que cada uno declara. Adaptadores: Azure DevOps (el actual, movido detrás
  de la capa), GitHub y Jira.
- **Conexiones** con nombre, configuradas en Synapse: al crear una se elige el proveedor y se piden sus datos. Puede
  haber varias, también varias del mismo proveedor con distinto servidor, cuenta o credencial. La credencial va al
  llavero del sistema o es una referencia `op://` de 1Password.
- Cada tablero elige **una** conexión, su ámbito (proyecto o repositorio) y un **filtro**, que se pide al crearlo; así
  varios tableros pueden repartirse el trabajo de un mismo proyecto. Conserva en sus archivos todas las referencias al
  proveedor: se puede leer sin conexión y en otro equipo.
- **Un archivo por tarea**, para tableros grandes: `Nombre.board.md` define el tablero (origen, filtro, columnas y
  orden) y la carpeta `Nombre.board/` guarda cada tarea con sus datos, descripción y actividad. `Nombre` es cualquier
  nombre y puede haber varios tableros.
- **Carpeta protegida**: la de un tablero se puede consultar pero no modificar a mano, como un archivo con candado que
  no se puede quitar. Solo la escriben las operaciones del tablero.
- **BREAKING** (respecto a lo aún sin publicar): la tabla y los comentarios del `.board.md` pasan a la carpeta del
  tablero, y la clave `azure_devops` y el token global se sustituyen por `source` y por las conexiones; se convierten
  con confirmación al abrirlos.
- **Todo editable desde el tablero** (Kanban, tabla, Gantt y detalle), con envío al proveedor de lo que admita.
  Cambiar el histórico se advierte antes y solo afecta a la copia local.
- **El agente ayuda a configurar**: con la configuración del tablero abierta se puede pedir al agente integrado que
  escriba la consulta, la pruebe y la corrija, mediante herramientas MCP privadas; guardar sigue siendo del usuario.
- **Referencias desde cualquier Markdown**: un enlace a una tarea se pinta con su estado, y un bloque `board` muestra
  un tablero entero o filtrado (resumen, lista, tabla o Kanban), con la fecha de los datos.

## Capabilities

### New Capabilities

- `issue-providers`: conexiones configurables a gestores de issues, su credencial y el contrato común que cumplen.
- `board-sync`: el tablero vinculado a una conexión y un filtro, cómo se guarda y se protege, y cómo trae y envía
  cambios.
- `config-assistant`: el agente integrado ayuda a configurar Synapse con herramientas privadas, empezando por el
  origen de un tablero.
- `board-references`: mostrar tableros y tareas, con su estado, dentro de otras notas Markdown.

### Modified Capabilities

Ninguna: la única especificación publicada es `acp-agent`.

## Impact

- Rust (`src-tauri/src`): puerto `IssueProvider` y modelo en `domain`/`application`; `azure_devops_client.rs` pasa a
  ser un adaptador más; adaptadores nuevos para GitHub y Jira; configuración en `config.jsonc`; `secret_store.rs`
  guarda una credencial por conexión; índice en memoria de las tareas de cada tablero, derivado de sus archivos;
  protección de sus carpetas en las escrituras; herramientas MCP privadas por sesión de agente.
- Frontend (`src`): `features/board-editor` deja de conocer a Azure DevOps y de leer un solo archivo; ajustes de
  conexiones en `widgets/settings-panel`; configuración del tablero no modal junto al chat del agente; candado fijo en el explorador y las pestañas; render de referencias en
  `features/markdown-editor` (lectura y vista en vivo).
- Formato `.board.md`: cambia; los tableros actuales se convierten con confirmación.
- Dependencias: ninguna nueva prevista (`dashmap`, `notify`, `reqwest`, `keyring`, `yaml` y `dompurify` ya están).
