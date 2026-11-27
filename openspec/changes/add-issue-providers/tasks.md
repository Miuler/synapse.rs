# Tasks

## 1. Capa de proveedores con Azure DevOps

- [x] 1.1 Definir en `domain` el modelo común (`Issue`, estado con categoría, actividad, capacidades) y el puerto
  `IssueProvider` en `application`; verificar con pruebas unitarias de la normalización de estados y `cargo test`
- [x] 1.2 Convertir `azure_devops_client.rs` en adaptador del puerto, con registro por `provider` y órdenes
  `issues_*`; verificar que las pruebas actuales del cliente y `e2e/board-azure.test.ts` pasan sin llamadas a las
  órdenes antiguas
- [ ] 1.3 Traducir el tablero y «Traer»/«Enviar» al modelo común, sin tipos de Azure DevOps en
  `features/board-editor`; verificar con `bun test ./src` y una búsqueda sin referencias a `azure` fuera de la API
  de conexiones

## 2. Conexiones y credenciales

- [ ] 2.1 Guardar conexiones en `issues.connections` y su credencial en el llavero (`issues/<id>`) o como `op://`,
  descartándola al cambiar el servidor; pruebas en Rust de varias conexiones del mismo proveedor, del cambio de
  servidor y de que ningún token llega a `config.jsonc` ni a la respuesta de las órdenes
- [ ] 2.2 Enviar cada credencial solo al servidor de su conexión (HTTPS o equipo local) y retirar la lista de
  servidores admitidos; pruebas con un tablero que indique otro servidor y con redirecciones
- [ ] 2.3 Pantalla de conexiones en Configuración, con formulario generado por el proveedor, «Probar» y borrado;
  e2e que crea dos conexiones de Azure DevOps contra servidores falsos y confirma que cada tablero usa la suya.
  Añade `check` al puerto, tras verificar su endpoint
- [ ] 2.4 Al crear un tablero, pedir conexión, ámbito y filtro (criterios que declara el proveedor o su consulta),
  permitir cambiarlo y marcar lo que queda fuera, bloqueando conexión y ámbito con tareas vinculadas; e2e con dos
  tableros del mismo proyecto y filtros distintos.
  Añade `states` al puerto y la categoría que declare el proveedor, tras verificar su endpoint

## 3. Formato y carpeta del tablero

- [ ] 3.1 Leer y escribir el archivo de una tarea y el del tablero (configuración y orden) como funciones puras,
  conservando lo desconocido; pruebas de ida y vuelta, de descripciones con `---` y encabezados y de la actividad
- [ ] 3.2 Índice en memoria en Rust por tablero, actualizado por el observador, y órdenes para leer un tablero y
  escribir una tarea con escritura atómica; pruebas en Rust y medición con 5000 tareas
- [ ] 3.3 Proteger la carpeta del tablero en las escrituras del editor, el explorador y MCP, con candado fijo en la
  interfaz, y renombrar o borrar la carpeta junto con su tablero; pruebas en Rust de cada vía y e2e que confirma que no
  se puede desbloquear
- [ ] 3.4 Adaptar el editor del tablero (Kanban, tabla, Gantt, Raw y detalle) al nuevo formato, mostrando referencia,
  tipo y asignado según `fields`; e2e de tableros y documentación del formato
- [ ] 3.5 Convertir con confirmación los tableros con tabla y la clave `azure_devops`; pruebas con tableros actuales,
  documentación en `docs/reference.md` y en los dos CHANGELOG
- [ ] 3.6 Traer descripción y actividad de Azure DevOps (comentarios e histórico) solo de lo cambiado desde la última
  sincronización, tras verificar sus endpoints; pruebas contra servidor falso y e2e de «Traer» y «Enviar».
  Añade `activity` al puerto
- [ ] 3.7 Editar en el detalle de la tarea todos sus campos y comentarios, desde Kanban, tabla y Gantt, con aviso y
  marca al cambiar el histórico; e2e de la edición, de cancelar el aviso y de que traer no la pisa
- [ ] 3.8 Enviar los cambios de descripción, tipo, asignado y comentarios, tras verificar sus endpoints, indicando lo
  no admitido y avisando del formato que se perdería; pruebas contra servidor falso de conflictos y rechazos

## 4. Referencias desde Markdown

- [ ] 4.1 Resolver un tablero y filtrar sus tareas (estado, tipo, asignado, referencia, orden, grupo, límite y
  campos) como funciones puras; pruebas unitarias de cada filtro, del resumen y de los errores
- [ ] 4.2 Pintar en lectura el bloque `board` y el enlace a una tarea, saneados y actualizados al cambiar el tablero;
  e2e de las cuatro vistas, de una referencia inexistente y de un título con HTML
- [ ] 4.3 Pintarlos en la vista en vivo; e2e de edición alrededor del widget
- [ ] 4.4 «Actualizar» desde un bloque y abrir el tablero en una tarea; e2e que confirma que abrir la nota no hace
  peticiones; documentar la sintaxis con ejemplos

## 5. GitHub

- [ ] 5.1 Verificar en la documentación autenticación, búsqueda, escritura, comentarios, cronología y límites, y
  escribir el adaptador (estado abierto/cerrado y columnas por etiqueta, sin pull requests); pruebas contra servidor
  falso
- [ ] 5.2 Ejecutar con GitHub las e2e de conexiones, sincronización y referencias, y documentar su configuración

## 6. Jira

- [ ] 6.1 Verificar en la documentación autenticación (Cloud y Data Center), búsqueda JQL, transiciones, comentarios,
  registro de cambios y formato ADF, y escribir el adaptador; pruebas contra servidor falso, incluida una transición
  no permitida
- [ ] 6.2 Ejecutar con Jira las e2e de conexiones, sincronización y referencias, y documentar su configuración

## 7. Asistente de configuración

- [ ] 7.1 Servir las herramientas `config_` solo a sesiones con su token, sin CORS, y entregarlo en `session/new` y
  `session/load` (HTTP y puente stdio); verificar qué agentes admiten añadirlo al reanudar; pruebas en Rust de
  clientes sin token, con otro token y desde un origen web
- [ ] 7.2 Implementar contexto, descripción, prueba de consulta limitada y cambio del borrador; pruebas contra
  servidor falso de errores, límites y de que ninguna respuesta incluye credenciales ni guarda nada
- [ ] 7.3 Configuración del tablero no modal junto al chat, con los cambios del agente marcados y el aviso de qué
  recibe; e2e con el agente falso que prueba, corrige y deja el borrador sin guardar; documentar en
  `docs/reference.md` y en los dos CHANGELOG

## 8. Integración

- [ ] 8.1 Un tablero por proveedor en la misma bóveda y una nota que referencie los tres; `cargo test`, `clippy`,
  `cargo deny`, `bun run check`, `bun test ./src` y las e2e de tableros
- [ ] 8.2 Comprobación manual de solo «Traer» con una cuenta real de cada proveedor disponible, y después una
  escritura sobre una tarea de prueba

## Workflow follow-up

- Archivar el cambio cuando se haya revisado.
