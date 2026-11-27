# Spec Delta

## Purpose

Conexiones configurables a gestores de issues (Azure DevOps, GitHub y Jira), la credencial de cada una y el
comportamiento común que todos los proveedores ofrecen al resto de Synapse.

## ADDED Requirements

### Requirement: Crear una conexión eligiendo el proveedor

El sistema SHALL permitir crear conexiones con nombre pidiendo primero el proveedor y después solo los datos que ese
proveedor necesita, y SHALL permitir probarlas antes de usarlas.

#### Scenario: Formulario según el proveedor

- **GIVEN** la pantalla de conexiones
- **WHEN** el usuario añade una conexión y elige Jira
- **THEN** se le piden el servidor, el usuario y la credencial, y no los datos propios de otros proveedores

#### Scenario: Probar una conexión

- **WHEN** el usuario prueba una conexión con una credencial válida
- **THEN** el sistema indica con qué cuenta ha entrado
- **AND** si la credencial no vale, explica el motivo sin mostrarla

### Requirement: Varias conexiones, también del mismo proveedor

El sistema SHALL admitir cualquier número de conexiones independientes, incluidas varias del mismo proveedor con
distinto servidor, cuenta o credencial.

#### Scenario: Dos organizaciones de Azure DevOps

- **GIVEN** las conexiones «Trabajo» y «Cliente», ambas de Azure DevOps
- **WHEN** un tablero vinculado a «Cliente» trae sus tareas
- **THEN** solo se usan el servidor y la credencial de «Cliente»

### Requirement: Credencial protegida

La credencial de una conexión SHALL guardarse en el llavero del sistema o como referencia `op://` de 1Password, y
MUST NOT escribirse en la bóveda, en la configuración en texto plano, ni mostrarse de nuevo.

#### Scenario: Token pegado

- **WHEN** el usuario pega un token en una conexión y la guarda
- **THEN** el token queda en el llavero y el campo no vuelve a mostrar su valor

#### Scenario: Referencia de 1Password

- **WHEN** el usuario escribe una referencia `op://`
- **THEN** se guarda la referencia y la credencial se pide a 1Password solo al comunicarse con el proveedor

### Requirement: La credencial solo viaja a su servidor

El sistema MUST enviar una credencial únicamente al servidor de su conexión, por HTTPS salvo que el servidor sea el
propio equipo, y MUST descartarla si se cambia ese servidor.

#### Scenario: Tablero que apunta a otro servidor

- **GIVEN** un tablero cuyo archivo indica un servidor distinto del de su conexión
- **WHEN** el usuario intenta sincronizarlo
- **THEN** no se envía nada y se explica la discrepancia

#### Scenario: Cambiar el servidor

- **GIVEN** una conexión con credencial guardada
- **WHEN** se cambia su servidor
- **THEN** la conexión queda sin credencial hasta que se vuelva a introducir

### Requirement: Modelo común de tareas

El sistema SHALL ofrecer de cada tarea, venga del proveedor que venga, su referencia, enlace, tipo, título, estado
con su categoría (`todo`, `doing`, `done` o `cancelled`), asignado, etiquetas y fecha de cambio.

#### Scenario: Estados distintos, misma categoría

- **WHEN** se traen una tarea «Active» de Azure DevOps y una cerrada como completada en GitHub
- **THEN** la primera queda en la categoría `doing` y la segunda en `done`, conservando su estado original

### Requirement: Diferencias declaradas

El sistema SHALL ofrecer solo las acciones que el proveedor de la conexión admite.

#### Scenario: Estado que exige una transición

- **GIVEN** una tarea de Jira cuyo flujo no permite pasar de su estado al de la columna elegida
- **WHEN** el usuario envía los cambios
- **THEN** ese cambio no se aplica y se indica a qué estados sí puede pasar
