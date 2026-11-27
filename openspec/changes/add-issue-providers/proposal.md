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
- Cada tablero elige **una** conexión y su ámbito (proyecto o repositorio y consulta), y conserva en su archivo todas
  las referencias a ese proveedor: se puede leer sin conexión y en otro equipo.
- **BREAKING** (respecto a lo aún sin publicar): la clave `azure_devops` del frontmatter y el token global se
  sustituyen por `source` y por las conexiones; se migran al abrirlos.
- Formato de tablero ampliado: columnas leídas por su nombre (referencia, tipo, asignado…) y actividad por tarea con
  un campo de clase (`comment` o `history`).
- **Referencias desde cualquier Markdown**: un enlace a una tarea se pinta con su estado, y un bloque `board` muestra
  un tablero entero o filtrado (resumen, lista, tabla o Kanban), con la fecha de los datos.

## Capabilities

### New Capabilities

- `issue-providers`: conexiones configurables a gestores de issues, su credencial y el contrato común que cumplen.
- `board-sync`: el tablero vinculado a una conexión, lo que guarda de cada tarea y cómo trae y envía cambios.
- `board-references`: mostrar tableros y tareas, con su estado, dentro de otras notas Markdown.

### Modified Capabilities

Ninguna: la única especificación publicada es `acp-agent`.

## Impact

- Rust (`src-tauri/src`): puerto `IssueProvider` y modelo en `domain`/`application`; `azure_devops_client.rs` pasa a
  ser un adaptador más; adaptadores nuevos para GitHub y Jira; configuración en `config.jsonc`; `secret_store.rs`
  guarda una credencial por conexión.
- Frontend (`src`): `features/board-editor` deja de conocer a Azure DevOps; ajustes de conexiones en
  `widgets/settings-panel`; render de referencias en `features/markdown-editor` (lectura y vista en vivo).
- Formato `.board.md`: cambia; los tableros actuales siguen abriéndose.
- Dependencias: ninguna nueva prevista (`reqwest`, `keyring`, `yaml` y `dompurify` ya están).
- Parte del trabajo de Azure DevOps y del llavero está sin commit: conviene confirmarlo antes de empezar.
