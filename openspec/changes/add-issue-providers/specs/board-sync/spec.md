# Spec Delta

## Purpose

El tablero de tareas vinculado a una conexión de un gestor de issues: cómo se filtra, dónde guarda cada tarea para
poder consultarla sin red, cómo se protege y cómo trae y envía los cambios.

## ADDED Requirements

### Requirement: Un tablero, una conexión

Cada tablero sincronizado SHALL estar vinculado a una sola conexión, con su ámbito y su filtro, y SHALL conservar
ese vínculo en su archivo. Con tareas vinculadas, la conexión y el ámbito MUST NOT cambiar sin desvincularlas antes
con confirmación.

#### Scenario: Cambiar el origen de un tablero con tareas

- **GIVEN** un tablero con tareas vinculadas
- **WHEN** el usuario intenta elegir otra conexión
- **THEN** el sistema avisa de que se perderán los vínculos y no cambia nada sin confirmación

### Requirement: Filtro elegido al crear el tablero

Al crear un tablero con origen, el sistema SHALL pedir la conexión, el ámbito y el filtro antes de traer tareas, con
los criterios que admita el proveedor o su propia consulta. El filtro SHALL poderse cambiar después, y MUST NOT borrar
las tareas que dejen de cumplirlo.

#### Scenario: Segmentar un proyecto

- **GIVEN** dos tableros del mismo proyecto, uno filtrado por la persona «Ana» y otro por el tipo «Bug»
- **WHEN** se trae cada uno
- **THEN** cada tablero contiene solo las tareas de su filtro

#### Scenario: Cambiar el filtro

- **GIVEN** un tablero con tareas
- **WHEN** se cambia el filtro y se trae
- **THEN** las que ya no lo cumplen siguen en el tablero, marcadas como fuera del filtro

### Requirement: El archivo del tablero solo define el tablero

El `.board.md` SHALL guardar en el frontmatter únicamente la configuración del tablero y en el cuerpo el orden de sus
tareas, sin ninguna credencial. Los datos propios de una tarea MUST NOT guardarse en el frontmatter del tablero.

#### Scenario: Reordenar

- **WHEN** el usuario sube una tarjeta en el Kanban
- **THEN** cambia el orden en el archivo del tablero y no cambia el archivo de la tarea

### Requirement: Un archivo por tarea

El sistema SHALL guardar cada tarea en un archivo de la carpeta del tablero, nombrada como su archivo sin `.md`, con
el identificador de la tarea como nombre. El archivo SHALL contener su título, descripción, actividad y planificación
y, de cada tarea vinculada, su referencia, enlace, tipo, estado y asignado; el tablero SHALL guardar la fecha de los
datos. SHALL conservar lo que Synapse no reconozca. Ningún archivo MUST contener una credencial.

#### Scenario: Tablero y carpeta

- **WHEN** el tablero `Soporte.board.md` trae el work item 123
- **THEN** existe `Soporte.board/ado-123.md` con sus datos y su descripción

#### Scenario: Abrir en otro equipo

- **GIVEN** un tablero sincronizado y un equipo sin su conexión
- **WHEN** se abre el tablero
- **THEN** se ven las tareas con sus referencias, estados y descripciones
- **AND** se ofrece elegir una conexión del mismo proveedor y servidor, o crearla

### Requirement: Carpeta del tablero protegida

Los archivos de la carpeta de un tablero SHALL abrirse en lectura con un candado que MUST NOT poderse quitar. El
sistema MUST rechazar cualquier escritura en ella que pase por Synapse (editor, explorador, «+ Nuevo…», archivos
soltados, pestañas o herramientas MCP, entre otras) salvo las operaciones del tablero.

#### Scenario: Consultar una tarea

- **WHEN** el usuario abre `Soporte.board/ado-123.md`
- **THEN** ve su contenido en lectura con el candado
- **AND** no existe ninguna acción para desbloquearlo

#### Scenario: Borrar por error

- **WHEN** el usuario intenta borrar o renombrar un archivo de la carpeta desde el explorador
- **THEN** la acción no se realiza y se explica que pertenece al tablero

#### Scenario: Cambiar desde el tablero

- **WHEN** el usuario mueve esa tarea a otra columna en el Kanban
- **THEN** su archivo se actualiza

### Requirement: Actividad con su clase

Cada tarea SHALL guardar su actividad indicando si es un comentario o un cambio del histórico, con su autor, fecha e
identificador remoto.

#### Scenario: Traer actividad

- **WHEN** se trae una tarea con dos comentarios y un cambio de estado
- **THEN** su archivo guarda tres entradas, dos de clase `comment` y una de clase `history`

#### Scenario: Enviar después de traer

- **GIVEN** un tablero con comentarios traídos y uno escrito en Synapse
- **WHEN** el usuario envía los cambios
- **THEN** solo se envía el escrito en Synapse, y no se vuelve a enviar la vez siguiente

### Requirement: Todo se edita desde el tablero

Desde el Kanban, la tabla, el Gantt y el detalle de la tarea SHALL poderse actualizar sus datos: título, descripción,
tipo, asignado, estado, planificación y comentarios. El cambio SHALL guardarse en el archivo de la tarea.

#### Scenario: Editar la descripción

- **GIVEN** una tarea abierta en su detalle
- **WHEN** el usuario cambia la descripción y la guarda
- **THEN** su archivo la conserva, aunque la carpeta esté protegida

#### Scenario: Abrir el detalle desde cualquier vista

- **WHEN** el usuario abre una tarea desde el Kanban, la tabla o el Gantt
- **THEN** puede editar los mismos datos

### Requirement: Advertir al editar el histórico

Antes de cambiar una entrada del histórico, el sistema MUST advertir que solo cambia la copia local, no el registro
del proveedor, y MUST pedir confirmación. La entrada cambiada SHALL quedar marcada, y traer o enviar MUST NOT
sobrescribirla ni enviarla.

#### Scenario: Confirmar el cambio

- **GIVEN** una entrada del histórico traída del proveedor
- **WHEN** el usuario la edita y confirma el aviso
- **THEN** se guarda marcada como editada y sigue así después de traer

#### Scenario: Cancelar

- **WHEN** el usuario cancela el aviso
- **THEN** la entrada no cambia

### Requirement: Traer sin pisar cambios locales

Traer SHALL añadir las tareas nuevas y actualizar las que solo cambiaron en el proveedor; lo que cambió en los dos
lados MUST conservarse como está en el tablero y avisarse.

#### Scenario: Cambio en los dos lados

- **GIVEN** una tarea cuyo estado cambió en el tablero y en el proveedor
- **WHEN** se trae
- **THEN** se conserva el estado del tablero y se informa del conflicto

### Requirement: Enviar solo lo revisado

Enviar SHALL mostrar antes los cambios, incluidos los de descripción, tipo, asignado y comentarios, MUST NOT
sobrescribir un conflicto sin que se marque y SHALL crear en el proveedor solo las tareas que se marquen. Lo que el
proveedor no admita cambiar SHALL indicarse sin aplicarlo.

#### Scenario: Tarea local sin vínculo

- **GIVEN** un tablero con una tarea creada en Synapse
- **WHEN** el usuario abre la revisión de envío
- **THEN** la tarea aparece sin marcar y no se crea si se deja así

#### Scenario: Descripción con formato que se perdería

- **GIVEN** una descripción del proveedor con formato que Markdown no conserva, editada en Synapse
- **WHEN** el usuario revisa el envío
- **THEN** se advierte de la pérdida y el cambio no se envía si no lo marca

### Requirement: Convertir tableros anteriores

Un tablero con tabla SHALL abrirse sin perder contenido y ofrecer convertirlo al formato de carpeta, lo que MUST NOT
ocurrir sin confirmación.

#### Scenario: Convertir

- **GIVEN** un tablero con tabla, comentarios y la clave `azure_devops`
- **WHEN** el usuario confirma la conversión
- **THEN** cada tarea tiene su archivo con sus comentarios y su vínculo
- **AND** el tablero conserva columnas, orden y consulta

#### Scenario: Columnas propias en la tabla

- **GIVEN** un tablero con tabla que tiene una columna que Synapse no conoce
- **WHEN** se convierte
- **THEN** el archivo de cada tarea conserva ese dato con el nombre de la columna

#### Scenario: Sin confirmar

- **WHEN** el usuario rechaza la conversión
- **THEN** el archivo no cambia
