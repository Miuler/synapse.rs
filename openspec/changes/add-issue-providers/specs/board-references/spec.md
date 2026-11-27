# Spec Delta

## Purpose

Mostrar dentro de cualquier nota Markdown el estado de un tablero, de una parte filtrada o de una tarea concreta,
sin abrir el tablero ni depender del gestor de issues.

## ADDED Requirements

### Requirement: Referencia a una tarea

Un enlace `[[tablero.board/tarea]]` al archivo de una tarea SHALL mostrarse, en lectura y en la vista en vivo, con su
referencia, título y estado, y SHALL abrir el tablero en esa tarea.

#### Scenario: Tarea dentro de una frase

- **GIVEN** la tarea `ado-123`, con referencia `AB#123`, está «In Progress» en `Equipo.board`
- **WHEN** se lee una nota con `[[Equipo.board/ado-123]]`
- **THEN** se ve `AB#123` con su título y su estado
- **AND** al pulsarla se abre el tablero en esa tarea

#### Scenario: Estado actualizado

- **WHEN** cambia el estado de la tarea en el tablero
- **THEN** la nota que la referencia muestra el estado nuevo

### Requirement: Bloque de tablero con filtros

Un bloque `board` SHALL mostrar las tareas de un tablero como resumen, lista, tabla o Kanban, y SHALL permitir
filtrar por estado, tipo, asignado y referencia, agrupar, ordenar, limitar y elegir los campos.

#### Scenario: Resumen de avance

- **GIVEN** un tablero con 6 tareas hechas de 10
- **WHEN** una nota incluye un bloque con `view: summary`
- **THEN** se muestran el recuento por categoría de estado y el avance del 60 %

#### Scenario: Solo los errores en curso de una persona

- **WHEN** el bloque filtra `type: [Bug]`, `state: [doing]` y `assignee: Ana`
- **THEN** solo aparecen las tareas que cumplen las tres condiciones

#### Scenario: Tareas sueltas

- **WHEN** el bloque filtra `key: [AB#123, AB#124]`
- **THEN** aparecen esas dos tareas, cada una con su estado

### Requirement: Mostrar no sincroniza

Mostrar una referencia MUST usar solo los archivos del tablero, sin red ni credenciales, SHALL indicar la fecha de los
datos y SHALL ofrecer actualizarlos únicamente a petición del usuario.

#### Scenario: Sin conexión

- **GIVEN** el equipo no tiene red ni la conexión del tablero
- **WHEN** se abre la nota
- **THEN** las referencias se muestran con los datos guardados y su fecha

#### Scenario: Actualizar

- **WHEN** el usuario pulsa «Actualizar» en un bloque
- **THEN** se traen las tareas del tablero y el bloque muestra los datos nuevos

### Requirement: Referencias que no se pueden resolver

El sistema SHALL explicar en el sitio de la referencia por qué no se puede mostrar, sin alterar la nota.

#### Scenario: Tarea o tablero inexistente

- **WHEN** una nota referencia una tarea o un tablero que no existen
- **THEN** se indica cuál falta y el resto de la nota se muestra con normalidad

#### Scenario: Bloque mal escrito

- **WHEN** un bloque `board` tiene un filtro no reconocido
- **THEN** se indica el error y las claves admitidas

### Requirement: Contenido externo seguro

El texto procedente de un proveedor MUST mostrarse saneado, sin ejecutar código ni cargar recursos remotos.

#### Scenario: Título con HTML

- **GIVEN** una tarea cuyo título contiene una etiqueta con un manejador de eventos
- **WHEN** se muestra en una referencia
- **THEN** no se ejecuta nada
