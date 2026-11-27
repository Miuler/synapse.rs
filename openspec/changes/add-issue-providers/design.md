# Design

## Context

Ver `proposal.md` — Why. Estado del código del que se parte:

- Rust ya tiene un puerto (`application/service/work_item_tracker.rs`), cuatro casos de uso y un adaptador
  (`infrastructure/service/http/azure_devops_client.rs`). El puerto asume `project_url`, ids numéricos y una revisión
  entera: son conceptos de Azure DevOps.
- La credencial es global (llavero, `azure_devops.token` con `op://`, o variables de entorno) y la URL del proyecto
  sale del archivo del tablero, protegida por una lista de servidores admitidos.
- `features/board-editor` guarda la configuración en `azure_devops` del frontmatter, lee la tabla por posición y los
  comentarios como una línea cada uno.
- La lectura Markdown ya convierte bloques de código en tarjetas (Mermaid, D2) y la vista en vivo usa widgets de
  CodeMirror; `![[nota#Título]]` incrusta secciones.

## Goals / Non-Goals

**Goals:**

- Añadir un proveedor sin tocar el tablero ni las referencias.
- Un tablero legible y consultable sin red, sin credencial y en otro equipo.
- Que un archivo de la bóveda no pueda decidir a dónde se envía una credencial.

**Non-Goals:**

- Sincronización automática o en segundo plano, y webhooks.
- Editar tareas desde una referencia incrustada.
- Mezclar varios proveedores en un mismo tablero.
- GitHub Projects v2 en la primera versión del adaptador de GitHub.
- Arrastre del Kanban, panel de detalle y vista raw: son otro cambio (usan el formato que define este).
- Obtener credenciales con OAuth.

## Decisions

### 1. Un puerto en Rust y un adaptador por proveedor

```text
UI (tablero, ajustes, referencias)         no conoce proveedores
        │  órdenes `issues_*` (id de conexión + ámbito)
application: casos de uso ── IssueProvider (puerto)
        │
infrastructure: azure_devops │ github │ jira      + registro por `provider`
```

El puerto: `describe` (nombre, campos del formulario de conexión y de ámbito, capacidades), `check` (quién soy),
`states`, `search`, `get`, `create`, `update`, `activity` y `add_comment`. Cada adaptador traduce a y desde el
modelo común. Alternativa descartada: adaptadores en TypeScript; la credencial pasaría por el webview y dependerían
del CORS de cada servicio.

El formulario de conexión se genera de `describe`, así un proveedor nuevo no añade pantallas.

### 2. Modelo común con estado normalizado y capacidades

```text
Issue  { id, key, url, type, title, state { name, category }, assignee { name, id },
         labels[], updated_at, revision?, extra{} }
Entry  { id, kind: comment | history, author, at, text, changes[] }
```

`category` es `todo`, `doing`, `done` o `cancelled`: permite columnas por defecto, resúmenes y filtros iguales para
todos, sin perder el nombre real del estado. `revision` es texto opaco.

Las diferencias se declaran como capacidades, no se disimulan:

| | Azure DevOps | GitHub | Jira |
|---|---|---|---|
| Referencia | `AB#123` | `dueño/repo#5` | `PROJ-12` |
| Ámbito y consulta | proyecto, WIQL | repositorio, búsqueda | proyecto, JQL |
| Cambiar de estado | escribir el campo | abrir/cerrar; columnas por etiqueta | transición permitida |
| Conflictos | revisión | fecha de cambio | fecha de cambio |
| Histórico | actualizaciones | cronología | registro de cambios |
| Texto enriquecido | HTML | Markdown | documento JSON (ADF) |

Los endpoints de GitHub y Jira, y los de actividad de Azure DevOps, están **por verificar** en su documentación
al implementar cada adaptador.

Alternativa descartada: quedarse con lo común a los tres; perdería tipos, transiciones e histórico.

### 3. Conexiones globales; credencial por conexión

En `~/.config/synapsers/config.jsonc`, nunca en la bóveda:

```jsonc
"issues": {
  "connections": [
    { "id": "ado-trabajo", "provider": "azure_devops", "name": "Trabajo",
      "server": "https://dev.azure.com/organizacion", "secret": "op://bóveda/elemento/campo" },
    { "id": "ado-cliente", "provider": "azure_devops", "name": "Cliente",
      "server": "https://dev.azure.com/otra" },            // sin `secret`: en el llavero
    { "id": "gh", "provider": "github", "name": "Personal", "server": "https://api.github.com" },
    { "id": "jira", "provider": "jira", "name": "Equipo",
      "server": "https://empresa.atlassian.net", "user": "persona@empresa.example" }
  ]
}
```

En el llavero, entrada `issues/<id>` del servicio `synapsers`. La credencial solo se envía al `server` de su
conexión, por HTTPS (HTTP solo si la conexión apunta a este equipo). **Cambiar el servidor de una conexión borra su
credencial** y obliga a ponerla de nuevo: así ni un script del webview puede redirigir un token guardado. Esto
sustituye a la lista de servidores admitidos y a `AZURE_DEVOPS_URL`.

Alternativa descartada: conexiones por bóveda; la bóveda puede compartirse y no es de fiar.

### 4. El tablero se vincula a una conexión

```yaml
type: board
source:
  connection: ado-trabajo
  provider: azure_devops
  server: https://dev.azure.com/organizacion   # informativo: para reconocerla en otro equipo
  scope: { project: Demo }
  query: ""
  synced_at: 2026-11-26T22:00:00-05:00
states: { todo: [To Do], doing: [In Progress], done: [Done] }
fields: { card: [key, type, assignee], table: [key, type, title, state, assignee] }
links: { ado-123: { id: "123", revision: "4", state: "To Do", column: todo, title: "…" } }
```

Con tareas vinculadas, la conexión y el ámbito quedan fijos; cambiarlos exige desvincular. Si el equipo no tiene esa
conexión, el tablero se abre igual y ofrece elegir una del mismo proveedor y servidor, o crearla.

### 5. Formato: tabla por cabecera y actividad con clase

Las columnas se reconocen por su nombre y las desconocidas se conservan. La actividad va en `## Actividad`:

```markdown
## Actividad

### ado-123
- `2026-11-26T10:00:00` comment · Ana · id:987
  Primera línea del comentario.
  Segunda línea.
- `2026-11-26T11:00:00` history · Luis · id:rev-5
  Estado: To Do → In Progress
```

El `id` remoto evita volver a enviar lo traído. La sección antigua `## Comentarios` se lee y se convierte al
guardar. Los encabezados son parte del formato, no texto de la interfaz. Alternativa considerada: un archivo por
tarea; reduce el tamaño del tablero y los conflictos en carpetas sincronizadas, pero multiplica archivos. Se
mantiene un solo archivo y se limita el histórico traído por tarea.

### 6. Referencias: enlace para una tarea, bloque para consultas

Una tarea, dentro de una frase:

```markdown
El despliegue depende de [[Equipo.board#ado-123]].
```

El ancla es el identificador de la tarea en el tablero (columna `id`: `ado-123`, `gh-5`, `PROJ-12`, `t1`), que nunca
lleva `#` ni `/`; así también vale para tareas sin proveedor. La referencia del proveedor (`AB#123`,
`dueño/repo#5`) no sirve de ancla: con varios `#`, Synapse toma solo el último tramo. En otras aplicaciones el
enlace lleva al tablero, no a la tarea.

Un tablero entero o filtrado:

````markdown
```board
board: Equipo.board
view: summary            # summary | list | table | kanban
filter:
  state: [doing, todo]   # categoría, columna o estado remoto
  type: [Bug]
  assignee: Ana
  key: [AB#123, AB#124]
group_by: assignee
sort: updated desc
limit: 20
fields: [key, title, state, assignee]
```
````

- Los datos salen **del archivo del tablero**: pintar una nota nunca usa la red ni la credencial.
- Cada bloque dice la fecha de `synced_at` y ofrece «Actualizar», que es el mismo «Traer» del tablero y requiere
  un clic.
- `summary` muestra recuento y barra de avance por categoría.
- El texto del proveedor se muestra saneado (`dompurify`), sin imágenes remotas.

Alternativas descartadas: `![[tablero?filtro]]` (sintaxis ajena a Markdown y difícil de leer) y un lenguaje de
consultas tipo Dataview (mucho más trabajo del que piden estos filtros).

### 7. Escrituras

Se mantiene lo que ya hace «Traer»/«Enviar»: dos acciones separadas, revisión antes de enviar, conflictos sin
sobrescribir y creación solo de lo marcado, ahora comunes a todos los proveedores.

## Risks / Trade-offs

- [El modelo común no encaja en un proveedor] → capacidades y `extra`; Azure DevOps primero, GitHub después para
  validar el contrato antes de Jira.
- [Tableros grandes con histórico] → límite por tarea y traer actividad solo de lo que cambió.
- [Datos viejos en una referencia] → fecha visible y «Actualizar».
- [Texto de terceros en el webview] → saneado y sin recursos remotos.
- [Sin acceso a cuentas reales en las pruebas] → servidores falsos por proveedor y una comprobación manual de solo
  lectura antes de dar cada adaptador por bueno.
- [Límites de peticiones] → consultas por lotes y mensajes claros al llegar al límite.

## Migration Plan

1. Al abrir un tablero con `azure_devops`, se propone la conexión cuyo servidor coincida o se crea «Azure DevOps»
   con la credencial global existente; al guardar se escribe `source`.
2. `azure_devops.token`, la entrada `azure-devops-token` del llavero y las variables `AZURE_DEVOPS_PAT`/`ADO_PAT`
   solo se usan para crear esa primera conexión.
3. Volver atrás: el tablero sigue siendo una tabla Markdown; basta restaurar el archivo desde el control de versiones.

## Open Questions

- Si GitHub Projects v2 se añade como segundo tipo de ámbito del adaptador de GitHub.
- Si las referencias se ofrecen también a los agentes como herramienta MCP de solo lectura.
