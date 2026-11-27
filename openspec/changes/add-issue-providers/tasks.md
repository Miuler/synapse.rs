# Tasks

## 1. Capa de proveedores con Azure DevOps

- [ ] 1.1 Definir en `domain` el modelo común (`Issue`, estado con categoría, actividad, capacidades) y el puerto
  `IssueProvider` en `application`; verificar con pruebas unitarias de la normalización de estados y `cargo test`
- [ ] 1.2 Convertir `azure_devops_client.rs` en adaptador del puerto, con registro por `provider` y órdenes
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
  e2e que crea dos conexiones de Azure DevOps contra servidores falsos y confirma que cada tablero usa la suya
- [ ] 2.4 Elegir conexión, ámbito y consulta desde el tablero, bloquearlos con tareas vinculadas y migrar la clave
  `azure_devops` y el token global; pruebas de ida y vuelta de la migración y documentación en `docs/reference.md` y
  en los dos CHANGELOG

## 3. Formato del tablero

- [ ] 3.1 Leer y escribir la tabla por los nombres de su cabecera, conservando columnas desconocidas, con referencia,
  tipo y asignado; pruebas de ida y vuelta con el formato actual y con columnas propias
- [ ] 3.2 Guardar `source`, `fields`, la fecha de los datos y `## Actividad` con clase, autor e identificador remoto,
  convirtiendo `## Comentarios`; pruebas con texto de varias líneas y de que no se reenvía lo traído
- [ ] 3.3 Traer actividad de Azure DevOps (comentarios e histórico) con límite por tarea, tras verificar sus
  endpoints en la documentación; pruebas contra servidor falso y e2e de «Traer» y «Enviar» con actividad
- [ ] 3.4 Mostrar referencia, tipo y asignado en el Kanban y la tabla según `fields`; e2e y documentación del formato

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

## 7. Integración

- [ ] 7.1 Un tablero por proveedor en la misma bóveda y una nota que referencie los tres; `cargo test`, `clippy`,
  `cargo deny`, `bun run check`, `bun test ./src` y las e2e de tableros
- [ ] 7.2 Comprobación manual de solo «Traer» con una cuenta real de cada proveedor disponible, y después una
  escritura sobre una tarea de prueba

## Workflow follow-up

- Archivar el cambio cuando se haya revisado.
