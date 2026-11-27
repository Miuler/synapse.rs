# Design

## Context

Ver `proposal.md` — Why. Estado del código del que se parte:

- Rust ya tiene un puerto (`application/service/work_item_tracker.rs`), cuatro casos de uso y un adaptador
  (`infrastructure/service/http/azure_devops_client.rs`). El puerto asume `project_url`, ids numéricos y una revisión
  entera: son conceptos de Azure DevOps.
- La credencial es global (llavero, `azure_devops.token` con `op://`, o variables de entorno) y la URL del proyecto
  sale del archivo del tablero, protegida por una lista de servidores admitidos.
- `features/board-editor` recibe el contenido de un solo archivo: guarda la configuración en `azure_devops` del
  frontmatter (incluidos los vínculos de cada tarea), los anchos en `column_widths`, lee la tabla por posición y los
  comentarios como una línea cada uno.
- El candado actual es estado de la interfaz (`lockedFiles` en el estado de la bóveda): el usuario lo pone y lo quita
  por archivo.
- El motor de navegación ya mantiene `notes: DashMap` y un observador de la bóveda avisa de los cambios.
- Medido con tableros sintéticos: leer 12 MB de texto tarda unos 35 ms, pero la vista de tabla tarda unos 10 s con
  1000 tareas.
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
- Archivar tareas cerradas y virtualizar las vistas (el rendimiento de la tabla se trata aparte).
- Impedir que otras aplicaciones modifiquen los archivos del tablero.
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
Issue  { id, key, url, type, title, description, state { name, category }, assignee { name, id },
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

En el llavero, entrada `issues-<id>` del servicio `synapsers` (el id solo admite `[a-z0-9-]`). La credencial solo se envía al `server` de su
conexión, por HTTPS (HTTP solo si la conexión apunta a este equipo). **Cambiar el servidor de una conexión borra su
credencial** y obliga a ponerla de nuevo: así ni un script del webview puede redirigir un token guardado. Esto
sustituye a la lista de servidores admitidos y a `AZURE_DEVOPS_URL`.

Alternativa descartada: conexiones por bóveda; la bóveda puede compartirse y no es de fiar.

### 4. El archivo del tablero solo define el tablero

`Nombre.board.md` guarda en el frontmatter la configuración y en el cuerpo el orden de las tareas. La marca
`format: 2` lo distingue del formato anterior (una tabla). Lo leen y escriben funciones puras de Rust, que son las
que usan el índice, la protección, la conversión y las referencias; la interfaz recibe estructuras.

````markdown
---
type: board
format: 2
default_view: kanban
columns:
  - { id: "todo", label: "Por hacer", color: "var(--text-muted)" }
column_widths: { todo: 320 }
source:
  provider: azure_devops
  connection: ado-trabajo          # sin ella, la que cubra el proyecto
  scope: https://dev.azure.com/organizacion/Demo
  filter: { type: [Bug, Task], assignee: ["@me"], iteration: ["@current"] }
  query: "SELECT [System.Id] …"    # la compilada del filtro, o una propia
  filter_query: "SELECT [System.Id] …"
  states: { todo: [To Do], doing: [In Progress], done: [Done] }
fields: { card: [key, type, assignee], table: [key, type, title, state, assignee] }
---

## Order

- ado-123
- t1
````

El orden usa solo identificadores (la carpeta se sobrentiende), así que renombrar el tablero no obliga a reescribirlo.

Nada de lo propio de una tarea va en el frontmatter. Con tareas vinculadas, la conexión y el ámbito quedan fijos;
cambiarlos exige desvincular. Si el equipo no tiene esa conexión, el tablero se abre igual y ofrece elegir una del
mismo proveedor y servidor, o crearla.

### 5. Un archivo por tarea en la carpeta del tablero

```text
Equipo.board.md
Equipo.board/
    ado-123.md
    t1.md
```

El nombre del archivo es el identificador de la tarea (`ado-123`, `gh-5`, `PROJ-12`, `t1`): letras ASCII, cifras,
`-` y `_`, único sin distinguir mayúsculas y distinto de `index`. `sync` es JSON en una línea y gana los campos que
necesite el envío (tipo, asignado y huella de la descripción) al implementarlo. Los datos van en el
cuerpo como lista, sin frontmatter; las claves y los encabezados son parte del formato, en inglés:

```markdown
# Publicar el servicio

- status: doing
- type: Bug
- assignee: Ana
- key: AB#123
- url: https://…
- start: 2026-11-20
- duration: 2d
- depends: -
- progress: 0%
- sync: {"id":"123","rev":"4","title":"…","state":"In Progress","column":"doing","sent":[],"outside":false}

## Description

> El texto, como cita para que sus encabezados no rompan el formato.

## Activity

- `2026-11-26T10:00:00` comment · Ana · id:987
  Primera línea del comentario.
- `2026-11-26T11:00:00` history · Luis · id:rev-5
  Estado: To Do → In Progress
```

Las claves y secciones desconocidas se conservan (las columnas propias de una tabla anterior pasan a ser claves); el `id` remoto evita volver a enviar lo traído.

Rust mantiene un índice en memoria (`DashMap` por carpeta de tablero) con lo que necesitan las vistas. Se valida con
la fecha y el tamaño de cada archivo: leer el tablero solo vuelve a interpretar lo que cambió, venga el cambio de
Synapse o de fuera, sin depender de que el observador no pierda avisos (el observador solo servirá para que la
interfaz vuelva a leer). La descripción y la actividad se leen al abrir la tarea. El frontend pide el tablero en una
sola orden. Sin base de datos: con 5000 tareas sintéticas, leer el tablero tarda unos 0,1 s y envía 2 MB a la
interfaz (compilación de desarrollo, archivos ya en la caché del sistema).

Alternativas descartadas: un solo archivo (con un equipo grande crece a megabytes, se reescribe entero en cada cambio
y choca al sincronizar carpetas); SQLite (otra fuente de datos que mantener sin consultas que la justifiquen); datos
en el frontmatter de cada tarea (el usuario quiere lo propio de la tarea en su detalle).

### 6. Referencias: enlace para una tarea, bloque para consultas

Una tarea, dentro de una frase:

```markdown
El despliegue depende de [[Equipo.board/ado-123]].
```

Es un enlace normal al archivo de la tarea, así que en otras aplicaciones lleva a ella. Incluye la carpeta del
tablero porque dos tableros pueden tener la misma tarea o el mismo identificador local.

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
sobrescribir y creación solo de lo marcado, ahora comunes a todos los proveedores. «Traer» pide lo que cambió desde
`synced_at` y escribe solo los archivos que cambian. `sync` guarda la base de cada campo editable (de la descripción,
su huella) para distinguir quién cambió qué.

### 8. Carpeta del tablero protegida

Una carpeta `Nombre.board/` con su `Nombre.board.md` al lado está protegida. La protección se deduce de la ruta: no se
guarda ni se puede quitar. Sus archivos se abren en lectura con el candado y sin la acción de desbloquear.

Se comprueba en Rust, en un único punto por el que pasan las escrituras (editor, explorador, «+ Nuevo…», archivos
soltados, pestañas, herramientas MCP): todas la rechazan salvo las órdenes del tablero. Renombrar el tablero renombra su carpeta, y borrarlo pregunta
si se borra también. Es una defensa contra errores, no un control de acceso: otras aplicaciones pueden modificar los
archivos, y una carpeta sin su tablero deja de estar protegida.

Alternativas descartadas: reutilizar la lista `lockedFiles` (se puede quitar y no cubre explorador ni MCP); permisos
del sistema de archivos (estorban a la sincronización de carpetas y a Git).

### 9. Filtro al crear el tablero

Al crear un tablero con origen se eligen conexión, ámbito y filtro antes de traer nada. El proveedor declara en
`describe` los criterios que admite (tipo, estado, asignado, iteración, etiqueta o área) y también se puede escribir
su consulta. El filtro se puede cambiar después: las tareas que dejan de cumplirlo no se borran, se marcan como fuera
del filtro. Un tablero sin origen no pide filtro.

### 10. Asistente de configuración con herramientas MCP privadas

La configuración del origen de un tablero se muestra en una vista **no modal**, para usar a la vez el chat del agente
(el diálogo actual lo bloquea). Las herramientas, con prefijo `config_`:

| Herramienta | Qué hace |
|---|---|
| `config_get_context` | Vista abierta y borrador: conexión (nombre, proveedor, servidor), ámbito, filtro o consulta, estados y columnas |
| `config_describe_source` | Lenguaje de consulta, criterios y valores admitidos (tipos, estados, iteraciones, personas) |
| `config_test_query` | Ejecuta en solo lectura un filtro o consulta: recuento y hasta 20 filas (referencia, tipo, título, estado, asignado), o el error del proveedor |
| `config_update_draft` | Cambia filtro, consulta o estados **del borrador**; la vista marca lo cambiado y el usuario guarda o descarta |

Ninguna lee ni escribe credenciales, cambia servidores o conexiones, guarda, trae ni envía.

«Privadas»: el servidor MCP actual solo escucha en local y no pide autenticación (límite conocido), así que estas
herramientas MUST NOT publicarse en él sin más. Solo se listan y se atienden para las sesiones de los
agentes que abre Synapse, identificadas con un token por sesión (como el que ya usa el canal entre instancias),
entregado en `mcpServers` de `session/new` o `session/load` (cabecera HTTP, o variable de entorno del puente stdio,
nunca en los argumentos), sin respuestas CORS. Sirven en una conversación ya existente al reanudarla con el token, y
fuera de la vista de configuración responden que no hay nada abierto.

Alternativas descartadas: herramientas en el MCP público (cualquier proceso local podría usar la conexión); dar la
credencial al agente para que consulte él (la expone); que el agente guarde directamente (un texto traído del
proveedor podría inducir un cambio sin revisión). El primer conjunto cubre el origen del tablero; otras
configuraciones se añadirán con la misma regla.

### 11. Todo editable desde el tablero

El detalle de la tarea edita todos sus campos y comentarios; la tabla, el Kanban y el Gantt mantienen su edición
directa y abren ese mismo detalle. Son órdenes del tablero, así que escriben en la carpeta protegida. Los comentarios
editados se envían como actualización si el proveedor lo permite.

El histórico del proveedor no se puede reescribir: editarlo cambia solo la copia local, tras un aviso, y la entrada
queda con la marca `edited` en su primera línea (`… history · Luis · id:rev-5 · edited`), que impide pisarla al traer.

La descripción se guarda como Markdown convertido desde el formato del proveedor. La conversión de vuelta puede perder
formato (tablas, imágenes, menciones): solo se envía si se editó, con aviso en la revisión.

Alternativa descartada: histórico de solo lectura (el usuario quiere poder corregirlo).

## Risks / Trade-offs

- [El modelo común no encaja en un proveedor] → capacidades y `extra`; Azure DevOps primero, GitHub después para
  validar el contrato antes de Jira.
- [Miles de archivos pequeños en una carpeta sincronizada] → escribir solo lo que cambia; medir la primera apertura.
- [Un cambio del tablero toca varios archivos y puede quedar a medias] → escribir cada archivo de forma atómica, el
  tablero al final, y reconstruir el orden con lo que haya en la carpeta.
- [La misma tarea en dos tableros] → cada uno guarda su copia y la actualiza al traer.
- [Datos viejos en una referencia] → fecha visible y «Actualizar».
- [Editar la descripción borra formato en el proveedor] → enviar solo lo editado, avisar y exigir que se marque.
- [Un histórico editado deja de reflejar lo que pasó] → aviso, marca `edited` y ninguna escritura en el proveedor.
- [Texto de terceros en el webview] → saneado y sin recursos remotos.
- [Sin acceso a cuentas reales en las pruebas] → servidores falsos por proveedor y una comprobación manual de solo
  lectura antes de dar cada adaptador por bueno.
- [Límites de peticiones] → consultas por lotes y mensajes claros al llegar al límite.
- [Las filas de una prueba llegan al proveedor del modelo del agente] → solo esos campos, sin descripción ni
  actividad, y la vista lo advierte.
- [Texto de terceros que intente dirigir al agente] → se entrega como datos y ningún cambio se guarda sin el usuario.
- [Una conversación reanudada que no admita cambiar sus servidores MCP] → verificar por agente y, si no, abrir una
  conversación nueva desde la vista.

## Migration Plan

1. Al abrir un tablero con tabla, se ofrece convertirlo: crea la carpeta con un archivo por fila (con sus comentarios
   y vínculos) y deja en el `.board.md` la configuración y el orden. No se convierte sin confirmación.
2. Si tiene `azure_devops`, se propone la conexión cuyo servidor coincida o se crea «Azure DevOps» con la credencial
   global existente, y su consulta pasa a ser el filtro.
3. `azure_devops.token`, la entrada `azure-devops-token` del llavero y las variables `AZURE_DEVOPS_PAT`/`ADO_PAT`
   sirven mientras no haya ninguna conexión (una implícita, solo hacia la nube de Azure DevOps) y para copiarlos a
   una conexión de esa nube cuando el usuario lo pida. Leer nunca reescribe `config.jsonc` y nada anterior se borra,
   para que la versión instalada siga funcionando.
4. Volver atrás: restaurar el `.board.md` desde el control de versiones o la copia previa y borrar la carpeta.

## Open Questions

- Si GitHub Projects v2 se añade como segundo tipo de ámbito del adaptador de GitHub.
- Si las referencias se ofrecen también a los agentes como herramienta MCP de solo lectura.
