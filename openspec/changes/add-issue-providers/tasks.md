# Tasks

## 1. Capa de proveedores con Azure DevOps

- [x] 1.1 Definir en `domain` el modelo común (`Issue`, estado con categoría, actividad, capacidades) y el puerto
  `IssueProvider` en `application`; verificar con pruebas unitarias de la normalización de estados y `cargo test`
- [x] 1.2 Convertir `azure_devops_client.rs` en adaptador del puerto, con registro por `provider` y órdenes
  `issues_*`; verificar que las pruebas actuales del cliente y `e2e/board-azure.test.ts` pasan sin llamadas a las
  órdenes antiguas
- [x] 1.3 Traducir el tablero y «Traer»/«Enviar» al modelo común (ids de texto, revisión opaca y estados por defecto
  de `issues_providers`); lo propio de Azure DevOps queda solo en el módulo que lee el formato anterior (hasta la 3.5)
  y en el campo de la credencial (hasta el grupo 2); verificar con `bun test ./src`, las e2e de tableros y una búsqueda
  sin otras referencias a Azure en `features/board-editor`

## 2. Conexiones y credenciales

- [x] 2.1 Guardar conexiones en `issues.connections` y su credencial en el llavero (`issues-<id>`) o como `op://`,
  descartándola al cambiar el servidor; pruebas en Rust de varias conexiones del mismo proveedor, del cambio de
  servidor y de que ningún token llega a `config.jsonc` ni a la respuesta de las órdenes
- [x] 2.2 Enviar cada credencial solo al servidor de su conexión (HTTPS o equipo local) y retirar la lista de
  servidores admitidos; pruebas con un tablero que indique otro servidor y con redirecciones
- [x] 2.3 Pantalla de conexiones en Configuración, con formulario generado por el proveedor, «Probar» y borrado;
  e2e que crea dos conexiones de Azure DevOps contra servidores falsos y confirma que cada tablero usa la suya.
  Añade `check` al puerto, tras verificar su endpoint
- [x] 2.4 Estados con su categoría: `states` en el puerto (tipos de work item y estados de cada uno) y la categoría en
  cada tarea traída; columna por categoría cuando el estado no está configurado, moviendo también las ya traídas que
  no cambiaron aquí. Pruebas contra servidor falso con estados fuera de la lista de respaldo
- [x] 2.5 Criterios de filtro declarados por el proveedor (tipo, estado, asignado, iteración, área y etiqueta) y sus
  valores, compilados a la consulta con una función pura que escapa los valores. El asignado se ofrece sin el endpoint
  de miembros (`@Me`, los ya vistos y texto libre) y la iteración actual se resuelve por fechas. Pruebas de la
  compilación y contra servidor falso
- [x] 2.6 Al crear un tablero, pedir conexión, ámbito y filtro (criterios o consulta propia), permitir cambiarlo y
  marcar lo que queda fuera; cambiar conexión o ámbito con tareas vinculadas exige confirmación
- [x] 2.7 e2e con dos tableros del mismo proyecto y filtros distintos, y documentación en `docs/reference.md` y en los
  dos CHANGELOG

## 3. Formato y carpeta del tablero

- [x] 3.1 Leer y escribir el archivo de una tarea y el del tablero (configuración y orden) como funciones puras,
  conservando lo desconocido; pruebas de ida y vuelta, de descripciones con `---` y encabezados y de la actividad
- [x] 3.2 Índice en memoria en Rust por tablero, validado con la fecha y el tamaño de cada archivo, y órdenes para leer un tablero y
  escribir una tarea con escritura atómica; pruebas en Rust y medición con 5000 tareas
- [x] 3.3 Proteger la carpeta del tablero en las escrituras del editor, el explorador y MCP, con candado fijo en la
  interfaz, y renombrar o borrar la carpeta junto con su tablero; pruebas en Rust de cada vía y e2e que confirma que no
  se puede desbloquear
- [x] 3.4 Adaptar el editor del tablero (Kanban, tabla, Gantt, Raw y detalle) al nuevo formato, mostrando referencia,
  tipo y asignado según `fields`; e2e de tableros y documentación del formato.
  Volver a leer el tablero cuando el observador avise de cambios en su carpeta.
  El Gantt lee y escribe el formato, sin datos nuevos (`fields` solo gobierna tarjetas y tabla); la referencia se ve
  como texto, sin enlace para abrirla en el proveedor
- [x] 3.5 Convertir con confirmación los tableros con tabla y la clave `azure_devops`; pruebas con tableros actuales,
  documentación en `docs/reference.md` y en los dos CHANGELOG.
  Los tableros nuevos se crean ya en el formato de carpeta, sin tareas de ejemplo. La conexión del tablero se deja como
  estaba (vacía, la resuelve el servidor o la credencial global): la conversión no propone ni crea conexiones (paso 2
  del plan de migración); se crean desde Configuración
- [x] 3.6 Traer descripción y actividad de Azure DevOps (comentarios e histórico) solo de lo cambiado desde la última
  sincronización, tras verificar sus endpoints; pruebas contra servidor falso y e2e de «Traer» y «Enviar».
  Añade `activity` al puerto.
  Solo en los tableros de carpeta. Los endpoints se comprobaron en la documentación, no contra una cuenta real. Se
  pide la actividad completa de cada tarea cuya revisión o número de comentarios cambió: un comentario editado en el
  proveedor llega cuando cambie la tarea. Si la descripción cambió en los dos lados se conserva la local y se avisa;
  desde la 3.7 se puede volver a la del proveedor
- [x] 3.7 Editar en el detalle de la tarea todos sus campos y comentarios, desde Kanban, tabla y Gantt, con aviso y
  marca al cambiar el histórico; e2e de la edición, de cancelar el aviso y de que traer no la pisa.
  Los tableros con tabla solo editan título, estado, planificación y comentarios (de una línea); las columnas propias de
  un tablero convertido no se ven ni se editan en el detalle. El aviso de conflicto en la descripción sale una vez, y
  se resuelve volviendo a la del proveedor o dejando la local
- [x] 3.8 Enviar los cambios de descripción, tipo, asignado y comentarios, tras verificar sus endpoints, indicando lo
  no admitido y avisando del formato que se perdería; pruebas contra servidor falso de conflictos y rechazos.
  Probado solo contra el servidor falso; nada se ha enviado a una cuenta real. Quitar la asignación (`""`) y cambiar el
  tipo junto con un estado salen de los ejemplos de la referencia; editar comentarios, de su página de actualización,
  que no dice quién puede: solo se reescriben los enviados desde el tablero. El tipo solo se cambia en la nube, con un
  estado que el tipo nuevo admita. El asignado se envía por nombre, que Azure DevOps puede rechazar si hay repetidos.
  La descripción se escribe en el campo del que se leyó, como HTML o como Markdown según lo que declare el work item


## 4. Referencias desde Markdown

- [x] 4.1 Resolver un tablero y filtrar sus tareas (estado, tipo, asignado, referencia, orden, grupo, límite y
  campos) como funciones puras; pruebas unitarias de cada filtro, del resumen y de los errores.
  En `src/entities/board`, sin conectar todavía a ninguna vista. La categoría sale del nombre de la columna o del estado
  y, si no dicen nada, de su sitio en el tablero; el avance no cuenta lo cancelado. Para ordenar por `updated`, «Traer»
  guarda ahora la fecha de cambio de cada tarea que lee (las anteriores quedan al final hasta que cambien). Las
  referencias solo resuelven tableros de carpeta
- [x] 4.2 Pintar en lectura el bloque `board` y el enlace a una tarea, saneados y actualizados al cambiar el tablero;
  e2e de las cuatro vistas, de una referencia inexistente y de un título con HTML.
  Pulsar un enlace abre el archivo de la tarea, no el tablero en ella, y todavía no hay «Actualizar» (4.4). Las fechas
  salen en UTC. Solo tableros de carpeta: de uno con tabla se pide convertirlo. El tablero se busca con el resolvedor
  de enlaces de la bóveda (sustituye a la función propia de la 4.1; un nombre ambiguo no tiene prueba). La sintaxis se
  documenta en la 4.4
- [x] 4.3 Pintarlos en la vista en vivo; e2e de edición alrededor del widget.
  Con el cursor dentro de un bloque o de un enlace se ve su texto, sin vista previa debajo. Cada referencia se pinta y
  vigila su tablero por separado; las que piden a la vez el mismo comparten la lectura
- [x] 4.4 «Actualizar» desde un bloque y abrir el tablero en una tarea; e2e que confirma que abrir la nota no hace
  peticiones; documentar la sintaxis con ejemplos.
  «Actualizar» es el «Traer» del tablero (lo comparten en `model/pull.ts`) y solo aparece si el tablero tiene origen; no
  ofrece elegir conexión si hay varias que sirven: lo indica y se elige en el tablero. Si mientras actualiza se reordena o se
  ensancha una columna en ese tablero abierto, ese cambio choca con la escritura: se avisa, se vuelve a leer y hay que
  repetirlo

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

- [x] 7.1 Servir las herramientas `config_` solo a sesiones con su token, sin CORS, y entregarlo en `session/new` y
  `session/load` (HTTP y puente stdio); verificar qué agentes admiten añadirlo al reanudar; pruebas en Rust de
  clientes sin token, con otro token y desde un origen web
  Las cuatro herramientas se listan y, de momento, responden que no hay ninguna configuración abierta. El token va en la
  cabecera `X-Synapse-Session` o en la variable `SYNAPSE_MCP_SESSION` del puente; vive en memoria, se retira al parar la
  sesión y al retomar la conversación, y el registro de depuración de ACP lo oculta. Un agente que al retomar conservara
  la sesión viva sin aceptar el token nuevo se quedaría sin estas herramientas. Reanudación: solo se revisó claude-acp
  0.113.0 (el único instalado, leyendo su binario): `session/load` aplica los `mcpServers` nuevos y recrea la sesión viva
  si cambiaron. Límite: ese adaptador pasa los `mcpServers` al CLI de Claude como argumento (`--mcp-config`), así que el
  token se ve en la línea de órdenes de ese proceso; su SDK tiene la opción `mcpServersDelivery: "initialize"`, que lo
  evitaría (sin probar ni aplicar)
- [x] 7.2 Implementar contexto, descripción, prueba de consulta limitada y cambio del borrador; pruebas contra
  servidor falso de errores, límites y de que ninguna respuesta incluye credenciales ni guarda nada
  La ventana avisa del borrador abierto con el comando `mcp_config_draft` (`null` al cerrarlo); un cambio le llega como
  evento `mcp://config-change` y lo contesta con `mcp_config_changed` (espera máxima de 5 s). Topes: 20 tareas por
  prueba y 200 valores por criterio. De los fallos de credencial no se entrega ningún detalle
- [x] 7.3 Configuración del tablero no modal junto al chat, con los cambios del agente marcados y el aviso de qué
  recibe; e2e con el agente falso que prueba, corrige y deja el borrador sin guardar; documentar en
  `docs/reference.md` y en los dos CHANGELOG
  Es un panel pegado a la izquierda del tablero; «Traer», «Enviar» y la configuración quedan desactivados mientras está
  abierto, y al guardar se conserva lo que el tablero tenga en ese momento. Incluye los estados por columna. El borrador
  sigue abierto si se cambia de pestaña; con varios, manda el último abierto, y el cambio solo se envía a su ventana. e2e
  en `board-assistant.test.ts` con `fixtures/fake-config-agent.ts` (puente stdio). Sin comprobar con un agente real ni
  por la conexión HTTP

## 8. Integración

- [ ] 8.1 Un tablero por proveedor en la misma bóveda y una nota que referencie los tres; `cargo test`, `clippy`,
  `cargo deny`, `bun run check`, `bun test ./src` y las e2e de tableros
  Avance 2026-11-27, solo con Azure DevOps: pasan `cargo test` (338 + 69), `clippy -D warnings`, `cargo deny` (avisos,
  licencias y fuentes; quedan avisos de versiones duplicadas), `bun run check`, `bun test ./src` (367) y las e2e de
  tableros (71). Añadida la e2e de abrir tareas y arrastrar tarjetas con la configuración abierta. Sigue abierta hasta
  que existan GitHub y Jira (grupos 5 y 6)
- [ ] 8.2 Comprobación manual de solo «Traer» con una cuenta real de cada proveedor disponible, y después una
  escritura sobre una tarea de prueba

## Workflow follow-up

- Archivar el cambio cuando se haya revisado.
