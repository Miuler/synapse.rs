# Spec Delta

## Purpose

El tablero de tareas vinculado a una conexión de un gestor de issues: qué guarda de cada tarea para poder
consultarlo sin red y cómo trae y envía los cambios.

## ADDED Requirements

### Requirement: Un tablero, una conexión

Cada tablero sincronizado SHALL estar vinculado a una sola conexión, con su ámbito y su consulta, y SHALL conservar
ese vínculo en su archivo. Con tareas vinculadas, la conexión y el ámbito MUST NOT cambiar sin desvincularlas antes
con confirmación.

#### Scenario: Elegir el origen

- **WHEN** el usuario configura un tablero sin origen
- **THEN** elige una conexión existente o crea una, y define el proyecto o repositorio y la consulta

#### Scenario: Cambiar el origen de un tablero con tareas

- **GIVEN** un tablero con tareas vinculadas
- **WHEN** el usuario intenta elegir otra conexión
- **THEN** el sistema avisa de que se perderán los vínculos y no cambia nada sin confirmación

### Requirement: Referencias completas en el archivo

El archivo del tablero SHALL guardar, de cada tarea vinculada, su referencia, enlace, tipo, estado, asignado y la
fecha de los datos, sin ninguna credencial.

#### Scenario: Abrir en otro equipo

- **GIVEN** un tablero sincronizado y un equipo sin su conexión
- **WHEN** se abre el tablero
- **THEN** se ven las tareas con sus referencias y estados
- **AND** se ofrece elegir una conexión del mismo proveedor y servidor, o crearla

### Requirement: Actividad con su clase

El tablero SHALL guardar la actividad de cada tarea indicando si es un comentario o un cambio del histórico, con su
autor, fecha e identificador remoto. El histórico MUST ser de solo lectura.

#### Scenario: Traer actividad

- **WHEN** se trae una tarea con dos comentarios y un cambio de estado
- **THEN** el tablero guarda tres entradas, dos de clase `comment` y una de clase `history`

#### Scenario: Enviar después de traer

- **GIVEN** un tablero con comentarios traídos y uno escrito en Synapse
- **WHEN** el usuario envía los cambios
- **THEN** solo se envía el escrito en Synapse, y no se vuelve a enviar la vez siguiente

### Requirement: Traer sin pisar cambios locales

Traer SHALL añadir las tareas nuevas y actualizar las que solo cambiaron en el proveedor; lo que cambió en los dos
lados MUST conservarse como está en el tablero y avisarse.

#### Scenario: Cambio en los dos lados

- **GIVEN** una tarea cuyo estado cambió en el tablero y en el proveedor
- **WHEN** se trae
- **THEN** se conserva el estado del tablero y se informa del conflicto

### Requirement: Enviar solo lo revisado

Enviar SHALL mostrar antes los cambios, MUST NOT sobrescribir un conflicto sin que se marque y SHALL crear en el
proveedor solo las tareas que se marquen.

#### Scenario: Tarea local sin vínculo

- **GIVEN** un tablero con una tarea creada en Synapse
- **WHEN** el usuario abre la revisión de envío
- **THEN** la tarea aparece sin marcar y no se crea si se deja así

### Requirement: Compatibilidad con tableros anteriores

Un tablero sin origen, con columnas distintas o con el formato anterior SHALL abrirse sin perder contenido.

#### Scenario: Columnas propias

- **GIVEN** un tablero con una columna que Synapse no conoce
- **WHEN** se edita una tarea y se guarda
- **THEN** esa columna y sus valores siguen en el archivo

#### Scenario: Tablero con la configuración anterior de Azure DevOps

- **WHEN** se abre un tablero con la clave `azure_devops`
- **THEN** se propone la conexión que corresponde a su servidor y, al guardar, conserva los vínculos
