# Registro de cambios

_[English](CHANGELOG.md)_

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Sin publicar] (Unreleased)

### Añadido

- Editor Markdown: escribir `[[#` en el autocompletado de wikilinks ahora busca entre todas las tareas de los tableros `.board/` por número/ID o título e inserta `[[Tablero.board/id]]`. Los enlaces inline a tareas se muestran como insignias segmentadas al estilo Jira (`[clave] [título] [estado]`), con variante compacta (`[[Tablero.board/id|#]]` o `|compact`) sin título.
- Tableros de tareas: las tareas locales ahora usan IDs UUIDv7 en Crockford Base32 de 26 caracteres, los issues traídos de Azure DevOps dejan de llevar el prefijo `ado-` (manteniendo compatibilidad con los `ado-*` existentes) y las referencias (`parent`, `AB#123`, `#123` en el detalle y `[[Tablero.board/123]]` en notas) navegan localmente a la tarea sincronizada.

- Tableros de tareas (Azure DevOps): «Traer» ahora obtiene y guarda en los archivos de tarea los atributos nativos del work item —enlace clicable al proveedor, etiquetas, prioridad, severidad, iteración, área, padre, estimación, fechas de inicio y vencimiento, horas de planificación (estimación original, trabajo restante y completado), columna y carril del tablero, motivo, campos de auditoría (creado/modificado por y fecha) y criterios de aceptación— y usa `uniqueName` como respaldo cuando una identidad no tiene `displayName`. Las tarjetas del Kanban, la vista de Tabla (con ordenación y columnas opcionales) y el diálogo de detalle ya los muestran.
- Tableros de tareas: la configuración del origen es ahora un panel lateral, así que se puede usar el chat del agente a la vez. Un agente abierto por Synapse puede ver la conexión (nunca su credencial), probar consultas en solo lectura y cambiar en pantalla la consulta, el filtro o los estados; sus cambios quedan marcados y solo tú los guardas. Ahí se editan también los estados de cada columna. Esas herramientas `config_` son privadas: otros clientes MCP y las páginas web no las ven ni pueden llamarlas.
- Chat del agente: clic derecho en una respuesta para «Copiar bloque» (el párrafo, lista, tabla o código bajo el puntero, como Markdown y con formato) o «Copiar respuesta». Con texto marcado, o sobre un enlace o una imagen, sale el menú del sistema como antes.
- Chat del agente: clic derecho en una herramienta del registro para copiarla (nombre, entrada y salida) o solo el texto marcado.
- Hacer clic en una notificación del agente (fin de turno, permiso o pregunta) trae Synapse al frente y muestra esa conversación (Linux; en Hyprland la ventana se enfoca a través del compositor).
- Los `index.md` muestran el fondo de la pestaña vacía como una marca de agua muy tenue y desenfocada; los temas pueden cambiarla con `--index-wallpaper-blur` y `--index-wallpaper-opacity`.
- Log de Git: «Merge de “rama” en “actual”» en el menú de un commit que no está en la rama actual. Si hay conflictos, un diálogo al estilo de JetBrains lista los archivos (diff, aceptar el mío, aceptar el suyo o fusionar; ↑/↓ y Enter), muestra qué cambió en un archivo (también en uno que un lado borró, para ver qué se perdería) y abre un editor de tres paneles (tuyo, resultado, suyo) donde cada cambio se aplica o se descarta, los que no chocan se aplican de una vez, el resultado se puede editar a mano y `F7` / `Shift+F7` pasan de un cambio a otro. El merge se confirma (y se puede deshacer desde el log) o se cancela; uno sin terminar, también si viene de la terminal, se retoma desde el log.
- Chat del agente: `@` completa también las carpetas de la bóveda (`@docs/guias/`; con `@f:` se ven solo archivos y carpetas): la carpeta va adjunta y se le indica al agente que liste su contenido.
- Pestaña vacía: un botón «Personalizar» abajo a la derecha muestra u oculta el buscador, los archivos recientes, el botón de crear y las sugerencias, fija cuántos recientes se listan y elige el fondo (imagen, el original o ninguno). Un pulpo sugiere un atajo, otro en cada clic, en lugar de los mensajes «Ningún archivo abierto».
- Pestaña vacía: bajo el buscador lista los últimos archivos abiertos (3 por defecto, hasta 15, configurable en Configuración), en una sección que se puede plegar (se recuerda así); «Crear nuevo archivo» abre ahora una lista de tipos de archivo, cada uno con su icono.
- `Ctrl+←` / `Ctrl+→` cambian a la pestaña anterior o siguiente cuando no se está escribiendo; en editores, campos de texto y Excalidraw las teclas hacen lo de siempre. Configurable en Configuración.
- Los tableros guardan un archivo por tarea (`format: 2`, en `Nombre.board/`): las vistas leen y escriben esos archivos, muestran referencia, tipo y asignado, abren la descripción y el histórico en el diálogo de la tarea y recogen los cambios hechos por fuera. Los nuevos se crean así, sin tareas de ejemplo. Los tableros con tabla siguen funcionando y ofrecen convertirse, con confirmación: se conservan comentarios, vínculos, columnas propias y demás texto, y el original se guarda como `Nombre.board.md.bak`.
- «Traer» en un tablero de carpeta guarda además la descripción de cada tarea (como Markdown saneado) y sus comentarios e histórico con autor y fecha, pidiendo solo lo que cambió; los comentarios ya enviados no se repiten y los traídos nunca se vuelven a enviar.
- «Enviar» manda además el tipo, el asignado, la descripción y los comentarios editados de una tarea, tras revisarlos: los conflictos y lo que perdería formato o reescribiría un comentario salen sin marcar, y lo que el proveedor no admite se indica y no se envía.
- Un tablero en modo lectura o bloqueado ya solo se puede mirar (cambiar de vista, ordenar y abrir una tarea siguen funcionando). Los tableros se abren en edición salvo que elijas lectura, que se recuerda por tablero.
- La vista de tabla de un tablero con muchas tareas aparece mucho antes (con 1000, unos 0,4 s en lugar de 7 s en la versión de desarrollo): la lista «Depende» se crea solo al usarla y las filas entran por tandas.
- Un bloque `board` tiene un botón «Actualizar» que trae el tablero solo al pulsarlo, y pulsar una referencia a una tarea abre el tablero en ella.
- La vista en vivo también muestra los bloques `board` y los enlaces a tareas; al entrar en uno vuelve su texto para editarlo.
- En modo lectura, un bloque `board` muestra un tablero como resumen, lista, tabla o Kanban, con filtros, y `[[Nombre.board/tarea]]` muestra la referencia, el título y el estado de una tarea; los dos siguen los cambios del tablero.
- El detalle de la tarea lo edita todo: título, estado, planificación y comentarios en cualquier tablero, y además tipo, asignado, descripción e histórico en los de carpeta (el histórico tras un aviso, y queda marcado como editado). Traer conserva lo que cambiaste y avisa de lo que cambió en los dos lados.
- La carpeta de tareas de un tablero (`Nombre.board/`) se puede ver pero no cambiar desde Synapse (editor, explorador y herramientas MCP): sus archivos llevan un candado que no se quita. Renombrar, mover o copiar el tablero se lleva la carpeta, y al eliminarlo se pregunta por ella.
- Tableros de tareas (`.board.md`): editor con vistas Tabla, Kanban y Gantt, con comentarios por tarea; se crean desde el menú del explorador («Nuevo tablero»), la paleta de comandos o el índice de una carpeta. Las tarjetas del Kanban se arrastran entre columnas para cambiar de estado, y arriba o abajo para decidir cuál va primero (ese orden es el de las filas de la tabla, así que Tabla y Gantt lo siguen), o se mueven con `Shift` + flechas sobre la tarjeta enfocada; `Escape` cancela el arrastre. Una columna se ensancha arrastrando su borde derecho (o con `←` / `→` sobre él; doble clic o `Enter` la restablecen) y el ancho se guarda en el tablero; la vista de tabla se ordena por cualquier columna sin cambiar el orden del tablero. Con doble clic en una tarea (o `Enter` sobre la tarjeta enfocada) se abre en un diálogo al estilo de Jira: los comentarios a la izquierda y el estado y los datos a la derecha. La vista «Raw» muestra el archivo tal como se guarda (solo lectura).
- Log de Git: el menú de un commit une los commits desde él hasta la punta de la rama actual (sin tocar los archivos ni lo sin confirmar; no a través de fusiones, y avisa si ya están en un remoto), lo revierte (se cancela si hay conflictos) o hace reset de la rama hasta él, soft o hard (los cambios sin confirmar se guardan en el stash y nunca se pisan archivos sin versionar). Lo último de esto se puede deshacer desde el mismo diálogo.
- El índice de una carpeta abre lo que pulses en su misma pestaña, en lugar del índice; «Atrás» vuelve a él.
- Los tableros de tareas se sincronizan con Azure DevOps en dos acciones separadas: «Traer» trae los work items de un proyecto (título y estado; por defecto los abiertos asignados a ti, cambiados o creados por ti, o tu propia consulta WIQL) y «Enviar» muestra lo que cambió en el tablero antes de mandarlo (título, estado, comentarios nuevos) y solo crea como work items las tareas que marques. Lo que cambió en los dos lados se avisa y no se sobrescribe salvo que lo marques. El campo del token único (mientras no haya conexiones), en Configuración → Conexiones y en la configuración del tablero, admite un token, que se guarda en el llavero del sistema, o una ruta `op://` de 1Password, que se guarda como `azure_devops.token` en `config.jsonc` y se resuelve con el CLI `op` al sincronizar; sin ninguno se usa `AZURE_DEVOPS_PAT` / `ADO_PAT`. El token no pasa por el entorno que heredan los agentes, nunca se guarda en el tablero y solo se envía a Azure DevOps. En Configuración → Conexiones se pueden crear varias conexiones (con «Probar», que dice con qué cuenta entra el token), cada una con su servidor y su token, y un token solo se envía al servidor de su conexión (cambiar el servidor lo descarta). Cada tablero elige sus tareas (las tuyas, por tipo, estado, asignado, iteración, área y etiqueta, o con tu consulta); se pregunta al crearlo, y las vinculadas que dejan de cumplirlo se quedan, marcadas. Cada estado remoto va a la columna de la categoría que el proveedor le declara (así los estados propios también quedan ubicados), salvo que el tablero lo configure de otra forma. No se sincronizan fechas, dependencias, progreso ni descripciones.
- Puntero láser en diagramas e imágenes: `K` lo activa o desactiva y `Escape` lo apaga (con el puntero sobre el visor); mantener el clic derecho arrastra la vista, también en modo láser.
- El modo láser muestra un cursor con forma de puntero láser en lugar de la cruz.
- Puntero láser: punto y rayo más finos, con el tamaño del punto configurable en Configuración; el punto se dibuja separado del icono del cursor. El botón del medio también arrastra la vista en los visores de imágenes y diagramas.

- Búsqueda y `full_text_search` del MCP: `a|b` (u `or` en minúsculas) busca varias alternativas en una sola consulta, p. ej. `TIEL|QTI1|QTI3`.
- MCP: `full_text_search` con `lines: true` devuelve las líneas que coinciden en cada resultado con su ubicación, como `rg -n`: el número de línea en Markdown, código y diagramas en texto, la página en PDF, la sección y el párrafo en documentos de Word, la hoja, fila y columna en planillas, la página de los diagramas de draw.io y el elemento en modelos ArchiMate y BPMN; con `exact: true` devuelve solo los documentos que contienen los textos tal cual (endpoints, ids o códigos con puntuación, como `outbounds-ach`). Los resultados cuyo texto se cortó al indexar (documentos muy grandes) llevan la marca `truncated`.
- Búsqueda: `hs:` también filtra por los títulos de los documentos de Word.

- MCP: `search_in_note` busca varios textos a la vez separados por `|` (como `rg "a|b"`), indica cuáles contiene cada línea, cuenta las líneas y apariciones de cada uno (`counts_only` devuelve solo los recuentos, como `grep -c`) y acepta varias notas a la vez con `relative_paths`. Una fila de tabla Markdown como `| campo |` se sigue buscando literal.

- MCP: cuando una ruta no existe, el error indica la bóveda activa, las otras bóvedas donde esa ruta sí existe y los archivos con el mismo nombre; y `search_notes` avisa cuando ninguna nota Markdown coincide pero sí archivos de otros tipos.

- Chat del agente: el logo del agente en uso aparece como marca de agua tenue en la esquina inferior derecha; su opacidad se ajusta en Configuración (0 % la oculta).
- Chat del agente: el menú «⋯» de una conversación muestra el id y el nombre que la sesión tiene en el propio agente; un clic en cualquiera de los dos lo copia; con los agentes que lo admiten (Claude) también se puede renombrar ahí. La conversación se titula ahora con el nombre que el agente le da a su sesión (Claude, Antigravity) en lugar de tu primera pregunta.
- Chat del agente: las preguntas aparte (`/btw`) se abren en una ventana flotante que se arrastra y cambia de tamaño, con el mismo chat dentro (razonamiento, herramientas, adjuntos, contexto usado) para seguir preguntando en la misma sesión. Muestra el historial del que partió y, plegado, lo que se envió al agente para empezarla, se minimiza (`Esc`) en una tarjeta encima del botón Enviar, varias se ven como pestañas y se guardan con su conversación hasta que las borres. La ayuda del comando avisa de que `/btw` lo resuelve Synapse, no el agente. Una pregunta aparte que no estás viendo (minimizada o en otra pestaña) cierra su sesión cuando termina de responder, con los agentes que saben retomarla (Claude, agy), y la retoma cuando vuelves a preguntar. Se abre con el modelo y el esfuerzo de su conversación; lo que cambies dentro es solo suyo, se conserva y no cambia el predeterminado recordado para el agente.
- Visor de imágenes: puntero láser en la barra, como el de los diagramas.
- `Ctrl+F` en documentos de Word y hojas de cálculo. En una hoja de cálculo busca en todas las hojas, sin distinguir mayúsculas ni tildes.
- Documentos de Word en el diff y el historial de Git: «Cambios» muestra los párrafos, títulos, celdas de tabla e imágenes que cambiaron, y «2 Paneles» dibuja las dos versiones lado a lado.
- Chat del agente: `Ctrl+F` con el foco en el panel (o el menú de la conversación) busca en la conversación, también en razonamientos plegados; un botón incluye la entrada y el resultado de las herramientas.
- Atajo `Alt+E` para alternar entre lectura y edición (además de `F2`): una pulsación entra a edición y la siguiente vuelve a lectura.
- Licencia: Synapse.rs es ahora software libre bajo GPL-3.0-or-later. No incluye `synapse-server/` (solo una prueba de concepto), el nombre ni los logotipos.
- Chat del agente: junto al contexto se ve cuánta cuota queda en la cuenta (`● 58% sem`), con todos los límites al pasar el ratón. Claude la adjunta a la conversación; con Antigravity se pregunta a `agy`, como mucho cada 3 minutos (configurable), y `/usage` la incluye.
- Chat del agente: el icono de cada agente del registro ACP se ve antes de su nombre; los desconocidos llevan el robot genérico.
- Chat del agente: al reabrir una conversación, la barra de contexto se ve sin esperar a la primera respuesta.
- Explorador: los archivos y carpetas arrastrados desde otra aplicación (Nautilus, Explorador, Finder…) se copian en la carpeta donde se sueltan.

### Cambiado

- Tableros de tareas: en el diálogo de detalle de la tarea, los comentarios se muestran ahora del más reciente al más antiguo con la caja para comentar arriba, el asignado (`@`) y el creador aparecen debajo del título del diálogo, el panel de detalles es más ancho por defecto con un separador arrastrable, y la lista de detalles incluye «Creado por», «Modificado por» y fechas adicionales del ciclo de vida de Azure DevOps (modificado, cambio de estado, activado, resuelto y cerrado).
- Historial del agente: cada conversación empieza por el número que tenía (`#1`) y su fecha, o solo la hora si es de hoy. Las guardadas antes no muestran número.
- Chat del agente: la notificación de fin de turno muestra ahora la respuesta final del agente en lugar de tu pregunta; en Configuración se puede volver a la pregunta.
- Chat del agente: copiar una respuesta (un solo botón «Copiar») o bloques completos de ella con `Ctrl+C` pone el Markdown original como texto plano y la versión con formato como HTML: en un campo de texto se pega Markdown y en un correo, con formato.
- El buscador de archivos (Ctrl+O) y el de contenido (Ctrl+Shift+F) recuerdan lo escrito, filtros incluidos, hasta cerrar Synapse; reaparece seleccionado, así que escribir lo reemplaza.
- El índice de búsqueda se reconstruye una vez al abrir cada bóveda: los PDF se indexan ahora página a página y los documentos de Word párrafo a párrafo.
- Panel del agente: cerrarlo solo lo oculta; reabrirlo es más rápido con conversaciones largas y conserva el scroll.
- Panel del agente: acoplado, el pin ocupa el lugar de la X (el panel se cierra con su botón de la barra); flotante conserva la X.

### Corregido

- Breadcrumb: ahora muestra el nombre completo del archivo cuando hay espacio en lugar de cortarlo a los 220 px, y si la barra se estrecha se contraen primero las carpetas iniciales de la izquierda para conservar el nombre del archivo.
- Prompts del agente y MCP: se localizaron los textos enviados a los agentes (`/feedback`, preguntas aparte `/btw` y reconstrucción de conversación) para seguir el idioma activo de la interfaz (`en`/`es`), y se tradujeron al inglés el contexto de adjuntos/bóveda y las descripciones, prompts, mensajes de recursos y respuestas del servidor y herramientas MCP.
- Interfaz: al cambiar entre inglés y español ya se actualiza el resto de la aplicación (menús y diálogos del explorador, panel y chat del agente, Ajustes, Atajos, Apertura rápida, búsqueda, temas, historial y comparaciones de Git, complementos, actualizaciones, errores de compatibilidad con Obsidian, visores y editores, diagramas y selectores nativos de archivos o carpetas), así como los mensajes de error del backend que ve el usuario. Los bloques `board` incrustados en vista de lectura también se vuelven a pintar al instante cuando cambia el idioma.
- Tableros de tareas: con la aplicación en inglés, el editor de tableros, el diálogo de tarea y el marco de los diálogos redimensionables ya no muestran textos en español; los tableros y tareas nuevos reciben sus nombres por defecto en el idioma de la interfaz. También se traducen los motivos que llevan sus mensajes de error (un nombre de tarea repetido salvo por las mayúsculas, una revisión que falta, 1Password sin responder, un `config.jsonc` ilegible…); lo que escriben el sistema o el servidor sale como está. También los avisos de error del explorador y los motivos por los que se rechaza una operación con archivos (un archivo que pertenece a un tablero, un nombre ya en uso, un elemento que no existe, una ruta no válida…).
- Visor de imágenes: `Shift` + rueda del mouse desplaza la imagen en horizontal (en Linux no hacía nada), igual que el desplazamiento horizontal del touchpad.
- Chat del agente: la lista de `@` ya no deja de filtrar cuando dos bóvedas de Synapse se llaman igual.
- Chat del agente: `@` sigue buscando tras un espacio, así que se encuentran archivos y carpetas con espacios en el nombre (deben aparecer todas las palabras). `Enter` justo tras un espacio envía el mensaje; `Tab` elige la sugerencia.
- Explorador: los archivos borrados muestran ahora una insignia «D», verde si el borrado ya está en stage (por eso no se ofrece «Git Add») y roja mientras no lo está; antes no tenían insignia. Una carpeta borrada cuyo borrado está entero en stage se ve en verde en vez de rojo.
- Chat del agente: `@` sugiere primero los archivos que mejor casan (por nombre y luego por ruta); en bóvedas grandes un archivo como `main.yaml` podía quedar fuera por otros que solo coincidían en letras sueltas.
- Atrás en el historial vuelve al `index.md` del que se salió por un enlace, también cuando era la pestaña restaurada al abrir o activada al cerrar otra.
- Notas: al volver de edición a lectura se conserva la posición también en las notas que tardan en pintarse (muchos enlaces, incrustaciones o imágenes), en vez de saltar al principio o a un título visitado antes.
- Conversaciones con agentes: al cerrar una ahora se cierra también su sesión en el agente (`session/close`); con Claude sus procesos seguían vivos mientras otras conversaciones compartieran el adaptador.
- MCP: `full_text_search` con `OR` (`TIEL OR QTI1`) no encontraba nada: la última palabra se buscaba también como prefijo y se exigían las dos partes.

- Búsqueda en el chat del agente y en documentos de Word: una sola letra con salidas largas de herramientas ya no congela la aplicación; solo se resaltan las coincidencias a la vista.
- Búsqueda en documentos de Word: las palabras partidas por el formato se encuentran enteras, se conservan `&` y otras entidades, y un documento más largo que el límite del índice ya no falla en una letra con tilde. El índice se reconstruye una vez.
- Explorador (Linux): arrastrar archivos dentro del explorador, o al chat del agente, vuelve a funcionar.
- Notas: al cambiar entre lectura y edición en medio de una tabla larga se conserva la fila visible, en vez de saltar al principio de la tabla.
- Chat del agente: elegir un archivo con `@` deja su referencia en el texto (`@docs/nota.md`) en vez de borrar lo escrito; se sigue adjuntando.
- Notificaciones en Linux: el pulpo se envía también como imagen de la notificación, para demonios como swaync.

### Seguridad

- Dependencias actualizadas. El visor de Excel pasa a `umya-spreadsheet` 4 (beta), que deja atrás una versión de `zip` con vulnerabilidades conocidas.

## [1.0.0-beta.9] - 2026-11-24

### Añadido

- Paleta de comandos (Ctrl+P): los nombres salen en el idioma de la interfaz y los comandos se encuentran en inglés y en español (`update`, `upgrade`, `cerrar pestaña`…). El más parecido queda primero.
- Novedades y notas de la actualización en inglés o español, según la interfaz. El CHANGELOG tiene ahora las dos versiones.

### Corregido

- Windows: las imágenes abiertas como archivo no se mostraban al abrir la bóveda desde recientes o la terminal, porque su ruta llevaba el prefijo de ruta larga `\\?\`.

### Cambiado

- El inglés es el idioma base: si falta un texto en español sale en inglés, y con el sistema en otro idioma Synapse arranca en inglés.

## [1.0.0-beta.8] - 2026-11-24

### Seguridad

- Dibujos Excalidraw en la API de plugins: un archivo manipulado podía ejecutar código al mostrarse. `toSVG` escapa sus valores y la vista sanea el SVG.
- API de plugins: `sanitizeHTMLToDom` sanea de verdad el HTML y `htmlToMarkdown` ya no deja etiquetas reconstruidas.
- Acceso a archivos: la interfaz solo puede leer y escribir en la bóveda abierta y en los archivos que eliges (diálogo o línea de comandos); antes, en cualquier ruta. Un archivo suelto muestra además las imágenes de su carpeta y subcarpetas. Una bóveda que no esté en recientes pide confirmación. Un fondo de pantalla elegido en una versión anterior hay que volver a elegirlo, y las imágenes con ruta absoluta de fuera de la bóveda ya no se muestran.
- Guardar un agente personalizado o escribir otra ruta de Git pide confirmación con un diálogo del sistema que muestra el programa que se ejecutará.
- Instalador de agentes: los permisos especiales (SUID/SGID/sticky) de un ZIP se descartan.
- Dependencias con vulnerabilidades conocidas y parche compatible, actualizadas (`url`, `cookie` y `lodash-es`).

### Corregido

- Git Log de todas las ramas: el menú contextual aparecía un instante en otro sitio y saltaba a su posición, así que un clic rápido podía caer en otra opción.
- Explorador con muchos archivos y zoom de interfaz distinto de 100 %: moverse por la lista o abrir un archivo bloqueaba la app varios segundos, porque la altura de fila oscilaba y la lista se volvía a montar cientos de veces.
- Índices de carpeta (`index.md`): el candado y el estado de Git quedan junto a los nombres y alineados en columna, en vez de en el borde derecho; la descripción va a continuación.
- Chat del agente: con Claude no aparecía el selector de modelo, porque ya lo anuncia en `configOptions`. Synapse lee ese formato y cambia el modelo con `session/set_config_option`.
- Pruebas unitarias (`bun test`): el plugin que compila los `.svelte.ts` quita los tipos con el transpilador de Bun, porque TypeScript 7 ya no trae `transpileModule`; el script `test` solo mira `./src` (ya no recoge la carpeta `zennotes/`).
- Chat del agente: lo escrito sin enviar ya no se pierde al cerrarse el panel (por ejemplo, sin fijar, al hacer clic fuera) ni al plegar la conversación.
- Enlaces internos con caracteres reservados (`%2C`, etc.) en vista de lectura y `index.md` OKF: se decodifican con `decodeURIComponent` en lugar de `decodeURI` para abrir correctamente archivos con comas u otros caracteres especiales en el nombre (como `.excalidraw`).
- Chat del agente: corregido un error por el que los mensajes encolados mientras el agente trabajaba no se enviaban al terminar el turno.
- Diálogos redimensionables (Índice de la nota `Ctrl+F12` y diálogos de Git): se muestra el indicador de redimensionado en la esquina inferior derecha, se amplían las áreas de agarre en bordes y esquinas y se captura el puntero durante el arrastre para redimensionar de forma fiable.
- Chat del agente: al colapsar un bloque de razonamiento ahora muestra «Razonamiento…» junto al icono en lugar de solo el emoji, y al expandirlo se elimina el espacio en blanco sobrante antes de las tarjetas de herramientas.
- Chat del agente: los enlaces `file://` de sus respuestas no abrían nada (el saneado del HTML les quitaba el destino). Ahora abren el archivo de la bóveda y un ancla `#L56-L65` lo abre en el editor con esas líneas seleccionadas. Si la ruta no cae en la bóveda (otra raíz u otra copia del repositorio), se busca por el final de la ruta en el índice: uno se abre, varios dan a elegir.

### Añadido

- Chat del agente: selector de nivel de esfuerzo junto al modelo, con los agentes que lo ofrecen (Claude, agy). Se recuerda por agente. Con agy, el esfuerzo ya no se repite en la lista de modelos.
- Notas Markdown: en lectura, los títulos y subtítulos se pliegan con la flecha de su izquierda (Alt+clic, todos los de ese nivel; `zM`/`zR`, todos). La búsqueda, el índice y los enlaces a un título despliegan lo necesario. Comandos «Plegar / Desplegar todos los títulos de la nota», también en edición.
- Al terminar el agente, además del sonido, Synapse envía una **notificación del sistema con el pulpo kawaii** y la consulta que terminó (`Terminó la consulta: "…"`) si la instancia que lo ejecuta no tiene el foco (por ejemplo, si estás en otra aplicación). Los avisos de permisos y preguntas del agente usan ahora el mismo icono.
- Los archivos **`.sql`** son un tipo soportado: aparecen en el explorador y en Ctrl+O, se indexan en la búsqueda de contenido como código y se editan con resaltado de sintaxis SQL (también en el diff de Git).
- Tema integrado **GitHub** (títulos verdes sobre fondo casi negro en oscuro; blanco en claro), también como «GitHub Islas».
- Tema integrado **Solarized** (crema en claro, azul petróleo en oscuro, títulos cian), también como «Solarized Islas».
- Tema integrado **Willemstad** (negro cálido en oscuro, blanco y salvia en claro, títulos en franjas verdes), también como «Willemstad Islas».
- Cambio rápido de tema: `` Ctrl+` `` o el comando «Elegir tema…» abre la lista de temas con búsqueda, sin entrar a Configuración. El atajo se puede cambiar (`pick_theme`).
- Búsqueda global y Quick Open: al abrir un resultado, el buscador del editor (Markdown en lectura y edición, YAML/JSON y texto) queda cargado con el término buscado y posicionado en la primera coincidencia.
- Chat del agente (ACP): `/btw <pregunta>` hace una pregunta aparte sin interrumpir el turno en curso. La respuesta sale en un panel flotante sobre la caja de texto, no entra en la conversación ni en el historial y se cierra con Esc; «Convertir en conversación» la continúa aparte. Rechaza los permisos que pida el agente.
- Chat del agente (ACP): `/fork [mensaje]` abre otra conversación con el contexto de la actual, también con el turno en curso. `/btw` y `/fork` usan `session/fork` si el agente lo anuncia (Claude) y, si no, le reenvían la conversación como texto (agy); la sugerencia del comando dice cuál de las dos.
- Barra de pestañas: opción «Cerrar todas las pestañas» en el menú contextual de cada pestaña.
- Chat del agente (ACP): sonido al terminar un turno (`Burbuja FM` por defecto), configurable en Ajustes → Agentes de IA con opciones `Burbuja FM`, `Sonar orgánico` o `Sin sonido` y botón «Probar».
- Chat del agente (ACP): soporte completo para `elicitation/create` en modos `form` (formulario interactivo generado desde JSON Schema con botones/selects para enums, booleanos, números, texto y validación de obligatorios) y `url` (muestra dominio, URL y botón «Abrir enlace»), anunciando `clientCapabilities.elicitation` en `initialize` y respondiendo con `accept`, `decline` o `cancel`.
- Visor de diff de texto (`@codemirror/merge`): navegación entre bloques de cambio (↑/↓ con contador), búsqueda integrada (`Ctrl+F` y botón en la barra), edición directa del archivo en disco en diffs del árbol de trabajo sincronizada con el botón Lectura/Edición (`Ctrl+E`) con botones para revertir bloques individuales (`→` en vista dividida, «Revertir» / «Mantener» en unificada), autoguardado y soporte completo del editor CodeMirror (modo Vim, plegado, comentarios, renombrar ocurrencias, atajos de línea, autocompletado e historial `Ctrl+Z`), además de resaltado de sintaxis y colores de diff adaptados a tema claro y oscuro.
- Git Log del repositorio: las columnas se pueden reordenar arrastrando sus cabeceras (orden guardado) y «Hash» pasa a ser la primera columna por defecto.
- Pantalla de bienvenida: las bóvedas fijadas se pueden reordenar con drag and drop (orden guardado en `workspaces.json`) y las fijadas con repositorio Git conservan el icono de carpeta Git con el color de fijado.
- Índice de Markdown y claves YAML/JSON: el popover del índice reutiliza `@shared/ui/dialog-resize` para redimensionarse arrastrando sus bordes o esquinas y recordar su tamaño.
- Índice de carpeta (`index.md`): «+ Nuevo…» (desplegable: nota Markdown, Mermaid, D2, PlantUML, Excalidraw) junto a «Archivos»/«Conceptos» y «+ Carpeta» junto a «Carpetas», que crean dentro de esa carpeta; si falta la sección, los botones van arriba.
- Explorador: clic en una carpeta abre su `index.md` (hacer clic en el icono de carpeta o en cualquier zona a su izquierda —flecha o indentación— la despliega/contrae; sin `index.md`, se despliega como antes). Ajustes → «Clic en una carpeta del explorador» permite volver a «Desplegarla». En ese modo el `index.md` no se lista dentro de su carpeta (el de la raíz sí); sigue en disco y en el MCP. «Localizar archivo activo» (Ctrl+Alt+1) sobre un `index.md` selecciona su carpeta sin desplegarla.
- Editor: al pasar el ratón por una marca del margen de Git aparece un tooltip con lo que había en Git y lo que hay ahora (−/+), con las letras que cambiaron resaltadas (aunque sea una sola); en un borrado, las líneas que se quitaron. Botones «Revertir» (devuelve el bloque a como está en Git; Ctrl+Z lo deshace) y «Copiar original». Colores propios en claro y oscuro (como el diff de GitHub), legibles con cualquier tema. El tooltip sale junto a la línea bajo el ratón y sigue abierto al pasar a él.
- Telemetría de agentes: registro global de uso por iteración en `~/.config/synapsers/agent-usage.jsonl` (modelo, sesión, duración, tokens/costo y referencias a los JSON de cada bóveda) y sidecar detallado por conversación en `<bóveda>/.synapse/agent-sessions/telemetry/<id>.json` con tiempos de iteración, uso de herramientas y correlación de request/response de llamadas al MCP de Synapse.
- Tablas Markdown: las celdas con `[ ]`, `[x]` o `[X]` se renderizan como checkboxes interactivos en el editor en vivo, el editor visual de celdas y la vista de lectura.
- Chat del agente: tus mensajes se muestran en Markdown (negrita, listas, código, enlaces…) con las menciones resaltadas, y tienen un botón «Copiar» que copia el texto tal como lo escribiste para reusar el prompt.
- La pestaña de un `index.md` muestra el nombre de su carpeta (el de la bóveda para el de la raíz) con un icono de carpeta.
- Imágenes y adjuntos (todo enlace que no es una nota `.md`) se buscan también en la carpeta `attachments/` junto a la nota: para `a/b/c/NOTA.md`, primero `a/b/c/imagen.png`, luego `a/b/c/attachments/imagen.png`, y después la regla de siempre (raíz de la bóveda, único por nombre o elegir).
- Panel del agente con pin, como el explorador: fijado ocupa su espacio a la derecha; suelto flota sobre el editor sin quitarle espacio y se oculta al hacer clic fuera o con Esc.
- Índice OKF (Open Knowledge Format v0.3) en cada carpeta: Synapse mantiene un `index.md` con sus subcarpetas, conceptos (notas con `type`, con su `description`) y demás archivos, para que un agente pueda recorrer la bóveda desde el disco. Se regenera al cambiar la carpeta y solo se escribe si cambia (pensado para commitearse); en la raíz lleva `okf_version: "0.3"` y conserva `title`, `description`, `license` y `x-…`. Un `index.md` propio se renombra a `index.deprecado.md`. En Synapse se abre en solo lectura (sin el botón de lectura/escritura) y no sale en la búsqueda; un enlace a una carpeta (`carpeta/`) abre su `index.md`. `SYNAPSE_OKF_INDEX=0` lo desactiva.
- MCP: `read_note` devuelve además el `index.md` de la raíz y el de la carpeta de la nota.
- Diff de texto: una línea indica qué es cada lado (◀ antes · después ▶, o − antes / + después).
- Enlaces con una sola regla en notas (wikilinks y enlaces Markdown), panel de relaciones y chat del agente: si la ruta existe tal cual (relativa a la nota o a la raíz) se usa esa; si no, el único archivo con ese nombre; si hay varios, una lista para elegir (↑/↓; Enter o clic abre; Ctrl+Enter, Ctrl+clic o «Fijar» abre y corrige el enlace en la nota con la ruta completa). Una imagen incrustada ambigua muestra un aviso con «Elegir cuál».
- Chat del agente: las imágenes se abren en una pestaña («Agente · …», con el icono del agente) con el visor de imágenes (zoom, mover, ajustar) y el botón «Guardar en la bóveda».
- MCP: `get_backlinks_and_relations` indica en cada enlace saliente `resolved_path` y, si es ambiguo, `ambiguous` con los candidatos.
- Chat del agente: mientras el agente trabaja, Enter deja el mensaje en cola (con sus adjuntos y menciones) y se envía solo al terminar el turno, en orden. Cada mensaje en cola tiene ✕ para quitarlo, ↑ con la caja vacía saca el último para editarlo, y «Detener» corta el turno y devuelve lo encolado a la caja sin enviarlo. Funciona con cualquier agente ACP (la cola la lleva Synapse).
- Chat del agente: las respuestas muestran las casillas de tarea (antes el saneado del HTML las quitaba); bajo cada respuesta, «Markdown» copia el texto tal cual y «Con formato» lo copia para un correo o Teams, con las tareas como ☐ / ✅; copiar una selección con tareas hace lo mismo.
- Chat del agente: zoom de los mensajes y la caja de texto con Ctrl+rueda, Ctrl++, Ctrl+− y Ctrl+0 (se recuerda).
- Chat del agente: la caja de texto crece con lo que escribes, hasta el 40 % de la altura de la ventana; después tiene su propio scroll.

### Cambiado

- Índice de carpeta (`index.md` OKF): las listas de carpetas, conceptos y archivos se renderizan en vista de lectura con aspecto similar al explorador (iconos de carpeta y tipo de archivo, metadatos discretos, estado Git y candado, y fila completa clickeable) sin modificar el archivo físico OKF en disco.
- Visor de diff de texto: migrado de `@git-diff-view/svelte` a `@codemirror/merge` para mejorar el rendimiento en archivos grandes.
- Menús contextuales del explorador de bóveda y breadcrumb: todas las opciones de Git ahora aparecen agrupadas juntas en la parte superior.
- Diálogos redimensionables unificados sobre `@shared/ui/dialog-resize` en `QuickOpen`, `FullTextSearch`, `CommandPalette`, `SettingsPanel`, `ShortcutsPanel` y `CommunityPluginsModal` (conserva tamaños guardados, grip en esquina inferior derecha y arrastre desde la cabecera en plugins comunitarios).
- Si el explorador y el panel del agente están fijados se recuerda por bóveda (la primera vez cada bóveda toma el último valor global).
- Diff visual «2 Paneles»: izquierda = antes (Git, versión anterior, primer archivo) y derecha = después (disco, versión nueva), igual que el diff de texto; el panel editable del disco pasa a la derecha.
- Al guardar una nota Markdown ya no se añade `title` al frontmatter (ni se crea un frontmatter solo para eso): el título por defecto es el nombre del archivo. Al renombrar, el `title` solo cambia si ya existía y coincidía con el nombre anterior; un título propio se respeta. El `write_note` del MCP solo escribe `title` si se le pasa.
- El diff de texto se abre colapsado, mostrando solo los cambios («Expandir todo» en la barra para verlo entero). `diff.max_expand_lines` en config.jsonc vale ahora 0; con un número mayor, los archivos de hasta esas líneas se abren expandidos como antes.
- Un enlace con varios archivos del mismo nombre ya no lleva al «más cercano» sin avisar: se pregunta. Esos enlaces ambiguos no cuentan en backlinks, grafo ni relaciones hasta que se fijan.
- Visor OpenAPI: Scalar ya no usa la URL de la ventana (`#…`); con varias pestañas de OpenAPI cada una va por su cuenta (antes una pestaña nueva saltaba a la sección de la otra).

## [1.0.0-beta.7] - 2026-11-20

### Añadido

- Zoom en los visores de Word y de hojas de cálculo que agranda todo, como en un PDF (no solo la letra): en Word la página entera, con «Ajustar al ancho»; en las hojas, anchos de columna, altos de fila, letra y cabeceras, sin dejar de pintar solo lo visible. De 50 % a 300 % con el control de la barra, Ctrl+rueda o los atajos de tamaño (Ctrl++, Ctrl+−, Ctrl+0), que dentro de estos visores hacen este zoom. Se recuerda un zoom para todos los Word y otro para todas las hojas.
- Al copiar desde el modo lectura de Markdown una selección con tareas, las casillas se copian como ☐ (pendiente) y ✅ (hecha), con la lista y la negrita, para pegarlas en un correo o en Teams web (que quitan las casillas HTML); también en texto plano (`- ☐ tarea`). Sin tareas se copia como siempre.
- Atajo configurable «Marcar / desmarcar tarea» (`toggle_task`, Ctrl+Enter por defecto) en el editor de Markdown, como en Obsidian: alterna `- [ ]` ↔ `- [x]` en la línea del cursor (o en todas las de la selección), añade la casilla a un elemento de lista que no la tiene y convierte una línea normal en tarea. Funciona también con Vim, en modo normal e inserción.
- Visor de documentos Word (`.docx`, `.docm`, `.dotx`) dentro de Synapse, en solo lectura, con docx-preview (se carga al abrir el primero): páginas, fuentes, colores, tablas, imágenes, encabezados y pies. El `.doc` antiguo sigue abriéndose con la aplicación del sistema.
- Botón «Abrir en la aplicación» en los visores de Word y de hojas de cálculo, para editar con la aplicación del sistema.
- Visor de hojas de cálculo dentro de Synapse (antes se abrían con la aplicación del sistema), en solo lectura: `.xlsx`, `.xlsm`, `.xltx`, `.xltm` con estilos (negrita, colores, rellenos, bordes, alineación), celdas combinadas y anchos/altos; `.xls`, `.xlsb`, `.ods` solo con valores y fórmulas. Los libros grandes se abren sin cortar: cada hoja se lee al abrirla, las celdas llegan por tramos de filas según te desplazas, solo se pintan las filas y columnas a la vista, y el libro leído queda en caché mientras se usa; un `.xlsx` de más de 10 MB se lee sin estilos para abrir el doble de rápido. Barra de fórmulas con la celda seleccionada, botón «Fórmulas» (Ctrl+`) para verlas en la rejilla, aviso en las fórmulas sin resultado guardado, pestañas de hojas abajo (las ocultas solo al pedirlas; Alt+RePág/AvPág cambia de hoja), navegación con flechas y Ctrl+C para copiar la celda.
- Actualizar una rama que va por detrás de su remota sin salir de Synapse (fast-forward, sin crear commits): el badge de la rama muestra ↓N / ↑N, el diálogo de ramas tiene «Actualizar desde origin/…», y el menú del Git Log ofrece «Avanzar «rama» hasta aquí» en los commits posteriores. Si las ramas divergieron no se toca nada y se explica por qué.
- Comando «Zoom: toda la interfaz» en la paleta (Ctrl+P): un selector pequeño de 70 % a 200 % que se maneja con ↑/↓, Enter y Esc, con vista previa al moverse; se guarda en los ajustes. El zoom del texto del contenido sigue en Ajustes y ahora también lo sigue el visor OpenAPI (solo la letra, como Markdown y el editor).
- El visor OpenAPI sigue los `$ref` a otros archivos de la bóveda (`./paths/x.yaml`, `../schemas.yaml#/Pet`), también en las revisiones de Git; avisa de las referencias que faltan y se actualiza al guardar un archivo referenciado.

### Cambiado

- El visor OpenAPI ya no muestra «Ask AI» de Scalar: el chat enviaba la especificación a servidores externos.
- **Icono de la app**: los iconos de `src-tauri/icons/` se regeneran desde `src/shared/assets/synapsers.svg`.
- **CI de Windows**: el paso de dependencias de compilación cruzada instala solo `clang lld llvm nasm` (el destino es Windows; sobraban las bibliotecas GTK/WebKit de Linux) y se retira el caché de APT, que no tenía efecto. Ahorra unos 7 minutos por release.
- **Caché de xwin**: la ruta cacheada era `~/.cache/xwin`, que `cargo-xwin` no usa; ahora es `~/.cache/cargo-xwin`, así la CRT y el SDK de MSVC (~8 min de descarga) se reutilizan entre ejecuciones.

### Corregido

- El `Buffer` que Synapse ofrece a los plugins de Obsidian tomaba cualquier `Uint8Array` por un `Buffer`, lo que hacía que JSZip leyera mal los nombres de los archivos dentro de un zip (por ejemplo al abrir un `.docx`).
- El menú contextual del Git Log sale junto al puntero (antes aparecía desplazado) y no se sale de la ventana.
- «Checkout de este commit» del Git Log fallaba al confirmar porque leía el commit del menú ya cerrado.

## [1.0.0-beta.6] - 2026-11-20

### Añadido

- **Actualizaciones desde la app**: al iniciar (desactivable en *Ajustes → Actualizaciones*) o con «Synapse: Buscar actualizaciones» en la paleta, Synapse consulta la última release y, si hay una versión nueva, avisa abajo a la derecha con «Ver novedades», «Más tarde» y «Actualizar». La descarga se verifica con la firma de la release y el ejecutable se reemplaza (en Linux y Windows la release publica el binario suelto, que el instalador de Tauri no sabe instalar); después, «Reiniciar ahora». `latest.json` lleva ahora las novedades de la versión.
- **Novedades**: «Synapse: Novedades (changelog)» abre el historial de cambios en una pestaña en memoria (no es un archivo del vault). La primera vez que se abre una versión nueva, se abren solas las novedades desde la versión que se usaba.
- **Ejecutable de Git configurable**: si la bóveda es un repositorio y no se encuentra Git, un diálogo permite elegir el ejecutable (o «No volver a preguntar»); la ruta se guarda en `config.jsonc` (`git.path`) y se cambia en *Ajustes → Git*, que muestra la versión en uso y permite volver al del sistema. Todas las operaciones que usan el Git real la respetan, y en Windows ya no abren una ventana de consola por comando.

### Corregido

- **Bóveda con Git que no se detectaba (Windows)**: si gix abre el repositorio pero falla al leer el estado de los archivos, la bóveda se sigue mostrando como repositorio (rama y acciones de Git, sin las marcas de cambios) y el motivo queda en el log de la app. Las rutas de los archivos cambiados usan `/` también en Windows (salían con `\` y el explorador no las marcaba).
- **Clave pública del actualizador**: estaba en `tauri.conf.json` como la línea interior de minisign (`RWQ…`) en vez del formato de Tauri (el archivo de clave en base64), así que ninguna descarga se habría podido verificar e instalar. Es la misma clave: las releases ya publicadas se verifican con ella.

- **e2e de agentes sin red**: la lista de agentes esperaba hasta 30 s al registro público de ACP (sin red o con el CDN colgado), y la prueba del panel del agente fallaba. Las e2e apuntan `SYNAPSE_ACP_REGISTRY_URL` a un puerto cerrado y usan solo los agentes locales y personalizados.

### Cambiado

- **e2e**: el puerto de WebDriver se configura con `E2E_DRIVER_PORT` (por defecto 4444; WebKitWebDriver usa el siguiente), para poder correr varios procesos de pruebas a la vez.

## [1.0.0-beta.5] - 2026-11-20

### Añadido

- **Release verificable**: cada release adjunta `SHA256SUMS.txt`, el SBOM CycloneDX del backend Rust y del frontend, y una attestation de procedencia (Sigstore, `gh attestation verify`) de cada paquete. No cambian los paquetes ni las firmas del actualizador.
- **Workflow `Security`**: `cargo audit`, `cargo deny` (avisos, licencias compatibles con GPL y fuentes, con `src-tauri/deny.toml`) y `bun audit` en cada push a `main`, en los PR y una vez por semana. CodeQL (JavaScript/TypeScript y Rust; Rust no bloquea) y OpenSSF Scorecard en workflows propios.
- **Dependabot semanal** para Cargo, bun y GitHub Actions, con las actualizaciones menores y de parche agrupadas en un PR por ecosistema.
- **Vista en árbol en el diálogo de commit**: un botón junto a «Seleccionar todos» cambia la lista de archivos a un árbol de carpetas (las cadenas de una sola carpeta se compactan, como `docs/guias/`). Cada carpeta se pliega y tiene su casilla, que marca o desmarca todo lo que contiene; la vista elegida se recuerda.
- **«Git Log…» y «Ramas…» en el explorador**: el clic derecho en el espacio vacío del explorador, en el nombre del vault o en el badge de la rama abre el log del repositorio o sus ramas. Antes, si el vault era un único repositorio, el log solo se abría desde la paleta de comandos.
- **`SECURITY.md`** (cómo reportar y cómo verificar un binario) y una sección «Seguridad y privacidad» en el README con el mapa de conexiones de red de la app.

### Cambiado

- **README de presentación**: el README es ahora la presentación del producto (antes `docs/about.md`, que queda como enlace a él); la documentación de desarrollo pasa a `CONTRIBUTING.md`. Ya no dice «100 % local»: Synapse es local-first y una sección nueva, «Qué sale de tu equipo», detalla qué funciones usan la red (draw.io, PlantUML, agentes de IA, plugins, Git) y qué viaja en cada una. La landing (`docs/index.html`) se corrige igual.
- **Workflows con permisos mínimos**: `contents: read` por defecto y `contents: write`, `id-token` y `attestations` solo en el job que publica la release. Las acciones están fijadas por SHA de commit y `dtolnay/rust-toolchain` y `taiki-e/install-action` reciben ahora `toolchain` y `tool` explícitos.
- `src-tauri/Cargo.toml`: `publish = false` (el crate no se publica en crates.io; lo necesita cargo-deny para no exigirle licencia).

### Corregido

- **SBOM del frontend en la release**: se genera con syft sobre las dependencias de producción. `cyclonedx-npm` no funcionaba con bun (`Unexpected npmJsPath`) ni con npm (conflicto con los `overrides`), y el job «SBOM» bloqueaba la publicación.
- **Versiones de un OpenAPI en el historial de Git**: «Ver archivo» muestra la API con Scalar, como en Lectura, en vez del YAML/JSON en texto plano; el botón Raw sigue mostrando el texto.
- **Checkout de ramas remotas**: «Checkout» en una rama remota sin rama local crea la local con seguimiento (`git checkout -b feature/x --track origin/feature/x`) en vez de dejar HEAD suelto. Y una rama local con `/` (`feat/y`) ya no puede saltar a otra local llamada `y`.
- **Vista Raw de una versión**: ya no dibuja una línea horizontal bajo cada línea de texto (heredaba el estilo de `code` en línea).

## [1.0.0-beta.4] - 2026-11-19

### Añadido

- **Búsqueda en diagramas BPMN**: de un `.bpmn` se indexa el nombre de cada paso con su tipo (tarea, evento, compuerta…), su documentación y las anotaciones, sin las coordenadas del dibujo; nuevo filtro `kind:bpmn`. El índice se reconstruye solo al abrir la bóveda.
- **El grafo y el láser siguen el tema**: nodos, nombres y enlaces del grafo (2D y 3D) y el trazo del láser toman sus colores de variables del tema (`--graph-*`, `--laser`) y se repintan al cambiar de tema. En tema oscuro los nombres de las notas del grafo ya se leen (antes gris oscuro sobre fondo oscuro).
- **Agente GitHub Copilot por ACP**: aparece en la lista de agentes junto a Claude y Antigravity. Usa el CLI `copilot` ya instalado (`copilot --acp`); si falta, indica cómo instalarlo (`npm install -g @github/copilot`).

### Cambiado

- **Botón Láser**: el punto ya no sale estirado como una píldora y, activo, usa el color del láser del tema.
- **Pruebas e2e 6 veces más rápidas (`syna-7qa9`)**: cada archivo esperaba 30 s de más porque se buscaba Vite en 127.0.0.1 y Vite escucha en ::1. Ahora arranca en ~2 s, Vite se reutiliza entre archivos y se cierra solo al terminar; la suite completa pasa de ~34 min a ~5,5 min (160/160). `test:e2e:quick` corre todo en un proceso. Si el WebDriver se cae, sus procesos se cierran todos (antes WebKitWebDriver quedaba huérfano).
- **CI y release**: un workflow nuevo (`ci.yml`) comprueba tipos y corre las unitarias del frontend y de Rust en cada push a `main` y en los PR. El release compila Linux, Windows y macOS en paralelo (antes en serie) y publica la release de una vez al final, con `latest.json` armado con las plataformas que compilaron; si falla Mac se publica sin él. El caché de Rust se calienta desde `main` (al cambiar `Cargo.lock` y cada lunes): el que guardaba cada tag no lo podía usar el siguiente, así que cada release compilaba todas las dependencias desde cero. La validación del tag también comprueba tipos y unitarias. Los e2e siguen siendo locales.
- **Pruebas e2e sin pausas fijas (`syna-7qa9`)**: `diagram-pan-zoom`, `compare-diagrams`, `compare-files-git`, `compare-images`, `explorer-keyboard-focus`, `editor-tab`, `document-words` y `drawio` esperan la condición real (diagrama quieto, zoom aplicado, selección del árbol, texto del diff, PNG reescrito) en vez de pausas fijas. Nuevo `bun run test:e2e:quick [nombre…]`: smoke más las e2e tocadas o nombradas.

## [1.0.0-beta.3] - 2026-11-19

### Añadido

- **Índice de claves en YAML y JSON**: el editor de código de `.yaml`, `.yml` y `.json` tiene el mismo botón de índice que las notas: las claves en árbol (hasta 4 niveles; los elementos de lista como `- valor`, sin las `$ref`), con búsqueda rápida, plegado y la sección del cursor resaltada. Al elegir una clave, el cursor salta a ella. **Ctrl+F12** (configurable como `toggle_outline`, también en la paleta) abre y cierra el índice de la pestaña activa (el de claves en YAML/JSON y el de títulos en las notas) en un diálogo centrado que se mueve arrastrando la cabecera (doble clic lo recentra) y cambia de tamaño desde bordes y esquinas, recordando su tamaño; el botón del editor sigue abriendo el desplegable junto a él. En ambos índices, desde la búsqueda, ↑/↓ (y RePág/AvPág) recorren las entradas visibles, →/← despliegan y pliegan la activa y Enter va a ella.
- **Seguir los `$ref` de YAML y JSON**: en el editor de código, los valores de `$ref` (OpenAPI, JSON Schema) se subrayan; con Ctrl+clic o Ctrl+Enter se abre el archivo referido (ruta relativa al archivo actual) y se salta a la clave del puntero (`./paths/a.yaml#/post/responses`, también con `~1` en rutas de OpenAPI e índices de lista). Los punteros internos (`#/components/schemas/Error`) saltan dentro del mismo archivo, y una referencia rota avisa en vez de abrir nada.
- **Git Log de todas las ramas**: un diálogo al estilo de la ventana Git Log de JetBrains con los commits de todas las ramas y tags del repositorio (`git log --all`), el grafo de ramas en colores, columnas de mensaje, autor, fecha y hash, y etiquetas de ramas locales, remotas y tags en cada commit. Al elegir un commit se ven su mensaje completo, autor y archivos cambiados; doble clic (o Enter) en un archivo abre su diff en ese commit. Filtra por rama o tag y por texto (mensaje, autor, hash o rama). El commit de HEAD se marca con una etiqueta «HEAD» (o «HEAD (separado)») y un punto hueco. Con clic derecho en un commit: checkout de sus ramas locales, checkout del commit (HEAD separado, con confirmación), nueva rama ahí (con o sin checkout, como `checkout -b`), crear tag (simple o anotado), borrar sus tags y ramas locales (con confirmación; una rama sin fusionar pide confirmar `git branch -D`) y copiar el hash. No borra nada del remoto. Se abre desde la paleta de comandos («Git: Log de todas las ramas») y desde el menú de una carpeta-repositorio; con varios repositorios tiene selector.
- **Diff píxel a píxel entre archivos de tipos distintos y entre draw.io**: «Comparar archivos» y los diffs de git/historial ofrecen «2 Paneles» y «Diferencias» siempre que los dos lados se puedan convertir en imagen, aunque sean de tipos distintos (una imagen contra un Excalidraw, Mermaid contra D2, draw.io contra Mermaid…). Dos `.drawio` se dibujan con draw.io (iframe oculto, export a PNG) y se comparan, sin perder el diff de texto del XML. draw.io necesita la URL de los ajustes: sin red solo funciona con un draw.io local. Si un lado no se puede dibujar, queda el diff de texto como antes.
- **La búsqueda de texto completo entiende mejor los identificadores y más formatos**: `AchOperationCrService` o `sendTransferRequest` se encuentran enteros y también por sus partes («operation», «transfer request»); `snake_case` y `kebab-case` ya se partían. Los XML (también `.xsd`, `.wsdl`, `.bpmn`) se indexan sin etiquetas, ids, coordenadas ni colores, en vez de crudos y cortados; las planillas Excel/OpenDocument por hojas y filas; y JSON/YAML con cada valor bajo su ruta de claves (`info.title: …`). El índice se reconstruye una vez.
- **Caché de análisis del MCP**: un modelo ArchiMate se recorre una vez y las consultas siguientes (`inspect_archimate_element`, `read_note` con `outline_only`) lo toman de memoria (de ~14 ms a ~0,3 ms con un modelo de 3,2 MB). Se invalida sola si el archivo cambia, guarda solo análisis costosos y tiene un límite de 64 MB con descarte del menos usado.
- **Modelos ArchiMate (`.archimate`) en el MCP**: `read_note` con `mode: "outline_only"` devuelve el índice del modelo (carpetas y vistas con su línea), y la herramienta nueva `inspect_archimate_element` busca un elemento por id o nombre y devuelve en una respuesta su tipo, documentación, propiedades (p. ej. `UNIQUECODE`), relaciones entrantes y salientes con el nombre del otro extremo, y las vistas donde aparece. La búsqueda de texto completo indexa sus nombres, documentación y propiedades (antes, el XML crudo se cortaba y se perdían).
- **Estructura de los diagramas de draw.io en el MCP**: `extract_note_diagrams` sobre un `.drawio`, `.drawio.png` o `.drawio.svg` (también con `revision`) devuelve por página los contenedores con sus figuras, las figuras sueltas y las conexiones (desde → hasta, con su texto); `read_note` devuelve el texto del diagrama, y sobre una imagen indica usar `render_diagram`, que ahora también devuelve imágenes (.png, .jpg, .svg…).
- **Filtros en las búsquedas del MCP**: `search_notes` y `full_text_search` aceptan `extensions` (p. ej. `["archimate"]`); por defecto `search_notes` sigue buscando solo Markdown. `search_in_note` acepta `max_matches`.
- **Fecha legible en `feedback.md`**: cada entrada lleva la fecha y hora además de la fecha unix.
- **Varios repositorios Git en una bóveda**: las subcarpetas que son repositorios muestran su rama junto al nombre (con un punto si tienen cambios), y sus archivos tienen estado de git, diff e historial como los de la raíz. La cabecera del explorador muestra la rama del repositorio del archivo activo. Los diálogos de commit y de ramas tienen un selector de repositorio, y el menú de la carpeta-repositorio ofrece Ramas, Commit y Push de ese repositorio. Un commit nunca mezcla repositorios. Las herramientas MCP de git (`get_vault_git_status`, `git_file_history`, `git_diff_revisions`, `git_read_revision`) usan el repositorio más cercano que contiene el archivo. Los repositorios se consultan en paralelo y la lista se reutiliza unos segundos.

- **Visor de especificaciones OpenAPI/Swagger con Scalar**: los `.yaml`, `.yml` y `.json` cuya clave raíz es `openapi` (3.x) o `swagger` (2.0) se abren en Lectura como documentación navegable, y en Edición con el editor de código. Toma los colores del tema activo de la app (también al cambiar entre claro y oscuro) y se recarga al cambiar el archivo en disco. Scalar va empaquetado y se carga solo al abrir uno de estos archivos: sin CDN, fuentes externas ni telemetría.

- **Resaltado de sintaxis YAML y JSON** en el editor de código para `.yaml`, `.yml` y `.json` (antes se coloreaban como Markdown).
- **El release también compila para macOS** (Apple Silicon e Intel) en un job aparte que corre después de publicar: sube `synapsers-darwin-*.app.tar.gz` firmados para el actualizador y suma las plataformas a `latest.json`. Sin firma de Apple, macOS pide abrirlo con clic derecho → Abrir.
- **Los archivos `.xml` y `.archimate` (modelos de Archi) se ven en el explorador de la bóveda**, se indexan en la búsqueda de texto completo y el agente los puede leer y buscar como el resto del código.

### Cambiado

- **Tablas en el editor de Markdown (modo En vivo)**: las líneas de la tabla ya no se reemplazan por un widget: siguen siendo texto del documento y se pintan como tabla (bordes, encabezado, filas alternas, `|` tenues), con las columnas alineadas midiendo el ancho real del texto (funciona con fuentes proporcionales y con enlaces dentro de las celdas). Así la numeración de las líneas de debajo ya no se desfasa, y con ↑/↓ el cursor entra en la tabla fila por fila en vez de saltarla entera. El editor de celdas sigue disponible con el botón «Editor visual» que aparece al pasar por el encabezado.
- **En Linux, la interfaz usa el renderizado rápido de WebKitGTK (DMABUF)**: draw.io y el resto de la app van más fluidos. Si con algún driver la ventana sale en blanco o parpadea, `SYNAPSE_DMABUF=0` vuelve al modo anterior.

### Corregido

- **Foco al reabrir una bóveda**: la pestaña activa que se restaura en Edición recibe el foco en cuanto se carga su contenido, así que se puede escribir y moverse por el código (CodeMirror) sin hacer clic antes.
- **Una imagen PNG guardada desde draw.io sale nítida**: se exporta a escala 2 en lugar de 1, que se veía pixelada al ampliarla.
- **El texto que se edita en Excalidraw con zoom ya no se ve borroso**: el cuadro de edición se amplía con su tamaño de letra real y no con una escala, que WebKitGTK (con DMABUF) dibujaba a baja resolución y estiraba.
- **«Diff» comparaba con HEAD en vez de con el stage**: como `git diff`, ahora compara lo local con lo que está en stage, también en imágenes y diagramas (2 Paneles y Diferencias), y el lado «antes» se llama «Stage / Index». Si todos los cambios ya están en stage, ya no muestra en silencio el diff contra HEAD: avisa «Sin cambios fuera de stage» con un botón para abrir «Diff --staged».
- **`full_text_search` con `folder_path` devolvía `hits` vacío con `total_matches` > 0**: el filtro de carpeta se aplicaba después de recortar los mejores resultados de toda la bóveda; ahora se piden más candidatos y `total_matches` cuenta solo los de la carpeta o extensiones pedidas.
- **El contexto de los adjuntos pedía leer imágenes con `read_note`**: ahora, si se adjunta una imagen o un diagrama, indica `render_diagram` para verlo y `extract_note_diagrams` para la estructura de un draw.io.
- **El índice de texto completo se reconstruye una vez** (versión 4) para pasar los `.drawio` y `.archimate` ya indexados por los extractores nuevos.
- **Los archivos de código (xml, json, archimate, …) se pueden editar al pasar a Edición**: el editor quedaba en solo lectura aunque el botón dijera «Edición».
- **En modo Lectura, los archivos de código y diagramas se pueden buscar con Ctrl+F**: el editor ahora recibe el foco (cursor y selección) aunque no deje escribir.
- **Al guardar en draw.io, las otras pestañas con la imagen se recargan sin perder el zoom ni la posición**; tampoco al salir del editor.

## [1.0.0-beta.2] - 2026-11-17

### Añadido

- **Los archivos `.xml` se ven en el explorador de la bóveda**, se indexan en la búsqueda de texto completo y el agente los puede leer y buscar como el resto del código.

- **Copias de pestaña con modo propio y un solo editor por dibujo**: cada copia de una pestaña (`Duplicar pestaña`) tiene su propio botón Lectura/Edición, así una nota puede estar en edición en una pestaña y en lectura en la otra. Los dibujos (Excalidraw y draw.io) solo se editan en una pestaña a la vez, para no pisarse al guardar: pasar a edición en otra copia lleva a la pestaña que ya lo edita; al cerrarla o pasarla a lectura, cualquier copia puede editar. Las copias de una imagen de draw.io se actualizan con cada guardado del editor.
- **«Abrir en otra pestaña (solo lectura)»** en el menú contextual del explorador, cuando el archivo ya está abierto: abre una copia en lectura junto a la original.
- **Arrastrar a otra ventana también las pestañas de una versión, de un diff y las copias** (antes solo las de archivos).
- **La búsqueda de texto completo encuentra el texto de los diagramas de draw.io**: los `.drawio` y las imágenes PNG/SVG con un diagrama dentro se indexan con los textos de sus figuras y los nombres de sus páginas (también los diagramas comprimidos), como documentos de tipo `drawio`. Las imágenes sin diagrama siguen sin indexarse; para saberlo se leen solo las cabeceras del PNG.
- **Las imágenes de draw.io se marcan al indexar**: el índice de la bóveda guarda si cada PNG/SVG lleva un diagrama, así el botón Lectura/Edición aparece al abrirla sin leer el archivo otra vez. La caché pasa a `.synapse/cache-v2.bin`; la anterior (`cache.bin`) se importa sola la primera vez, sin perder las pestañas abiertas.
- **Imágenes de draw.io: botón Lectura/Edición y detección por contenido**: en una imagen de draw.io, el botón Lectura/Edición de la cabecera abre el editor de draw.io (Edición) o vuelve a la imagen (Lectura), como con las notas; el botón del visor ahora dice «Abrir en draw.io». Un PNG cuenta como de draw.io por su nombre (`.drawio.png`) o porque lleva el diagrama dentro (el bloque `mxfile` que draw.io guarda al exportar con «incluir una copia del diagrama»), aunque se llame `foto.png`. Lo mismo con los SVG: un `.drawio.svg`, o cualquier SVG con el diagrama en su atributo `content`, se edita en draw.io y se guarda como SVG (formato `xmlsvg`).
- **Revisar las bóvedas que pueden conversar con esta**: en Ajustes → Agentes de IA se ve qué bóvedas se aceptaron (✅) o rechazaron (🚫) y se puede quitar cada una; la próxima vez se vuelve a preguntar.
- **El registro de depuración incluye el canal entre Synapses**: además de los mensajes ACP, anota los pedidos `/peer` que salen y llegan (`peer-out`, `peer-reply`, `peer-in`, `peer-answer`), sin los `ping`.
- **Documentación de la comunicación entre Synapses** (`docs/comunicacion_entre_synapses.md`): quién le escribe a quién, el canal `/peer`, los permisos y los límites.
- **Mencionar conversaciones y bóvedas en el chat del agente**: `#` sugiere las otras conversaciones de esta ventana y `@` también las otras bóvedas de Synapse (además de los archivos). Cada conversación se muestra con su estado (🧠 trabajando, ⏸ libre, ⚠ esperando un permiso), título, agente y última actividad; el tooltip trae su última respuesta y la última pregunta. Las bóvedas se marcan 🟢 abierta, 🟡 ocupada o ⚪ apagada; `@Bóveda#` elige una de sus conversaciones o `#nueva`. Elegir una bóveda apagada la abre en otra ventana y el mensaje espera a que responda. Se inserta el número (`#2`, `@Bóveda#2`) y, al enviar, Synapse le explica al agente a quién apunta cada mención; en el mensaje enviado se ve como chip con su título en el tooltip.
- **Filtros en las menciones con `@`**: `@agent:` (o `@a:`) muestra solo agentes, es decir otras bóvedas de Synapse y las conversaciones de esta ventana, y `@file:` (o `@f:`) solo archivos; el prefijo solo filtra y al elegir se inserta la mención sin él. Sin prefijo, si lo escrito es el comienzo del nombre de una bóveda (`@main` → MAINFRAME), las bóvedas aparecen primero; cuando se ven archivos y bóvedas juntos, cada grupo indica su atajo.
- **Agente por defecto**: en Ajustes → Agentes de IA se elige un agente para todas las bóvedas. Se usa en una bóveda que todavía no usó ninguno (por ejemplo, cuando otra bóveda le escribe por primera vez) y en «＋ Nueva conversación»; si no hay ninguno elegido ni instalado, se usa el primero de la lista y se descarga solo. Antes, una bóveda nueva no podía atender conversaciones de otra.
- **Synapse pide atención cuando espera tu respuesta**: con un permiso del agente o una pregunta (por ejemplo, otra bóveda que quiere conversar), si la ventana no está al frente se marca como urgente en la barra de tareas y aparece una notificación del sistema. No le quita el foco a lo que estés haciendo.
- **Los mensajes de otro agente llegan cerrados**: «De Agente · #2 · 2,3 KB ▸»; al abrirlos se ven renderizados (no en crudo), con un botón para copiar el texto tal como llegó. Tu agente los recibe completos igual.
- **Registro de depuración de ACP**: en Ajustes → Agentes de IA, guarda sin filtrar cada mensaje que entra y sale de los agentes en `acp-debug.jsonl` de la configuración, para ver qué manda cada agente (por ejemplo, si incluye el resultado de sus herramientas).
- **Alt+2 abre y cierra el panel del agente**: nueva acción «Mostrar / ocultar el agente» (`toggle_agent`) en los atajos configurables, que también aparece en la paleta de comandos, en el panel de atajos y en el tooltip del botón de la barra lateral.
- **Copiar y pegar archivos con otras aplicaciones**: «Copiar» en el explorador deja los archivos en el portapapeles del sistema, como un explorador de archivos, así se pegan en otra instancia de Synapse, en el gestor de archivos o en otra aplicación; «Pegar» trae lo copiado fuera (otra instancia, el gestor de archivos) a la carpeta elegida («Pegar (N archivos de fuera)»).
- **Arrastrar archivos del explorador a otras aplicaciones**: soltarlos en Telegram, el gestor de archivos u otra aplicación los adjunta o copia, como desde un explorador de archivos; dentro del árbol arrastrar sigue moviendo.
- **El Historial de Git de un archivo dibuja sus ramas**: una columna a la izquierda muestra el grafo (como `git log --graph`), con las fusiones que unen ramas que también tocaron el archivo. Cada commit se marca «nuevo» (creó el archivo: punto hueco y sin «Ver cambios», porque no hay versión anterior en su rama), «borrado», «renombrado», «fusión» u «otra rama» (no es la versión anterior del commit de arriba).
- **El agente entiende el historial de Git de un archivo**: herramientas nuevas del MCP `git_file_history` (commits del archivo, siguiendo renombres), `git_diff_revisions` (qué cambió entre dos commits, o contra la versión local, con líneas agregadas y quitadas) y `git_read_revision` (el archivo como era en un commit); `render_diagram` acepta `revision` para ver un diagrama o una imagen (p. ej. un `.drawio.png`) tal como era. En el Historial de Git, con dos commits elegidos (o uno solo, para preguntar por ese commit), «Preguntar al agente qué cambió» abre el panel con el archivo adjunto y la pregunta escrita.
- **Elegir commits en el Historial de Git**: clic elige un commit, `Ctrl`+clic suma o quita otros y `Shift`+clic elige el rango desde el último clic. Con el clic derecho: «Diff entre estas 2 versiones» (con exactamente dos elegidas) y copiar sus hashes, completos o cortos, separados por coma.
- **Duplicar pestaña**: en el menú de cada pestaña, «Duplicar pestaña» abre otra pestaña con lo mismo, junto a la original y en el mismo panel; cada copia tiene su propia vista (modo del diff, zoom, desplazamiento).
- **El visor de diff recuerda las vistas ya calculadas**: «2 Paneles», «Diferencias» (píxel a píxel) y texto se calculan la primera vez que los eliges y quedan en memoria hasta cerrar la pestaña, así cambiar entre ellos es inmediato. Las vistas ocultas conservan su tamaño y sus imágenes ya dibujadas, así «2 Paneles» no se ve cargar de nuevo al volver. Cada pestaña de diff tiene su propio modo: con «Duplicar pestaña» puedes ver «Diferencias» en una pestaña y «2 Paneles» en la otra; el último modo elegido es el inicial de los diffs nuevos.
- **`Shift+Escape` cierra el último panel lateral que abriste**: el explorador (u otro panel de la barra izquierda) o el del agente, el que hayas abierto más recientemente; si lo vuelves a pulsar, cierra el anterior.
- **Mostrar el explorador con el teclado le da el foco**: `Alt+1` / `Ctrl+B` (además de «ubicar», `Ctrl+Alt+1`) dejan el foco en el árbol, sobre el archivo abierto o el primero, para moverse con las flechas y abrir con Enter.
- **Explorador fijado o suelto**: una chincheta en la cabecera del explorador. Fijado (como siempre) ocupa su espacio al lado del editor; suelto, el editor usa todo el ancho y el explorador flota encima, pegado a la barra lateral, cuando lo abres desde ella. Se oculta al abrir un archivo, con Esc o al hacer clic fuera. La elección se recuerda.
- **Diff entre dos versiones de un archivo**: en el Historial de Git, «Diff entre versiones» cambia las acciones de cada commit por casillas; eliges exactamente dos y «Comparar» abre sus diferencias (de la más antigua a la más nueva) en los visores de siempre: texto, imágenes y diagramas. Al desactivarlo vuelven las acciones.
- **Los diálogos de Git se mueven**: el historial (log), el de commit y el de ramas se mueven arrastrando su cabecera, sin salir de la ventana; recuerdan la posición mientras la app está abierta y un doble clic en la cabecera los vuelve a centrar.
- **Agente de IA en un panel lateral (ACP)**: nuevo botón «Agente de IA» en la barra lateral y en la cabecera del editor (junto a los controles de la ventana) que abre un panel acoplado a la derecha o flotante (movible y redimensionable). Conversa con agentes compatibles con ACP (Agent Client Protocol) sobre la bóveda abierta: respuestas en streaming, herramientas que usa el agente y peticiones de permiso con botones para aprobar o rechazar. Los agentes verificados son **Claude**, que se descarga del registro público de ACP (el mismo que usa Zed) y se verifica con su sha256, y **Google Antigravity**, que usa el CLI `agy` ya instalado; también se pueden registrar agentes propios (comando, argumentos y variables de entorno) en Ajustes → Agentes de IA.
- **El agente trabaja con tu bóveda**: cada sesión recibe el MCP de la ventana de Synapse abierta como `synapse_instance` (notas, búsqueda, enlaces, Git…), que se enciende solo si estaba apagado; los agentes que no hablan MCP por HTTP llegan a la misma instancia a través de un puente (`synapsers --mcp-proxy`). Al empezar se le pide al agente que use ese MCP y que cree y edite las notas con él en lugar de escribir los archivos. En las compilaciones con la feature `develop` hay además un **modo feedback** (Ajustes → Agentes de IA): el agente anota en `~/.config/synapsers/feedback.md` lo que le falte al MCP, con el agente, el modelo y la conversación de la que salió. Además, `/feedback [indicaciones]` en el mensaje le pide al agente que revise la conversación y registre con `report_feedback` qué mejorar del MCP y del panel. En el mensaje puedes adjuntar notas con `@` (`@this` adjunta el archivo de la pestaña que estás viendo), arrastrarlas desde el explorador o mandar la selección del editor con `Ctrl+Alt+A`; la nota abierta se adjunta sola como etiqueta que puedes quitar. El panel muestra el plan del agente, los archivos que toca con su diff, los comandos que ofrece (con `/`), el uso de contexto y, si el agente los tiene, selectores de modo y de modelo (por ejemplo, los modelos Gemini y Claude de Antigravity).
- **Un cliente MCP externo puede hablar con los agentes del panel**: por ejemplo, Claude Code usando el MCP de Synapse. `send_to_conversation` funciona sin conversación propia (el mensaje llega marcado como «Cliente MCP externo») y, si era una pregunta, la respuesta se recoge con la herramienta nueva `read_replies`.
- **`/usage` en el panel del agente**: lo responde Synapse con un resumen de la conversación (agente, modelo, modo, contexto, costo, mensajes, herramientas usadas por tipo, compactados y duración), salvo que el agente tenga su propio `/usage`.
- **Conversaciones por proceso, por agente**: en Ajustes → Agentes de IA eliges cuántas conversaciones comparte un proceso de cada agente (por defecto 4; usa 1 para un agente que atiende una a la vez).
- **Ícono propio del pulpo**: un pulpo robot en SVG, que sigue el color del tema, marca las conversaciones y los mensajes que vienen de otra bóveda.
- **Los agentes conversan con los de otra bóveda abierta en otra ventana** (🐙): «pregúntale a mainframe…» llega a la otra ventana de Synapse por un canal local con token, y allí abre una conversación nueva y limpia con su agente (o sigue el hilo de la misma conversación de origen, o usa `mainframe#1` si se pide). La primera vez, la bóveda de destino pregunta si acepta conversaciones de la otra. La respuesta vuelve sola a quien preguntó. Las conversaciones abiertas por otra bóveda y sus mensajes llevan un pulpo. Si la bóveda de destino está entre tus recientes pero no abierta, Synapse la abre en otra ventana. En Ajustes → Agentes de IA eliges qué agente atiende esas conversaciones; en ellas, las lecturas del agente se permiten solas y lo demás se rechaza si nadie responde en 2 minutos. `render_diagram` también dibuja diagramas de otra bóveda.
- **Los agentes consultan otras bóvedas**: con `list_vaults` y el parámetro `vault` en las herramientas de lectura del MCP (`search_notes`, `read_note`…), el agente de una bóveda puede buscar y leer las notas de otra (por ejemplo, validar los diagramas contra la documentación de mainframe), esté o no abierta. Es solo lectura; la primera vez Synapse pregunta si esa bóveda puede consultar la otra y lo recuerda (`.synapse/vault-access.json`). La respuesta se marca con 📚 y el nombre de la bóveda.
- **Los agentes se hablan entre conversaciones**: cada conversación del panel tiene un número (#1, #2…). Si le pides a un agente «pregúntale a #2…» o «avísale a Gemini…», usa las herramientas nuevas del MCP `list_conversations` y `send_to_conversation`, aunque sean agentes distintos (Claude, Gemini…): Synapse entrega el mensaje, y si el destino está trabajando lo recibe al terminar su turno (✉ en su fila). Si era una pregunta, la respuesta vuelve sola a quien preguntó («esperando a #2» mientras tanto). Los mensajes de otro agente se ven marcados con su origen, y hay un límite de 3 idas y vueltas seguidas sin intervención del usuario.
- **Panel del agente más compacto**: modelo y modo pasan a un ▾ junto a la línea de contexto, en corto («Argon·L», y el modo solo si no es el de inicio; un modelo que Synapse no reconoce se muestra con su nombre completo), con los nombres completos en el tooltip y en el menú; la línea de contexto muestra solo el porcentaje (el detalle va en el tooltip). «MCP Synapse» y ⋯ pasan al título de la conversación, junto al 📌, y desaparece la barra de la conversación.
- **`/compact` en el panel del agente**: si el agente ofrece `/compact`, la línea de uso de contexto muestra «Compactar» (desde el 60 % o al pasar el mouse, y en ámbar desde el 80 %). Al compactar, el chat marca «— contexto compactado —» y la barra baja con el siguiente dato de uso. Al retomar con un agente que no sabe cargar sesiones, el resumen de reemplazo empieza en el último compactado. «Ver en crudo» y «Borrar conversación» pasan a un menú ⋯ en la barra de la conversación.
- **Cada bóveda recuerda su panel de agentes**: si el panel estaba abierto o cerrado, el último agente y las conversaciones abiertas (desplegadas y fijada). Al volver a esa bóveda se retoman con sus mensajes. Una bóveda que nunca usó agentes empieza con el panel cerrado y sin agente.
- **Varias conversaciones con agentes a la vez, en acordeón**: «Nueva conversación con…» abre otra arriba del panel sin cerrar las demás (del mismo agente o de otro); como mucho dos quedan desplegadas y, al desplegar una tercera, se pliega la que lleva más tiempo abierta. Cada fila muestra su estado (🧠 pensando, «permiso» si espera tu respuesta) y se marca si terminó mientras estaba plegada. 📌 fija una conversación al costado del panel, en su propia columna; ✕ la cierra y pasa al historial, desde donde se reabre. Mientras el agente piensa, el chat lo muestra con 🧠, y el razonamiento contraído es solo un 🧠 que se despliega al hacer clic. Las conversaciones con un mismo agente comparten su proceso (hasta 4 por proceso; con más se lanza otro), así abrir una nueva es casi inmediato; el proceso se cierra 30 s después de cerrar su última conversación. Las imágenes pegadas se ven ampliadas al hacer clic en su miniatura (Esc, clic fuera o ✕ para cerrar).
- **Diagramas como imagen por el MCP**: al leer un diagrama (Excalidraw, Mermaid, D2, PlantUML), `read_note` lo marca con `diagram.render_tool`, y la herramienta nueva `render_diagram` lo devuelve dibujado como PNG, igual que se ve en Synapse (lo dibuja la ventana abierta). Así el agente puede ver el diagrama y no solo su código.
- **Pegar imágenes en el mensaje al agente**: `Ctrl+V` con una imagen en el portapapeles (por ejemplo, una captura) la adjunta con su miniatura; si el agente no acepta imágenes, se le avisa que había una. Cada agente recuerda el último modelo elegido, así una conversación nueva empieza con él.
- **Historial de conversaciones con el agente**: cada conversación se guarda en la bóveda (`.synapse/agent-sessions/`) y se retoma desde el desplegable de historial del panel; cada conversación recuerda su agente y retomarla cambia a ese agente; al elegir un agente se retoma su última conversación (el panel empieza cerrado y sin agente elegido); si el agente sabe cargar sesiones (`session/load`) continúa con su propio contexto, y si no, recibe un resumen de lo hablado. Hay botones para empezar una conversación nueva, abrir la conversación en crudo (JSON) con la aplicación por defecto y borrar una conversación o todas. Debajo del mensaje se ve el uso de la ventana de contexto del modelo (tokens usados, total y porcentaje, y el costo si el agente lo informa). Las respuestas del agente se ven como markdown (los enlaces a notas de la bóveda las abren en Synapse) y cada herramienta se expande para ver su entrada y su salida. La cabecera del panel se alinea con la barra de pestañas y el panel acoplado sigue el tema Islas.
- **Historial de Git más ancho y redimensionable**: el diálogo se agranda arrastrando sus bordes o esquinas, como una ventana, y recuerda el tamaño para todos los archivos. Los dos botones de diff van uno sobre otro y el mensaje del commit ocupa hasta dos líneas. Los commits con tag muestran el nombre del tag como etiqueta junto al mensaje.
- **Diálogos de commit y de ramas más grandes y redimensionables**: abren más anchos y altos, se agrandan arrastrando sus bordes o esquinas y recuerdan su tamaño; la lista de archivos o de ramas usa el alto extra. Los tres diálogos de Git (commit, ramas e historial) comparten ahora el mismo marco, cabecera y pie.
- **Ver los cambios de un commit**: en el Historial de Git, «Ver cambios» abre el parche de ese commit para el archivo (contra su commit padre). Texto en Dividida, Unificada o Parche; imágenes, dibujos y diagramas con 2 Paneles y Diferencias.
- **Diff píxel por píxel de diagramas**: Mermaid, D2 y PlantUML tienen el modo **Diferencias** en el diff de Git, en el stage, en el parche de un commit y al comparar dos archivos. Las dos versiones se renderizan y se comparan en memoria, sin escribir nada en disco.
- **Comparar dos dibujos o diagramas**: «Comparar archivos» con dos Excalidraw o dos diagramas del mismo lenguaje ofrece 2 Paneles, Texto y Diferencias.
- **Comparar dos imágenes**: «Comparar archivos» con dos imágenes (o dos dibujos, diagramas o PDF del mismo tipo) muestra el modo visual: **2 Paneles** lado a lado y, para imágenes raster y dibujos de Excalidraw, **Diferencias** píxel por píxel. Si las imágenes tienen distinto tamaño, se comparan desde la esquina superior izquierda.
- **Menú de la pestaña completo**: además de abrir en nueva ventana, con la app por defecto y mostrar en carpeta, ahora permite **Renombrar archivo** (abre la edición en el breadcrumb de esa pestaña), **Copiar ruta relativa** y **Copiar ruta completa**, igual que el menú del breadcrumb.
- **Editor de draw.io embebido**: los `.drawio` se abren en el editor de draw.io dentro de una pestaña y al guardar se escribe el XML del diagrama. Los `.drawio.png` se siguen viendo como imagen y se editan con **Abrir con draw.io** en el menú contextual del explorador o con el botón **draw.io** del visor de imágenes; al guardar se reescribe el PNG con el diagrama incrustado, que sigue siendo editable. Respeta el tema claro u oscuro. Se carga desde `embed.diagrams.net` (el diagrama no sale del equipo: viaja por `postMessage`); la dirección se puede cambiar con `drawioUrl` en los ajustes para usar un draw.io propio sin red.
- **Los `.drawio.png` vuelven a abrirse como imagen**: el modo draw.io solo dura mientras la pestaña está abierta; abrir el archivo desde el explorador o la apertura rápida, o cerrar la pestaña, lo devuelve al visor de imágenes.

### Cambiado

- **Pruebas más rápidas**: las pruebas de integración de Rust van en un solo binario (`src-tauri/tests/integration/`; un grupo se corre con `cargo test --test integration <módulo>::`), los perfiles `dev` y `test` guardan menos información de depuración y se enlaza con `mold`. Recompilar las pruebas tras un cambio pasa de minutos a unos segundos. Las e2e usan el mismo binario de desarrollo (`target/debug`, Vite en el 5173) en lugar de una compilación aparte en `target/e2e`.
- **«Bóveda» en lugar de «Workspace»**: la pantalla de bienvenida, los recientes, los atajos y la paleta dicen «bóveda» (en inglés, «vault»), igual que el resto de la app.

### Corregido

- **La publicación de versiones vuelve a compilar en Linux**: el flujo de release instala `mold`, el enlazador que usa `src-tauri/.cargo/config.toml` (faltaba en el runner y fallaba con «cannot find ld»).
- **Hacer clic en el explorador sobre una imagen abierta en draw.io ya no cierra ni redibuja el editor**: solo activa su pestaña.
- **Una imagen abierta se refresca cuando el archivo cambia en disco**, también en sus copias de pestaña; antes el webview seguía mostrando la versión en caché.
- **Guardar en draw.io una imagen (PNG/SVG) ya no redibuja el editor**: el archivo que reescribe su propio guardado se tomaba como un cambio externo y el editor se volvía a montar. Ahora, mientras la imagen está en el editor, un cambio en disco no la recarga, y al salir del editor el visor muestra lo guardado.
- **Pruebas e2e más robustas (en curso, `syna-7qa9`)**: si el WebDriver se cae, las pruebas que quedan fallan al instante en vez de esperar su tiempo límite; el arranque espera a que el WebDriver anterior libere el puerto 4444 (causa probable de las caídas en cadena con ECONNREFUSED); varias pruebas cambian pausas fijas por esperas de la condición real.
- **Shift+F6 (cambiar todas las coincidencias) no hacía nada si la ventana no tenía el foco del sistema**: el atajo comprobaba `hasFocus`, que en WebKit depende de `document.hasFocus()`; ahora basta con que la tecla llegue al editor.
- **Pestañas de una versión o de un diff en una ventana nueva**: «Abrir en nueva ventana», «Mover a nueva ventana» y el botón de la cabecera ahora también funcionan con la pestaña de «Ver archivo» de un commit y con las de diff (antes solo con archivos); la ventana nueva muestra la misma versión.
- **Fondo por defecto de la pestaña vacía en blanco**: los fondos claro y oscuro venían en AVIF, que el WebView de algunos Linux (WebKitGTK sin soporte AVIF) no decodifica; por eso se veía en blanco al abrir la app y al quitar un fondo propio. Ahora van en WebP.
- **«Renombrar archivo» desde el menú de una pestaña**: la edición del nombre ya no se cierra sola. Al activarse la pestaña, su vista de lectura tomaba el foco, el campo del nombre lo perdía y la edición se confirmaba sin cambios; ahora la vista de lectura no le quita el foco a un campo de texto, y el menú no devuelve el foco a la pestaña al elegir renombrar.
- **Historial de Git de archivos renombrados**: «Ver archivo», «Ver cambios», «Diff con local» y el diff entre versiones de un commit anterior a un renombre o a mover el archivo de carpeta ya muestran su contenido, en vez de «El archivo no existía en la revisión». En esa revisión se busca el nombre que tenía entonces, igual que el historial sigue al archivo.
- **Zoom y arrastre de diagramas e imágenes**: en los diagramas (Mermaid, D2, PlantUML) arrastrar ya mueve el diagrama lo mismo que el mouse con cualquier zoom (antes, con zoom, se desfasaba), y la rueda hace zoom con el cursor como ancla. El zoom del diff de imágenes y del visor de Excalidraw también se ancla ahora en el cursor, como ya hacía el visor de imágenes.
- **`bun run check` sin errores ni advertencias**: el tipo de los píxeles en el diff de Excalidraw y el esquema de títulos, que tomaba a propósito solo el valor inicial.
- **Abrir una conversación mientras la bóveda restaura las suyas ya no la repliega** al terminar la restauración.
- **El agente ya no sigue en la bóveda anterior al cambiar de bóveda**: la conversación se cierra (queda guardada en su bóveda) y el panel vuelve a empezar sin agente elegido, con el historial de la bóveda nueva.
- **`Shift+F6` vuelve a seleccionar todas las coincidencias en el editor de código**: había dos copias de CodeMirror y el atajo quedaba registrado en una que el editor no leía; ahora se usa una sola versión.
- **La caché de la bóveda ya no se corrompe si la app se cierra mientras guarda**: `.synapse/cache.bin` se guarda con `object_store`, que escribe en un archivo temporal y lo renombra al terminar; si el guardado se interrumpe, queda la versión anterior.
- **Volver a lectura conserva la posición**: al pasar de edición a lectura en una nota Markdown, la lectura se abre en la parte del documento que se estaba editando en lugar de volver al inicio.
- **Diff visual en stage**: comparaba el disco contra el stage; ahora compara el stage contra HEAD, como `git diff --staged`.
- **Fechas del Historial de Git**: mostraban «Invalid Date».
- **PlantUML en el diff lado a lado**: el panel de la versión de Git lo dibujaba como Mermaid.
- **«Diff» con dos archivos seleccionados**: en una bóveda con Git, el «Diff» del menú del explorador hacía el diff de Git de un solo archivo (y solía mostrar «Sin diferencias»). Ahora, con dos archivos seleccionados, se llama «Diff entre los 2 archivos» y los compara entre sí.

## [1.0.0-beta.1] - 2026-11-12

### Añadido

- **Tema integrado «Papiro»**: papel kraft envejecido con fibras y manchas, la nota como una hoja de bordes rotos y tostados, y tinta sepia con títulos en serif. En oscuro, cuero curtido con letras de pergamino. También en «Papiro Islas».
- **Carpetas de temas globales**: los `.css` de `~/.config/synapsers/themes` (que se lee siempre) y de las carpetas extra que agregues en Configuración → Apariencia aparecen como temas en todas las bóvedas, sin recompilar. Las carpetas extra se guardan en `appearance.theme_dirs` del nuevo `~/.config/synapsers/config.jsonc`; se leen al iniciar y con el botón **Reescanear**. Admiten `@nombre`, `@capa` y `@base` como los integrados.
- **Tema integrado «Neón CRT» y temas integrados automáticos**: «Neón CRT» es Neón con líneas de barrido tipo monitor viejo en la lectura (en Linux pueden dar lag al desplazarse, por eso «Neón» ya no las trae). Para sumar un tema integrado basta con dejar su `.css` en `src/entities/appearance/themes/`, con `@nombre`, `@capa` o `@base` en su comentario inicial.
- **Actualizador automático (Auto-updater)**:
  - Integración del plugin `tauri-plugin-updater` para detectar y aplicar actualizaciones firmadas desde GitHub Releases.
  - Firma digital criptográfica de los paquetes y generación automática del manifiesto `latest.json` en los flujos de CI/CD para Linux (`.tar.gz`) y Windows (`.zip`).

### Cambiado

- **Una sola carpeta de configuración global, `~/.config/synapsers`**: el registro de puertos MCP (`mcp-ports.json`) ya no se escribe también en `~/.config/synapse`. Esa carpeta ya no se usa y se puede borrar.
- **Caché de navegación a `postcard`**:
  - Reemplazo de la biblioteca desmantenida `bincode` por `postcard` (`v1.1` con `use-std`) para la serialización binaria en disco de la caché del motor de navegación (`DashMap<NoteId, NoteMeta>`), reduciendo huella y garantizando compatibilidad moderna de tipos.
- **Optimización de compilación**:
  - Compilación con *ThinLTO* en modo release y uso del enlazador `lld` en Linux para acelerar los tiempos de enlazado y empaquetado en CI.

## [1.0.0-alpha.5] - 2026-11-11

### Añadido

- **Página web de presentación** (`docs/index.html`): landing estática basada en `docs/about.md`, con capturas ampliables (visor con zoom a tamaño real y navegación con flechas), comparativa, temas, arquitectura y la mascota (pulpo kawaii) fija en la esquina inferior derecha.

- **Temas integrados «Rosa» e «Islas»**: «Rosa» en tonos rosa suaves; «Islas» muestra los paneles como islas flotantes con esquinas redondeadas sobre un marco más oscuro, y se combina con los demás en «Papel Islas», «Neón Islas» y «Rosa Islas».
- **Temas integrados «Papel» y «Neón»**: vienen con la app y aparecen en Configuración → Apariencia → Tema personalizado sin copiar archivos. Un archivo `.synapse/themes/` con el mismo nombre los reemplaza. «Neón» es cyberpunk con fondo de rejilla, paneles translúcidos con desenfoque, títulos con brillo magenta, cian y amarillo, íconos de carpeta propios y desplegables dibujados a mano. Tiene versión oscura y una clara «amanecer».

- **Temas: todo el color en variables, zonas públicas y tema de ejemplo**: los colores de la interfaz salen de variables CSS (nuevas: `--success`, `--warning`, `--danger`, `--purple`, `--on-accent`, `--overlay-bg`…), así un tema cambia toda la app. Las zonas principales tienen clases estables `synapse-*` (barra lateral, filas del explorador, pestañas, editor, lectura, barra de estado, diálogos). Guía en `docs/temas.md` y tema de ejemplo «Papel». Algunos tonos cambian levemente al seguir las variables del tema, sobre todo en la pantalla de bienvenida.

- **Temas y fragmentos CSS por bóveda**: en Configuración → Apariencia se elige un tema de `.synapse/themes/` y se activan fragmentos de `.synapse/snippets/` (archivos `.css`). Se aplican sin reiniciar y cada bóveda guarda los suyos en `.synapse/appearance.json`. Los temas pueden cambiar los títulos por nivel en lectura y edición (`--h1-font`, `--h1-size`, `--h1-weight`, `--h1-color`… hasta `h6`), la fuente del texto de las notas (`--font-text`) y el ícono de carpeta (`--icon-folder`, `--icon-folder-open`).

- **Diff de Excalidraw por píxeles**: el diff visual de un dibujo tiene el modo «Diferencias», como las imágenes. Las dos versiones se exportan a imagen con el mismo marco y se resaltan en magenta los píxeles que cambiaron.

- Fondo por defecto en la pestaña vacía: una imagen para el tema claro y otra para el oscuro, incluidas en la app. Se usan mientras no se elija un fondo propio.
- El fondo propio de la pestaña vacía ahora es de cada bóveda, no general. El que ya estaba elegido pasa a la bóveda abierta.

- **Explorador: expandir y contraer todo**: dos botones en la cabecera del explorador. Con el filtro de Git activo solo afectan a las carpetas del filtro; al quitarlo, las demás siguen como estaban.
- **Explorador: archivos borrados**: un archivo borrado que Git todavía tiene sigue apareciendo en el árbol, en rojo y tachado; al hacer clic se abre su diff.
- **Filtro de Git completo**: con el filtro de Git el explorador muestra todo lo que entraría en un commit (sin seguimiento, modificado, añadido o borrado), aunque sea un tipo de archivo que normalmente no se ve en el explorador.
- **Índice de la nota**: al hacer clic en el título de una nota Markdown (o en el botón de índice a su lado) se abre un índice con todos sus títulos y subtítulos, contraído por defecto (solo se ven los de primer nivel), con botones para expandir y contraer todo; al elegir uno, la nota va a ese título. El índice abre desplegado hasta el título donde estás, que aparece resaltado, y tiene arriba una búsqueda rápida de títulos (Enter va al primero que coincide).
- **Ubicación en la nota**: junto al título de una nota Markdown, entre paréntesis, se ve la ruta de títulos de la parte que estás viendo (p. ej. `(Inicio › Parte A › Detalle)`), que se actualiza al desplazarte; al hacer clic en un tramo se va a ese título.
- **De lectura a edición sin perder el sitio**: al pasar del modo lectura a un modo de edición, el cursor queda en la primera línea que se estaba viendo en lectura.
- **Breadcrumb: copiar ruta completa**: el menú contextual del archivo en el breadcrumb ofrece «Copiar ruta relativa» y «Copiar ruta completa».

### Corregido

- **Cabeceras alineadas**: la línea inferior de la cabecera del explorador ahora coincide con la de la barra de pestañas.
- **Bloques de código en lectura**: dentro de un bloque de código aparecía una línea horizontal bajo cada línea de texto (heredaba el borde del código en línea).
- **Commit con archivos borrados**: hacer commit de un archivo borrado cuyo borrado ya estaba en stage (o de una carpeta borrada entera) fallaba con «git add falló: … no concordó con ningún archivo».

## [1.0.0-alpha.4] - 2026-11-09

### Añadido

- **Panel de configuración**: el botón de engranaje del ribbon abre un panel para cambiar el tema, el idioma, el tamaño del texto, el fondo de la pestaña vacía, el modo Vim, las propiedades de la nota, los visores de Mermaid y Excalidraw, el servidor de PlantUML y todos los atajos de teclado. Los atajos se guardan en `shortcuts.jsonc` sin perder sus comentarios.
- **Pestaña vacía con buscador y fondo**: la pestaña vacía muestra arriba el buscador de `Ctrl+O`, con el foco puesto para empezar a escribir, y permite elegir una imagen de fondo con «Cambiar fondo».
- **Abreviatura `hs:` para `headings:` en la búsqueda de contenido**, y la guía de búsqueda ahora explica este filtro.
- **`hs:` y `headings:` también funcionan en Quick Open (`Ctrl+O`)**: buscan en los encabezados sin tener que escribir `?` delante.
- **Al filtrar por `headings:` o `hs:`, el resultado muestra el encabezado encontrado con formato de título** y, en la línea siguiente, el texto que le sigue. El índice se reconstruye solo la primera vez que se abre la bóveda tras actualizar.
- **MCP: `write_note` informa de lo que hizo**: la respuesta indica con `created` si la nota se creó o se sobrescribió, y añade `warning` si su carpeta no existía.
- **MCP: `write_note` ya no pisa notas sin permiso** (cambio incompatible): para reemplazar una nota existente hay que enviar `expected_hash` u `overwrite: true`; si no, falla y explica cómo seguir. También falla si se envía `expected_hash` y la nota no existe.
- **Tamaño del texto en el editor y la previsualización**: `Ctrl+rueda`, `Ctrl++` y `Ctrl+-` lo cambian, y `Ctrl+0` lo restablece. Los atajos se configuran en `shortcuts.jsonc` (`zoom_in`, `zoom_out`, `zoom_reset`) y admiten varias combinaciones separadas por comas.
- **Comparar dos archivos desde VaultExplorer**: al seleccionar exactamente dos archivos, el menú contextual ofrece «Comparar archivos» y abre un diff entre ellos, sin necesidad de git. Solo admite archivos de texto.
- **La búsqueda en el contenido indexa los títulos y subtítulos de las notas Markdown** y les da más peso que al resto del texto: una nota con el término en un encabezado sale antes que una que solo lo menciona en un párrafo. También se puede filtrar con `headings:término`. El índice se reconstruye solo la primera vez que se abre la bóveda tras actualizar.
- **Filtro `kind:` en Quick Open (`Ctrl+O`)**: `kind:pdf informe` busca solo entre los PDF; `kind:markdown`, `word`, `excel`, `code`, `image`, `mermaid` y `excalidraw` funcionan igual, basta con escribir el comienzo (`kind:pd`). Solo `kind:pdf` lista todos los PDF.
- **Los enlaces `[[nota#Título]]` llevan al encabezado**, como en Obsidian: al pulsarlos (en el editor y en modo lectura) se abre la nota y se va a ese título o subtítulo, no a la primera vez que aparece el texto. Con `[[nota#Título#Subtítulo]]` vale el último.
- **`![[nota#Título]]` incrusta esa sección en el modo lectura**: muestra el título y todo lo que le pertenece, hasta el siguiente encabezado del mismo nivel o superior (los subtítulos de nivel inferior van incluidos). También se expanden los embeds dentro de lo incrustado, hasta 3 niveles y sin repetir ciclos. Si la nota o el título no existen, queda como enlace a la nota (antes se mostraba una imagen rota). En el editor sigue siendo un enlace al encabezado. Las imágenes y demás recursos con `![[...]]` no cambian.
- **Atajo `Shift+F6` en VaultExplorer para renombrar**: permite renombrar el archivo o carpeta seleccionado cuando el explorador de archivos tiene el foco.

### Corregido

- **La búsqueda en el contenido ya no pierde resultados cuando la última palabra es completa** y su raíz es más corta, como «para» en `hs:Prompts para`.
- **`Tab` en los editores de código y Markdown inserta 4 espacios** en vez de sacar el foco del editor; con texto seleccionado sangra las líneas, y `Shift+Tab` quita la sangría. Con Vim, solo en modo inserción.
- **La búsqueda en modo lectura ya no congela la app al escribir** en notas grandes: se busca una sola vez al dejar de escribir, y limpiar los resaltados es mucho más rápido.
- **`Ctrl+Shift+F` abre la búsqueda en el contenido** también con una nota abierta; antes abría la búsqueda dentro de la nota.
- **`Ctrl+R` en un editor abre buscar y reemplazar** con el foco en el campo de reemplazo. Con Vim sigue siendo rehacer, salvo con el foco en el panel de búsqueda. Fuera del editor sigue recargando.
- **Las propiedades de la nota reconocen listas en línea** como `tags: [uno, dos, tres]`.
- **`Shift+F6` en CodeMirror solo se activa si tiene el foco**, evitando capturar la combinación o seleccionar ocurrencias cuando el editor no está enfocado.

- **Los mensajes de error del editor de código ya no quedan cortados o tapados por la barra superior**, por ejemplo en un error de la primera línea de un diagrama.
- **`Ctrl+R` en un editor con Vim rehace** (como en Vim) en vez de recargar el archivo; fuera de Vim sigue recargando.
- **El render de D2 y Mermaid ya no se atrasa al editar**: antes cada tecla lanzaba un render y, con un layout lento como TALA (~3 s por render), se encolaban y el diagrama tardaba decenas de segundos en ponerse al día. Ahora se espera a que se deje de escribir (más tiempo si el render es lento) y nunca hay más de uno pendiente.
- **Render de D2 y Mermaid al editar**: ya no se lanza un render por tecla (con TALA se acumulaban y el diagrama llegaba muy tarde); se espera una pausa al escribir y nunca hay más de un render en cola. `Ctrl+Enter` fuerza el redibujado y la barra de estado muestra el render en curso y lo que tardó.
- **El editor de diagramas (D2/Mermaid) recupera el foco y el cursor al volver a su pestaña**, y una recarga del archivo desde disco ya no recrea el editor ni manda el cursor al inicio: solo se aplica el tramo que cambió.
- **Búsqueda de Vim (`/`) y cursor invisibles en el tema oscuro** del editor de código.
- **Una segunda instancia de la app ya no reescribe el puerto MCP de la bóveda** ni la marca como detenida.
- **Título editable** de la nota: se guardaba sobre una propiedad no reactiva.
- **Quick Open**: separadores coherentes y orden estable en los empates.
- **Persistencia del estado de MCP al cerrar y abrir workspaces**:
  - Se desacopló el ciclo de vida del runtime y socket TCP efímero (`running`) de la preferencia configurada por el usuario en la bóveda (`enabled`).
  - Al cerrar la aplicación, ventana o alternar entre bóvedas, el servidor MCP detiene su proceso/socket sin sobrescribir ni apagar `enabled: false` en `.synapse/mcp.json`.
  - Al abrir un workspace donde MCP estaba activo, el servidor se reactiva automáticamente.
  - Se introdujo el trait de dominio `McpConfigRepository` y su implementación de infraestructura `FileMcpConfigRepository` siguiendo la Arquitectura Cebolla.

### Cambiado

- **`F2` alterna entre edición y lectura** (antes `Ctrl+E`).
- **En el editor, `Ctrl+E` y `Ctrl+Y` desplazan la pantalla una línea y `Ctrl+D` duplica la línea**, también con Vim (en Vim, `Ctrl+D` deja de ser media página).
- **Rendimiento**:
  - Arranque más rápido: las pestañas se montan al activarse, el explorador ya no usa un árbol de respaldo y los comandos de lectura salen del hilo principal (bloqueo de JS al arrancar de ~1250 a ~750 ms).
  - El backend devuelve al sistema la memoria liberada tras abrir la bóveda (de ~800 a ~400 MB).
  - Explorador virtualizado: solo se pintan las filas visibles.
  - Imágenes servidas con el protocolo de assets de Tauri en vez de base64 por IPC.
  - Una sola lista de la bóveda en el frontend, sin avalancha de IPC al abrir.
  - Búsqueda de texto completo: consultar las versiones indexadas ya no carga cada documento (19 ms → 3 ms con 3 740 documentos).

### Añadido

- **Soporte para PlantUML (`.puml`, `.plantuml`)**: editor con resaltado y autocompletado, vista previa, errores marcados en su línea, `Ctrl+Enter` para redibujar, creación desde el explorador y la paleta, y exportación a PNG/SVG. El render lo hace un servidor PlantUML (por defecto `plantuml.com`, por lo que el código del diagrama sale de tu máquina); se cambia con el comando «PlantUML: cambiar servidor». El SVG recibido se limpia antes de mostrarlo.
- **`Ctrl+Enter` en el editor de D2 y Mermaid fuerza el redibujado** del diagrama.
- **Estado del render de diagramas en la barra de estado**: «Renderizando D2… 1,2 s» mientras dura y el tiempo que tardó al terminar.
- **Autocompletado con las palabras del documento** en todos los editores (markdown, D2 y Mermaid): los nombres que ya usas (variables, nodos…) se sugieren aunque el lenguaje no los conozca, detrás de las sugerencias del lenguaje.
- **Exportar diagramas a PNG y SVG** desde el menú contextual del explorador, para Excalidraw, Mermaid y D2. El archivo se guarda junto al diagrama con el tema actual.
- **`Shift+F6` cambia todas las coincidencias de una palabra** (como renombrar en JetBrains, pero textual): selecciona las coincidencias exactas de palabra completa y lo que se escribe las cambia todas. Con Vim, se cambian con `c`.
- **Plegado entre llaves en D2 y Mermaid**, desde el margen del editor de código.
- **Atajos de plegado de Vim** con el comportamiento de Vim: `zc`, `zo`, `za`, `zC`, `zO`, `zM` y `zR`. `zc` pliega el bloque que contiene al cursor y `zM` pliega todos los niveles.
- **Copiar la ruta de la bóveda** desde el menú contextual del explorador; copiar ruta se desactiva si no hay archivos seleccionados.
- **Compatibilidad con la API de Obsidian para plugins**:
  - La fachada responde de forma síncrona como en Obsidian: `getAbstractFileByPath`, `getActiveFile` y `metadataCache.getFileCache`.
  - `SynapseVault` mantiene un índice ruta → metadatos (sin contenido) que se actualiza con las mutaciones locales y los eventos del vigilante. Los plugins se cargan cuando el índice está listo.
  - Lo específico de Excalidraw pasa a `integrations/excalidraw`.

- **Filtro "Solo compatibles" en el catálogo de plugins comunitarios**:
  - Activo por defecto: muestra solo los plugins comprobados manualmente, listados en `verified-plugins.jsonc` (`verified_plugins`).
  - Nuevo archivo `verified-plugins.jsonc` en la raíz del proyecto, importado en build con `?raw` y parseado en `community-plugins.svelte.ts`.
  - Se puede desactivar con la casilla junto al selector de orden.

## [1.0.0-alpha.3] - 2026-10-30

### Añadido

- **Deshacer y rehacer en el editor**:
  - `Ctrl+Z` para deshacer y `Ctrl+Shift+Z` o `Ctrl+Y` para rehacer.
  - Atajos de vim: `u` para deshacer, `Ctrl+R` para rehacer y `U` para revertir la última línea editada.
  - Módulo reutilizable `shared/lib/editor-history`.

- **Comentar/descomentar líneas con `Ctrl+/`** (`Cmd+/` en macOS):
  - Nuevo módulo `shared/lib/editor-comment`, disponible en el editor de Markdown y en el editor de código de diagramas.
  - Usa los tokens de comentario del lenguaje: `#` para D2, `%%` para Mermaid y `<!-- -->` para Markdown.

- **Deshacer de línea (`U`) en el editor**:
  - Nuevo módulo `shared/lib/editor-history` que replica el `U` de vim: revierte los cambios de la última línea editada, y vuelve a aplicarlos si se repite.
  - Disponible en el editor de Markdown y en el editor de código de diagramas.

- **Versión de compilación derivada de Git**:
  - Nuevo módulo `version.rs` que compone la versión SemVer a partir de `git describe` (por ejemplo `1.0.0-alpha.2+4.g09ab330`), y añade la marca de tiempo si el árbol está sucio.
  - `build.rs` emite la información con `vergen`. El comando `get_app_version` y la opción `--version` usan esta versión en lugar de `package_info()`.

- **Guardado de cambios pendientes antes de operaciones sobre archivos**:
  - Nuevo puente `shared/lib/pending-writes`: el explorador, el breadcrumb y el menú de Git persisten el buffer del editor antes de renombrar o hacer `git add`.
  - Con esto, renombrar o añadir al stage un archivo abierto ya no opera sobre una versión desactualizada en disco.

- **Panel de código redimensionable en diagramas D2 y Mermaid**:
  - Nueva barra `SplitResizer` entre el código y el visor del diagrama.
  - La proporción elegida se recuerda por archivo durante la sesión, en memoria del backend (comandos `get_diagram_split_ratio` y `set_diagram_split_ratio`). No se persiste a disco.

- **Renombrado de archivos versionados en Git**:
  - Si el archivo está en el índice, el renombrado se prepara en el índice (`git add` de la ruta antigua y la nueva), de modo que Git lo detecta como renombrado.
  - Nuevo método `GitService::is_tracked` para comprobar si una ruta tiene archivos versionados.

- **Mover archivos y carpetas en el explorador con `Ctrl+X` y `Ctrl+V`**:
  - Nuevo comando `move_vault_items`: mueve elementos a la carpeta destino, sin sobrescribir ni mover una carpeta dentro de sí misma.
  - Las pestañas abiertas siguen a sus archivos tras el movimiento, y los archivos versionados quedan preparados en el índice de Git.
  - Si un elemento está bloqueado y tiene cambios sin guardar, se pide confirmación antes de moverlo.

- **Panel de atajos de teclado** (`F1` o el comando "Mostrar atajos de teclado"):
  - Lista los atajos configurables de `shortcuts.jsonc` y los fijos del editor, con búsqueda.
  - Nuevo componente `Kbd` para mostrar combinaciones, usado también en la paleta de comandos.

- **Confirmación al renombrar archivos bloqueados**: si el archivo bloqueado tiene cambios sin guardar, el diálogo de cambios pendientes ofrece guardar y continuar, o cancelar.

### Cambiado

- **Ancho mínimo del panel de código de diagramas**: baja de 280 a 160 px.

- **Atajos en la barra lateral**: los rótulos del Ribbon muestran solo la combinación principal, mediante `primaryShortcutText`.

- **Autoguardado al quitar el bloqueo**: un archivo con cambios pendientes al desbloquearse se vuelve a guardar automáticamente.

- **Metadatos y edición de Rust**: `Cargo.toml` pasa a `edition = "2024"` y `rust-version = "1.93.0"`, con autor, descripción y repositorio del proyecto. La línea `version` queda comentada, así que la versión ya no sale de `Cargo.toml`.

- **Rendimiento del editor de notas**:
  - Autoguardado a los 4 s de dejar de escribir. También se guarda al perder el foco, al cambiar de pestaña o al pasar a solo lectura, respetando los archivos bloqueados.
  - Ya no se reconcilia toda la bóveda en cada guardado.
  - Refresco de `git status` agrupado con un debounce de 5 s.
  - Debounce del indexador de texto completo (FTS) de 5 s.
  - Nuevas claves en `config.jsonc`: `editor.git_status_refresh_debounce_ms`, `indexing.debounce_ms` y `drawing_change_debounce_ms` (2500 ms por defecto).

- **Excalidraw**:
  - Detección de cambios por firma de elementos, de modo que el scroll, el zoom y la selección no serializan la escena.
  - Debounce de 2500 ms y propagación inmediata de los cambios pendientes al perder el foco.
  - Nueva clave `drawing_max_wait_ms` (15000 ms): tope de edición continua sin pausas, pasado el cual el dibujo se propaga y se guarda igualmente.
  - El indexador omite el reindexado si el texto del dibujo no ha cambiado.

- **Diagramas D2 y Mermaid**: los bloques ```` ```d2 ```` y ```` ```mermaid ```` ya no se re-renderizan al editar texto cercano.

### Corregido

- **`Ctrl+R` ya no recarga la ventana** de la aplicación.
- **Tamaño del SVG de D2**: el SVG recibe `width` y `height` a partir de su `viewBox`, evitando que colapse en las tarjetas de lectura.
- **Renombrar o hacer `git add` con cambios sin guardar**: el buffer del editor se persiste antes de la operación, para no trabajar con el archivo desactualizado en disco.
- **Pestañas tras renombrar una carpeta o un archivo**: se reasignan todas las referencias abiertas (pestañas principales y divididas, selección, historial, cierre, bloqueos y rutas recientes), incluidas las notas dentro de una carpeta renombrada.
- **Guardado tardío recreaba el archivo antiguo**: un guardado pendiente o un visor que cerraba después de un renombrado volvía a crear la ruta anterior. Ya no ocurre.
- **Dibujos sin guardar al perder el foco**: al ocultarse la ventana o perder el foco, se guardan también los dibujos con cambios aún sin propagar.
- **Renombrar o mover un archivo bloqueado sin confirmación**: ahora se pide confirmación si el archivo está protegido.

## [1.0.0-alpha.2] - 2026-10-30

### Añadido

- **Componente reutilizable `GitStatusBadge` y soporte de tokens CSS para Git**:
  - Creación del componente `GitStatusBadge.svelte` en la entidad `@entities/git` para centralizar la presentación visual de insignias de estado Git (`M` modificado/staged, `A` añadido, `?` untracked, `S` stashed).
  - Integración del componente `GitStatusBadge` tanto en `VaultExplorer` como en `BreadCrumb`, unificando su diseño e interacción en toda la interfaz.
  - Adición de variables semánticas CSS para estados Git en el tema claro y oscuro (`--git-modified`, `--git-staged`, `--git-untracked`, `--git-stashed` y sus respectivas variantes `-bg` y `-border`).

- **Pegado inteligente de imágenes en notas Markdown**:
  - Intercepción de eventos de pegado (`paste`) en CodeMirror cuando el portapapeles contiene una imagen (archivo o datos binarios).
  - Detección automática del formato de imagen correspondiente (`webp`, `png`, `jpg`, `svg`, `gif`, etc., con fallback por defecto a `webp`).
  - Guardado físico de la imagen en la subcarpeta `attachments/` relativa a la carpeta de la nota actual (o en la raíz si la nota reside en ella), con timestamp `image_YYYYMMDD_HHmmss.[ext]` y resolución de colisiones.
  - Inserción automática del enlace transcluido `![[image_...]]` en la posición del cursor de la nota, renderizándose visualmente de forma inmediata gracias al soporte de imágenes del editor.

- **Versión dinámica de la aplicación y visualización adaptativa**:
  - Exposición del comando IPC Tauri `get_app_version` en el backend para obtener la versión de forma fidedigna desde `package_info().version`.
  - Conexión de la versión dinámica en la pantalla de bienvenida (`WelcomeScreen`) reemplazando valores codificados en duro.
  - Indicador de versión integrado en la barra de estado inferior (`StatusBar`), con visibilidad adaptativa mediante estilos CSS que la ocultan automáticamente si no hay espacio horizontal suficiente.

- **Reorganización de herramientas en la barra lateral Ribbon**:
  - Reordenamiento de las herramientas situando directamente debajo de las carpetas el acceso a la búsqueda de archivos por nombre (`quick-open`) y la búsqueda full-text en el contenido de las notas (`search`).

- **Puntaje compuesto en Quick Open (`Ctrl+O`)**:
  - Nuevo módulo `ranking.rs` (`QuickOpenScorer`) construido sobre `nucleo_matcher`, que puntúa por campo (nombre de archivo, título, tags y ruta) con pesos: nombre > título > tags > ruta.
  - Bonus de recencia acotado (último día, última semana o alguna vez abierta) y bonus por coincidencia exacta del nombre, de modo que una coincidencia dispersa reciente no supera a una coincidencia limpia.
  - Pruebas de integración en `navigation_tests.rs` para el nuevo orden de resultados.

- **Miniaturas de videos de YouTube en notas Markdown**:
  - Los links a YouTube (`youtube.com`, `youtu.be`, `youtube-nocookie.com`, incluidos `shorts`, `embed` y `live`) muestran una miniatura clicable en el modo de lectura y en el editor en vivo.
  - Nuevo módulo `youtube.ts` con extracción del ID de video y pruebas unitarias en `youtube.test.ts`.
  - La miniatura se oculta automáticamente si no se puede cargar.

- **Sincronización del título en el frontmatter**:
  - Nueva función `sync_frontmatter_title` que actualiza o crea el campo `title` en el frontmatter YAML de una nota Markdown, conservando el resto de campos y el cuerpo.

- **Extensión `.txt` tratada como código**:
  - Los archivos `.txt` se añaden a la lista de extensiones de código en `config.jsonc` y se muestran con el icono de código en lugar del de Markdown.

### Cambiado

- **Guardado automático con temporizador por archivo**:
  - Cada nota tiene su propio temporizador de guardado, por lo que editar una nota ya no cancela el guardado pendiente de otra.
  - Se guarda de inmediato al perder el foco de la ventana, al ocultarse, al cambiar de pestaña o al pasar a solo lectura, respetando siempre el bloqueo de archivo.
  - `auto_save_delay_ms` pasa de 600 a 4000 ms y `fs_watcher_recent_save_grace_ms` de 3000 a 8000 ms.

- **Refresco de Git agrupado**: nueva clave `git_status_refresh_debounce_ms` (5000 ms) en `config.jsonc`. Los guardados agrupan el `git status` en lugar de lanzar uno por guardado.

- **Reindexación Full-Text configurable**: nueva clave `indexing.debounce_ms` (5000 ms por defecto, antes fijo en 350 ms). Ya no se reconstruye toda la bóveda en cada guardado: el watcher detecta el cambio.

- **Orden del explorador de la bóveda**: las carpetas con emoji al inicio van primero, seguidas del resto de carpetas y luego archivos, todo en orden alfabético sin distinguir mayúsculas (`sortSiblings` en `VaultExplorer.svelte`).

- **Motor D2 migrado a `@d2lang/d2`**:
  - Se reemplaza el paquete `@terrastruct/d2` por `@d2lang/d2` (v0.1.34) en `package.json` y en el servicio `d2-service.ts`.
  - Nuevo motor de layout `tala` disponible en el selector de D2 junto a `dagre` y `elk`.

- **Ordenamiento de Quick Open reescrito**: se sustituye el sistema de niveles (tiers) y el ordenamiento manual por el puntaje compuesto de `QuickOpenScorer`, y el escaneo lineal de respaldo solo se ejecuta cuando nucleo no termina a tiempo o no ha inyectado todas las notas.

### Corregido

- **Filtrado de Quick Open por palabras**: cada palabra de la consulta debe aparecer en el campo, en cualquier orden (por ejemplo, `bcp ttib` encuentra `BCP - TTIB ...`).
- **Bucle de `nucleo.tick` en búsqueda**: el bucle ya no ejecuta un `tick` de más tras terminar el trabajo de nucleo.
- **Diagramas Mermaid que se re-renderizaban al escribir**: el widget ya no compara posiciones absolutas, así que escribir antes de un diagrama no lo destruye y vuelve a renderizar en cada pulsación.
- **Diagramas D2 con altura cero en WebKit**: el SVG generado recibe `width` y `height` derivados de su `viewBox`, evitando que colapse dentro de las tarjetas de lectura.

## [1.0.0-alpha.1] - 2026-10-28

### Añadido

- **Integración del plugin CLI de Tauri (`@tauri-apps/plugin-cli` / `tauri-plugin-cli`)**:
  - Incorporación de soporte CLI formal en la aplicación de escritorio mediante el plugin oficial de Tauri v2.
  - Opciones de consulta de versión (`--version`, `-V`) y ayuda detallada (`--help`, `-h`) que imprimen información directamente en la terminal y finalizan la ejecución de forma inmediata sin instanciar la interfaz gráfica.
  - Documentación en `--help` de las opciones de apertura de bóveda/archivo (`--vault`/`-v`, `--file`/`-f`, ruta posicional) y de inicio de servidor local stdio MCP (`--mcp`).

- **Preservación y reapertura de pestañas cerradas en la navegación del historial**:
  - Al cerrar una pestaña activa o en segundo plano, su registro se mantiene en `tabHistory` de forma desacoplada y con bajo consumo de memoria.
  - Al utilizar la navegación hacia atrás o adelante (incluyendo los botones laterales del ratón o <kbd>Alt+Left</kbd>/<kbd>Alt+Right</kbd>), si la pestaña destino había sido cerrada, se reabre automáticamente restituyendo su posición física original en la barra de pestañas y conservando el estado de selección del cursor.

- **Parámetros de temporización de editor y Excalidraw configurables en `config.jsonc`**:
  - Nuevas propiedades bajo el bloque `"editor"` tipadas y validadas tanto en el modelo de dominio Rust (`EditorConfig`) como en la entidad frontend (`file-types.svelte.ts`):
    - `drawing_auto_save_delay_ms`: Tiempo de inactividad antes de persistir dibujos a disco (3000 ms por defecto).
    - `drawing_change_debounce_ms`: Tiempo de espera para propagar cambios de escena hacia la pestaña (400 ms por defecto).
    - `fs_watcher_recent_save_grace_ms`: Ventana de gracia para ignorar eventos del watcher originados por guardados propios (3000 ms por defecto).

- **Capa de compatibilidad Obsidian API: Entorno global, emulación Node.js y stubs CodeMirror**:
  - **Emulación de módulos en `require()`**: Soporte para paquetes `@codemirror/search` y `@codemirror/autocomplete`, así como módulos esenciales de Node.js requeridos por plugins comunitarios (`path`, `events`, `constants`, `url`, `punycode`, `crypto` con `createHash` y `randomBytes`, `stream`, `process`, `util` y `querystring`), junto a stubs seguros para dependencias de escritorio (`electron`, `child_process`, `fs`, `http`, `https`, `os`).
  - **Polyfill nativo de `Buffer`**: Implementación basada en `Uint8Array` con codificación y decodificación binaria UTF-8 y Base64 (`Buffer.from`, `isBuffer`, `toString`).
  - **Entorno global y objetos canónicos**: Exposición de `activeWindow`, `global`, `process` y `CodeMirror.defineMode` / `defineSimpleMode` en el contexto global de ejecución.
  - **Nuevas exportaciones en bundle Obsidian**: Incorporación de la clase `Vault` (`SynapseVault`), `Workspace` (`SynapseWorkspace`), `SuggestModal`, `loadMermaid` (con import dinámico seguro), `loadMathJax` y función `getIconIds`.

- **Capa de compatibilidad Obsidian API: Soporte para Vault, Workspace y App**:
  - **Opciones y recorrido en `SynapseVault`**: Métodos `vault.getConfig` (`vimMode`, `theme`, `legacyEditor`, `livePreview`), `vault.getName` y `recurseChildren(folder, callback)` para iteración recursiva de jerarquías de archivos.
  - **Extensiones en `SynapseWorkspace`**: Propiedad `containerEl`, métodos `getMostRecentLeaf()` e `iterateAllLeaves(callback)`, disparo de eventos con `updateOptions()` y registro de fuentes de vista previa flotante (`registerHoverLinkSource` / `unregisterHoverLinkSource`).
  - **Persistencia local y propiedades en `SynapseApp`**: Métodos `loadLocalStorage` y `saveLocalStorage` para plugins que almacenan ajustes temporales, objeto `app.foldManager` (`load`, `save`) para soporte de plegado de código/secciones y bandera `app.isMobile = false` para compatibilidad con plugins de calendario y vistas de notas diarias.
  - **Caché de metadatos**: Método `metadataCache.getCachedFiles()` para consultar todas las rutas registradas en la caché.

- **Capa de compatibilidad Obsidian API: Ecosistema de Plugins, UI y Barra de Estado**:
  - **Ciclo de vida en `Plugin`**: Métodos `registerObsidianProtocolHandler`, `registerEditorSuggest` y `registerHoverLinkSource` con limpieza automática en `onunload`.
  - **Registro centralizado en `PluginLoader`**: Exposición del mapa `app.plugins.plugins[pluginId]` asignado antes de la inicialización de cada plugin y limpieza al descargarse.
  - **Gestor `SynapseInternalPluginsManager` (`app.internalPlugins`)**: Soporte para plugins nativos (`app.internalPlugins.getPluginById` y `getEnabledPluginById`), requerido por plugins comunitarios como `Calendar`, `Periodic Notes` y la biblioteca `obsidian-daily-notes-interface`.
  - **Soporte nativo de `daily-notes`**: Registro del plugin interno `daily-notes` con configuración predeterminada (`YYYY-MM-DD`, carpeta y plantilla), cargando asíncronamente las opciones de `.obsidian/daily-notes.json` o `.synapse/daily-notes.json`.
  - **Plugins internos y stubs**: Registro de `workspaces`, `file-recovery` (con manejadores asíncronos seguros) y stubs para `backlink`, `outgoing-link`, `tag-pane`, `page-preview` y `graph`.
  - **Integración con la barra de estado**: `SynapseStatusBarManager` conecta directamente los elementos `.status-bar-item.plugin-status-item` creados por plugins con el componente visual `StatusBar.svelte` en la parte inferior del espacio de trabajo.
  - **Componentes de UI y modales**: Clase base `SuggestModal<T>` con contenedor de resultados y `FuzzySuggestModal<T>` conectada; métodos fluidos `setPlaceholder` y `setInstructions` en `Modal`.

- **Capa de compatibilidad Obsidian API: Extensiones DOM nativas y biblioteca `moment()`**:
  - **Extensiones DOM nativas en `HTMLElement.prototype`**: Métodos canónicos de Obsidian (`setText`, `getText`, `addClass`, `removeClass`, `addClasses`, `removeClasses`, `toggleClass`, `hasClass`, `setAttr`, `getAttr`) en el prototipo global del DOM para manipulación segura sin excepciones en tiempo de ejecución.
  - **Soporte ampliado de fechas y locales en `moment()`**: Métodos de comparación y límites (`isSame`, `isBefore`, `isAfter`, `startOf`, `endOf`), manipulación de unidades (`year`, `month`, `date`, `day`, `weekday`, `isoWeekday`, `week`, `set`, `get`), formateo relativo (`calendar()`), métodos de internacionalización (`locale`, `localeData`, `moment.locale`, `moment.localeData`, `moment.updateLocale`, `moment.locales`, `moment.weekdays`, `moment.weekdaysShort`, `moment.weekdaysMin`) y spec global `_bundledLocaleWeekSpec`.

- **Indexación en memoria de carpetas (`DashSet`) y navegación rápida**:
  - **`NavigationEngine` (Backend Rust)**: Nueva colección concurrente `folders: Arc<DashSet<CompactString>>` para registrar directorios de la bóveda (incluidas carpetas vacías).
  - **Hidratación en frío (< 1 ms)**: Pre-poblado instantáneo de carpetas ancestros al arrancar desde las rutas en RAM de las notas sin romper la compatibilidad binaria del caché `bincode`.
  - **Watcher reactivo de filesystem**: Sincronización en vivo de carpetas ante eventos de creación, eliminación y renombrado en disco.
  - **Servicio de dominio puro (`domain/services/folder_service.rs`)**: Métodos `extract_ancestor_folders` y `summarize_folder` para cálculo de huella semántica (`notes_count`, `subfolders_count`, `dominant_tags` y `sample_notes`) en memoria sin I/O directo.
  - **Comando Tauri `search_folders_command`**: Búsqueda difusa y paginada de carpetas con retorno de huella semántica.

- **Herramientas Agénticas MCP avanzadas para carpetas**:
  - **`search_folders`**: Permite a los LLMs buscar carpetas con coincidencia difusa y recibir su huella semántica (conteo, tags más frecuentes y notas de muestra) para desambiguar carpetas homónimas en una sola llamada sin sobrecargar la ventana de contexto.
  - **`move_folder_contents`**: Mueve de forma masiva notas entre directorios con prevención de path traversal y bucles infinitos, filtro opcional por extensión, estrategia de resolución de conflictos (`skip`, `rename`, `overwrite`) y soporte de simulacro `dry_run`.
  - **Paginación en `list_folder_contents`**: Soporte de `limit` y `offset` con `has_more` y `next_offset` para directorios con cientos de notas.
  - **Filtro de ruta en `full_text_search`**: Soporte de coincidencias parciales por segmento de carpeta con alto recall.

- **UX mejorada en `Ctrl+O` (Quick Open)**:
  - **Modo `/` (`DIR`)**: Escribir `/` filtra exclusivamente carpetas en la bóveda con badge indicador visual.
  - **Colapso inteligente a Top 3 en búsqueda normal**: En consultas generales, las carpetas coincidentes se limitan a las 3 más relevantes para evitar desplazar las notas y archivos hacia abajo.
  - **Elemento interactivo "Ver X carpetas más..."**: Alternancia expandir/colapsar con soporte completo de clic de ratón o teclado (`Enter`).
  - **Drill-down interactivo y botón Revelar**: Presionar Enter sobre una carpeta autocompleta la búsqueda a `ruta/carpeta/`, o botón para revelarla directamente en el explorador lateral.

- **Navegación unificada por páginas con retención de altura relativa (`PageDown` / `PageUp`)**:
  - **Módulo compartido `shared/lib/navigation` (`relative-page-navigator.ts`)**: Utilidad modular reusable que calcula el vector $\Delta Y = Y_{\text{target}} - Y_{\text{current}}$ y desplaza el contenedor asegurando que el elemento seleccionado mantenga la misma fila visual o altura en pantalla (Relative Viewport Retention).
  - **QuickOpen (`Ctrl+O`)**: Navegación adaptativa a 60 FPS fijos que preserva la posición vertical del elemento seleccionado o hace clamping seguro en los extremos.
  - **Búsqueda Full-Text (`Ctrl+Shift+F`)**: Soporte completo de `PageDown` y `PageUp` en `FullTextSearch` con retención de posición relativa y sincronización de foco.
  - **VaultExplorer (Explorador lateral)**: Soporte de `PageDown` y `PageUp` con navegación relativa tanto para selección simple como por bloques de rango con `Shift`.

### Corregido

- **Optimización de rendimiento y fluidez en la edición de Excalidraw**:
  - **Eliminación del retardo al escribir texto**: Se suspendió la serialización de escena y compresión LZ en el hilo principal mientras el usuario edita activamente texto (`appState.editingTextElement`), evitando que los caracteres se retengan en búfer y se dibujen con desfase.
  - **Desacoplamiento del Proxy reactivo de Svelte 5**: Sustitución de `$state<any>(null)` por variable plana no reactiva para `excalidrawAPI` en `ExcalidrawViewer`, eliminando sobrecostos de intercepción por Proxy sobre métodos y estructuras internas complejas de Excalidraw.
  - **Protección contra sobreescritura destructiva de escena**: En el `$effect` sincronizador, se evita llamar a `excalidrawAPI.updateScene()` durante la edición de texto activa para impedir el bloqueo y congelamiento momentáneo del textarea WYSIWYG.
  - **Eliminación de doble compresión y serialización en `isTabDirty`**: `WorkspacePage` ahora verifica el estado sucio mediante comparación directa en memoria (`note.content !== note.savedContent`), prescindiendo de llamadas síncronas a `getCurrentSerializedScene()` en cada re-renderizado reactivo de la barra de pestañas.
  - **Omisión de conteo de palabras en dibujos**: `docWordCount` y `docCharCount` se omiten cuando la pestaña activa es de tipo dibujo (`isDrawingTab`), ahorrando costosas ejecuciones de expresiones regulares sobre JSON y base64.
  - **Generación perezosa de SVG en Obsidian Excalidraw**: En `ObsidianExcalidrawViewer`, la generación vectorial con `ExcalidrawAutomate.toSVG` ahora se ejecuta exclusivamente en modo visual (`viewMode === 'visual'`), evitando cálculos pesados en cada pulsación del modo código.
  - **Supresión de recargas redundantes del backend en `vault:files-changed`**: Si todos los archivos notificados por el watcher del sistema provienen de guardados propios recientes dentro de la ventana de gracia y no hay archivos eliminados, se omite `fetchNotesFromBackend()` y `refreshTree()`, refrescando únicamente el estado de Git de manera desacoplada.

- **Eliminación de la doble ejecución de `onload()` en el ciclo de vida de plugins**:
  - En `PluginLoader`, corrección de la invocación redundante de `instance.onload()` tras `instance.load()`, asegurando que `onload()` se ejecute una única vez por plugin activo.
  - Soporte para métodos de ciclo de vida asíncronos en `Component.prototype.load()` y `Component.prototype.unload()`, aguardando adecuadamente promesas devueltas por `onload()` y `onunload()`.

- **Protección defensiva contra desbordamiento de pila (`Maximum call stack size exceeded`) por monkey-patching**:
  - **`SynapseDataAdapter` (`adapter.read`, `readBinary`, `write`)**: Detección y corte automático de ciclos recursivos en lecturas y escrituras simultáneas sobre la misma ruta activa provocados por plugins comunitarios (ej. `gpg-crypt`), delegando de manera segura en el repositorio base sin agotar la pila de llamadas.
  - **Plugin interno `file-recovery`**: Guardianes de reingreso por archivo (`activeFileRecoveryPaths`) en `onFileChanged` y `forceAdd`, garantizando que intercepciones circulares sobre eventos de modificación se resuelvan sin recursión infinita.

- **Aislamiento de plugins comunitarios en `.synapse/plugins`**:
  - Restricción del descubrimiento, escaneo, carga y desinstalación de plugins para operar de manera autónoma en el directorio `.synapse/plugins/` de la bóveda, evitando colisiones o interferencias con la configuración nativa de `.obsidian/plugins/`.

- **Estabilidad y robustez en el editor Markdown (CodeMirror 6)**:
  - **Recuperación transparente ante reconfiguración de extensiones**: Detección del error `Field is not present in this state` al inyectar dinámicamente StateFields de plugins, recreando el `EditorState` completo sin interrumpir al usuario y preservando la posición de selección y el documento activo.
  - **Debounce en actualización de extensiones de plugins**: Debounce de 150 ms en `WorkspacePage` para el evento `editor-extensions-change`, agrupando la inicialización concurrente de múltiples plugins comunitarios.

## [2026.10.27.04-alpha] - 2026-10-27

### Añadido

- **Creación y visibilidad de carpetas en VaultExplorer**:
  - **Creación directa de carpetas**: Botón de acceso rápido con icono `FolderPlus` en la cabecera del explorador de la bóveda y opción "Nueva carpeta" en el menú contextual del árbol de archivos.
  - **Soporte y persistencia de carpetas vacías**: Se garantiza que las carpetas vacías se listen y permanezcan visibles en la jerarquía del explorador sin ser ocultadas ni descartadas en el filtrado de archivos.
  - **Soporte para renombrar carpetas**: Integración fluida del renombrado en línea para nodos de tipo carpeta en el árbol.

- **Comando Git Push y atajo de teclado (`Ctrl+Shift+K`)**:
  - **Paleta de comandos (`Ctrl+P`)**: Nuevo comando "Bóveda: Empujar cambios a Git (Push)" registrado en la paleta de comandos del espacio de trabajo.
  - **Atajo global configurable**: Asignado por defecto a `Ctrl+Shift+K`, configurable en `config.jsonc` y gestionado mediante el store de atajos de la aplicación.
  - **Backend y Arquitectura Cebolla**:
    - Puerto `git_push` en el trait `GitPort` en `domain/services/git_service.rs`.
    - Caso de uso `git_push` en `application/use_cases/git_use_cases.rs`.
    - Adaptador de infraestructura en `infrastructure/services/git_service.rs` invocando `git push` con manejo seguro de errores.
    - Exposición del comando Tauri `git_push_vault` en `infrastructure/tauri/commands/git.rs`.
  - **Feedback visual**: Notificaciones toast contextuales de éxito y error en la interfaz de usuario al sincronizar con el repositorio remoto.

- **Creación de archivos de diagramas D2 (`.d2`)**:
  - **Opción en menú contextual y paleta de comandos**: Acción "Nuevo diagrama D2" añadida al menú contextual de archivos/carpetas y en la paleta de comandos (`cmd-new-d2`).
  - **Plantilla predeterminada**: Generación automática de código fuente inicial (`x -> y: conexión\n`) y apertura directa en el visor de diagramas D2.

## [2026.10.27.03-alpha] - 2026-10-27

### Añadido

- **Menú contextual y copiado de URL del Servidor MCP en la barra de estado**:
  - **Menú contextual accesible con Bits-UI (`ContextMenu.Root`)**: Permite hacer clic derecho sobre el indicador MCP en la barra de estado para acceder rápidamente a acciones clave.
  - **Acción para copiar URL de MCP**: Opción "Copiar URL del servidor MCP" (`/mcp` endpoint con puerto dinámico) al portapapeles con feedback visual mediante toast en la barra de estado.
  - **Acción rápida para alternar servidor**: Alternar activación/desactivación del servidor MCP para la bóveda actual directamente desde el menú contextual.
  - **Sincronización de estado reactiva en WorkspacePage**: Llamadas a `refreshMcpStatus()` en la apertura de la bóveda, al cargar carpetas recientes y al inicializar pestañas de workspace.

### Cambiado

- **Actualización de dependencias del Backend (Rust / Cargo)**:
  - Actualización del motor de búsqueda de texto completo `tantivy` a `0.27.0`.
  - Actualización del parser XML `quick-xml` a `0.42.0`, adaptando `docx_extractor.rs` al nuevo tipado `&str` de nombres de elementos y eventos de texto.
  - Actualización de `lopdf` a `0.46.0` para extracción e indexación de PDFs.
  - Fijada la dependencia de `bincode` en `2.0.1` (con feature `serde`) garantizando serialización binaria estable y compatible en caché.

### Añadido

- **Búsqueda y Exploración por Etiquetas (#tags) en Quick Open (`Ctrl+O`)**:
  - **Integración en Memoria RAM (`DashMap<NoteId, NoteMeta>`)**:
    - Persistencia de `tags: Vec<CompactString>` directamente en la estructura caliente `NoteMeta`, extrayendo etiquetas automáticamente (tanto YAML frontmatter como hashtags `#tag` inline) durante la reconciliación y el monitoreo de cambios (`watcher`).
    - Consumo de memoria ultra-reducido (< 1 MB de RAM para 10,000 notas) mediante `CompactString` inline sin asignaciones en heap para tags de hasta 24 bytes.
    - `get_vault_tags()` y `get_notes_by_tag()` ahora resuelven en memoria en **< 0.5 ms** (eliminando I/O y lecturas sincrónicas a disco).
  - **Indexación Multihilo en `nucleo`**:
    - Inyección de etiquetas en el motor difuso `nucleo` para permitir coincidencias a 60 FPS al buscar por nombre de tag con o sin el símbolo `#`.
    - Soporte completo a sub-etiquetas jerárquicas con coincidencia flexible (`linux/cachyos` responde a consultas como `#linux` o `#cachyos`).
    - Búsqueda combinada de texto y etiqueta (ej. `kernel #cachyos`).
  - **Experiencia de Usuario en Frontend (`QuickOpen.svelte`)**:
    - Modo de exploración de etiquetas: al escribir `#`, sugiere la lista de tags disponibles con conteo dinámico de notas asociadas y autocompletado en un clic.
    - Badges/pills de etiquetas interactivos en los resultados de búsqueda: permiten re-enfocar la consulta en una etiqueta específica al hacer clic.
    - Indicador visual y badge `TAG` en la caja de búsqueda.

- **Visualización de Frontmatter en Modo Lectura (Reading View)**:
  - **Panel de Propiedades Interactivo (`NoteProperties.svelte`)**:
    - Visualización limpia y estructurada de metadatos YAML en la parte superior del documento Markdown en modo lectura, evitando renderizar bloques de código o triples guiones crudos.
    - Detección inteligente de tipos de propiedades: etiquetas interactivas (`#tag`) con acción de búsqueda, fechas con iconos contextuales, enlaces y wikilinks (`[[Nota]]`), badges booleanos (`Verdadero` / `Falso`) y arrays/listas en chips compactos.
    - Botón de copiado al portapapeles en hover para cada propiedad y cabecera colapsable con recuento dinámico de metadatos.
  - **Aislamiento y Extracción Limpia (`frontmatter-extractor.ts`)**:
    - Separación atómica de metadatos YAML y del cuerpo Markdown antes del procesamiento con `marked`, asegurando que el contenido del documento no muestre duplicaciones de frontmatter.
    - Suite de pruebas unitarias exhaustiva con Bun/Vitest (`frontmatter-extractor.test.ts`).
  - **Preferencias de Usuario (`propertiesViewMode`)**:
    - Soporte en `appSettings` y `settings.svelte.ts` para modos `'expanded'` (expandido por defecto), `'collapsed'` (colapsado por defecto) y `'hidden'` (oculto).

- **Optimización de Capacidades MCP (Sub-tags, Tolerancia de Espacios y Ergonomía)**:
  - **Sub-tag Matching Jerárquico en `search_notes` y `full_text_search`**:
    - Incorporación de `matches_tag` y `any_tag_matches` en [`tag_service.rs`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src-tauri/src/domain/services/tag_service.rs), permitiendo que búsquedas por tags como `cachyos` o `linux` coincidan automáticamente con sub-etiquetas jerárquicas como `linux/cachyos` u `os/linux/cachyos`.
    - Soporte unificado de etiquetas YAML e inline hashtags (`#etiqueta`) extraídos del cuerpo de la nota.
    - Argumento `query` opcional en `search_notes`, facilitando el filtrado exclusivo por etiquetas.
  - **Snippets Limpios y Métricas en `search_notes`**:
    - Retorno de `snippet` (primeros 200 caracteres limpios de cuerpo sin frontmatter ni delimitadores de código), `description` (desde `extra_properties`), `total_lines` y `estimated_tokens`, evitando lecturas completas innecesarias con `read_note`.
    - Carga en memoria del contenido Markdown en `FileNoteRepository::walk_directory` para entregar resultados instantáneos.
  - **Parche Quirúrgico Tolerante y Anclaje Difuso en `patch_note`**:
    - Función pura `apply_surgical_patch` con reemplazo en múltiples niveles de tolerancia: coincidencia exacta, normalización `\r\n` a `\n` (CRLF/LF), y anclaje difuso por líneas ignorando espacios en blanco finales (`.trim_end()`) e indentación (`.trim()`).
    - Desambiguación automática de múltiples ocurrencias utilizando `start_line`.
    - Mensajes de error enriquecidos que muestran las líneas reales circundantes más parecidas en caso de fallo de coincidencia.
  - **Ergonomía de Lectura en `read_note`**:
    - Parámetro `outline_only: true` (y modo `outline_only`) para obtener la estructura TOC y metadatos sin descargar el contenido completo.
    - Parámetro `section` como alias de `section_heading` con inferencia automática del modo `section`, y reporte de secciones disponibles en caso de no encontrar el encabezado solicitado.

- **Herramienta MCP `delete_note` y Estandarización de Anotaciones Normativas**:
  - **`delete_note` (Nuevo)**: Eliminación permanente y segura de notas en disco con validación estricta de límites de la bóveda (prevención de Path Traversal) y soporte de Control Optimista de Concurrencia mediante `expected_hash`.
  - **Estandarización Normativa de Anotaciones MCP (`McpToolAnnotations`)**: Metadatos `readOnlyHint`, `destructiveHint`, `idempotentHint` y `openWorldHint` alineados con la especificación oficial de MCP y el estándar de facto de `@modelcontextprotocol/server-filesystem`:
    - `openWorldHint: false` en todo el catálogo (entorno local cerrado).
    - `destructiveHint: true` en operaciones que sobrescriben o eliminan datos (`write_note`, `patch_note`, `update_note_metadata`, `move_notes`, `delete_note`).
    - `append_to_note` catalogada como la única operación de escritura puramente aditiva (`destructiveHint: false`, `idempotentHint: false`).
    - `idempotentHint: false` para operaciones relativas, dependientes de estado o de un solo uso (`patch_note`, `move_notes`, `delete_note`, `append_to_note`).

## [2026.10.27.02-alpha] - 2026-10-27

### Añadido

- **Evolución del Catálogo Unificado de Herramientas MCP para Synapse**:
  - **Catálogo de 14 Herramientas de Precisión Quirúrgica y Multimodalidad**:
    - **Control Optimista de Concurrencia (OCC) y Hashing Criptográfico**: Incorporación de `calculate_content_hash` con SHA-256 (`sha256:<hex>`) en `read_note`, `write_note`, `patch_note`, `append_to_note` y `update_note_metadata` para prevenir sobreescrituras ciegas entre agentes y la UI de edición.
    - **`read_note` Extendido**: Modos `full`, `chunk` (rango acotado con `start_line` y `line_count`), `section` (extracción aislada de encabezados) y `metadata_only`. Generación de Tabla de Contenidos (TOC) con números de línea, conteo de bloques `mermaid`/`d2`, estimación de tokens (`estimated_tokens`) y prefijo opcional de números de línea (`include_line_numbers`).
    - **`write_note` Robusto**: Título opcional conservando metadatos OKF preexistentes, freno anti-truncamiento (>150 líneas y >40% de reducción), validación `expected_hash` y previsualización con `dry_run` generando `unified_diff`.
    - **`patch_note` (Nuevo)**: Edición quirúrgica de fragmentos con reemplazo atómico, anclaje difuso tolerante a desfases, control de concurrencia OCC y generación de diffs unificados (`similar`).
    - **`search_in_note` (Nuevo)**: Grep contextual dentro de una nota que retorna número de línea, coincidencia y líneas de contexto circundantes (`context_lines`).
    - **`append_to_note` (Nuevo)**: Inserción directa al final del documento o en secciones de encabezado específicas (`start_of_section` o `end_of_section`).
    - **`update_note_metadata` (Nuevo)**: Mutación atómica exclusiva de frontmatter YAML (añadir/quitar etiquetas sin duplicados, actualizar título, aliases y propiedades personalizadas) preservando el cuerpo intacto.
    - **`read_pdf_document` (Nuevo)**: Inspección de documentos PDF con `lopdf` (modos `outline_only`, lectura por rango de páginas con límite estricto de 5 páginas para proteger la ventana de contexto, y búsqueda interna `search`).
    - **`extract_note_diagrams` (Nuevo)**: Extracción especializada de diagramas Mermaid y D2lang con líneas de inicio/fin o lectura directa de archivos `.d2` y `.mmd`.
    - **`get_backlinks_and_relations` Enriquecido**: Inclusión de snippets contextuales de la línea que contiene el enlace, detección de enlaces huérfanos/no resueltos (`unresolved_links`) y cálculo de similitud Jaccard de etiquetas para el top 5 de notas conceptualmente afines (`shared_tags_neighbors`).
    - **`move_notes` con Reescritura de Wikilinks**: Reescritura automática de referencias `[[...]]` en la bóveda cuando `update_wikilinks: true`.
    - **Anotaciones MCP (`McpToolAnnotations`)**: Metadatos `readOnlyHint`, `destructiveHint` e `idempotentHint` para gobernanza segura Human-in-the-Loop en clientes ACP y UI.

- **Menú Contextual y Estado Git en Breadcrumbs de Archivo**:
  - **Indicadores Visuales de Estado Git en Breadcrumbs**:
    - Se muestran insignias con código de colores según el estado de Git en la barra de ruta (`BreadCrumb.svelte`) junto al archivo activo:
      - `M` verde para cambios staged / preparados (`index: 'M'`).
      - `A` verde para archivos nuevos staged (`index: 'A'`).
      - `M` rojo para modificaciones locales sin preparar en el worktree (`worktree: 'M'`).
      - `?` ámbar para archivos sin rastrear (`worktree: '?'`).
      - `S` púrpura para archivos con cambios guardados en stash (`is_stashed: true`).
    - Lógica de normalización desacoplada y compartida en `@entities/git` (`parseGitStatusInfo`, `NormalizedGitStatus`) respetando FSD y evitando duplicación de código.
  - **Menú Contextual Extendido con Soporte Completo de Git**:
    - Al hacer clic derecho sobre el segmento del archivo activo en el breadcrumb, se despliega un menú contextual accesible (`bits-ui ContextMenu`) con opciones clave del archivo y suite completa de acciones Git:
      - *Hacer commit...*: Abre directamente el diálogo de confirmación de Git para el archivo actual.
      - *Ver diferencias (Diff / Diff staged)*: Soporte para diff de texto y visual (2 paneles) para archivos gráficos, diagramas o PDFs.
      - *Historial de Git (Log)...*: Abre el diálogo de historial de revisiones del archivo activo.
      - *Git Add*: Prepara el archivo activo directamente al índice/stage de Git con notificación toast.
      - *Git Restore --staged*: Desmarca el archivo del stage (`git restore --staged`).
      - *Git Restore*: Abre diálogo de confirmación seguro y descarta las modificaciones locales en el árbol de trabajo.
      - *Abrir en nueva ventana* (desacopla o abre una ventana independiente del archivo).
      - *Abrir con la app por defecto* del sistema operativo.
      - *Mostrar en carpeta* (revela el archivo en el explorador de archivos nativo).
      - *Renombrar archivo* (activa la edición directa inline del breadcrumb).
      - *Copiar ruta* (copia la ruta relativa del archivo al portapapeles).
    - **Reutilización y Arquitectura FSD**: Componente [`GitRestoreDialog`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src/shared/ui/git-restore-dialog/) promovido a `@shared/ui` para ser reutilizado tanto por el explorador de la bóveda (`VaultExplorer`) como por el breadcrumb (`BreadCrumb`).

- **Persistencia Per-Vault y Registro Global de Puertos del Servidor MCP**:
  - **Estado y Puerto Recordado por Bóveda**:
    - Cada bóveda almacena su preferencia de activación (`enabled: true/false`) y su puerto TCP asignado en `<vault>/.synapse/mcp.json`.
    - Al abrir o cambiar de bóveda (`activate_vault`), Synapse sincroniza automáticamente el servidor MCP, iniciando o apagando el runtime según corresponda para esa bóveda concreta sin interferir con otras.
  - **Registro Global en `~/.config/synapse/mcp-ports.json`**:
    - Se registra el mapeo de todas las bóvedas, puertos asignados, PID del proceso activo y estado de ejecución en `~/.config/synapse/mcp-ports.json` (y `~/.config/synapsers/mcp-ports.json`).
    - Permite a múltiples instancias concurrentes de `synapsers` / MCP coordinar y descubrir qué puertos están en uso activo.
    - **Prevención Inteligente de Conflictos**: Algoritmo de asignación que detecta procesos muertos y busca el siguiente puerto TCP disponible (rango `27182-28000`) cuando el puerto preferido ya está ocupado por otra bóveda o servicio.
    - Limpieza automática al cerrar la ventana principal de Synapse para liberar el puerto registrado.
    - Nuevo comando Tauri `get_mcp_port_registry` y tipos actualizados en `system.api.ts`.

- **Navegación con Atajos de Vim en Modo Lectura (Reading Mode)**:
  - Soporte completo para desplazarse por notas Markdown en modo lectura utilizando teclas de movimiento estándar de Vim / *pager*:
    - **Desplazamiento vertical**: <kbd>j</kbd> (bajar) y <kbd>k</kbd> (subir) con paso continuo sin retardo al mantener la tecla pulsada; <kbd>d</kbd> / <kbd>Ctrl+D</kbd> (media página abajo) y <kbd>u</kbd> / <kbd>Ctrl+U</kbd> (media página arriba) con scroll suave; <kbd>Ctrl+E</kbd> / <kbd>Ctrl+Y</kbd> (desplazamiento línea a línea).
    - **Salto al inicio y fin**: <kbd>gg</kbd> (doble pulsación rápida de `g`) para desplazarse al inicio del documento y <kbd>G</kbd> (<kbd>Shift+G</kbd>) para ir directamente al final.
    - **Desplazamiento horizontal**: <kbd>h</kbd> (izquierda), <kbd>l</kbd> (derecha), <kbd>0</kbd> (inicio horizontal) y <kbd>$</kbd> (fin horizontal).
    - **Búsqueda interactiva estilo Vim**: <kbd>/</kbd> para abrir la barra de búsqueda en el documento, <kbd>n</kbd> / <kbd>N</kbd> para navegar entre coincidencias y <kbd>Escape</kbd> para cerrar la búsqueda devolviendo el foco inmediatamente al documento para continuar navegando.
    - **Aislamiento seguro**: no se interceptan teclas cuando el usuario escribe en un campo de texto, `<input>` o `<textarea>`.

### Corregido

- **Foco Automático en el Contenido Markdown al Cambiar de Pestaña**:
  - Se corrigió la pérdida de foco al alternar pestañas (mediante clic en pestaña, atajos <kbd>Ctrl+Tab</kbd> / <kbd>Ctrl+PageDown</kbd> o desde QuickOpen): el foco se transfiere automáticamente al contenido principal de la nota (`.markdown-reading-wrapper` en modo lectura y `editorView` de CodeMirror en modo edición).
  - Permite utilizar las flechas de teclado (<kbd>ArrowDown</kbd>, <kbd>ArrowUp</kbd>, <kbd>PageDown</kbd>, <kbd>PageUp</kbd>) inmediatamente sin necesidad de hacer clic manual con el ratón sobre el documento.
  - Sincronización precisa con el ciclo de vida de Svelte 5 y `ResizeObserver` para otorgar el foco en el momento exacto en que la pestaña pasa de `display: none` (`.tab-pane.hidden`) a visible, respetando diálogos modales activos (`[role="dialog"]`).

## [2026.10.27.01-alpha] - 2026-10-27

### Añadido

- **Soporte Multilenguaje (i18n) con Svelte 5 Runes**:
  - Implementación nativa de internacionalización en `src/shared/lib/i18n/` sin dependencias externas:
    - Diccionarios en español (`es.ts`) e inglés (`en.ts`) con tipado estricto en TypeScript (`TranslationKey`, `LocaleSchema`) y verificación de paridad de claves en tiempo de compilación.
    - Gestor reactivo `I18nManager` (`i18n.svelte.ts`) usando `$state` de Svelte 5 y función de traducción `t(...)` con soporte para interpolación dinámica de variables (`{param}`).
    - Detección automática del idioma preferido del sistema/navegador y persistencia en `localStorage` sincronizada con `AppSettings`.
  - Integración completa en el modelo de configuración global (`appSettings.locale`, `setLocale`, `toggleLocale`).
  - Botón de cambio rápido de idioma en la barra de herramientas lateral (`Ribbon`) con el icono de `Languages`, y en la barra superior de la pantalla de bienvenida (`WelcomeScreen`).
  - Nuevo comando en la paleta de comandos (<kbd>Ctrl+P</kbd>): *"Cambiar idioma de la interfaz (Español / English)"* bajo la categoría *"Configuración"*.
  - Internacionalización reactiva de la barra lateral (`Ribbon`), pantalla de bienvenida (`WelcomeScreen`), formateador de tiempo relativo y plurales de conteo de notas.

- **Búsqueda en Vista de Markdown de Solo Lectura (Reading Mode)**:
  - Soporte para buscar texto dentro de notas Markdown en modo lectura interactivo:
    - Barra de búsqueda flotante accesible con <kbd>Ctrl+F</kbd> / <kbd>Cmd+F</kbd>.
    - Resaltado visual en el documento (`<mark class="reading-search-highlight">`) con scroll automático suave hacia la coincidencia seleccionada (`.reading-search-active`).
    - Contador de coincidencias en tiempo real (`X de N` o `Sin coincidencias`).
    - Navegación bidireccional entre resultados mediante <kbd>Enter</kbd> (siguiente), <kbd>Shift+Enter</kbd> (anterior) o botones de interfaz.
    - Alternancia de coincidencia de mayúsculas y minúsculas (*case-sensitive*) con el botón `Aa`.
    - Cierre rápido y limpieza de resaltados mediante tecla <kbd>Escape</kbd> o botón de cierre.
  - Integración transparente en `MarkdownViewer.svelte` y el despachador de atajos globales de `WorkspacePage.svelte`.
- **Navegación Circular entre Pestañas con Atajos de Teclado**:
  - Soporte para alternar rápidamente entre pestañas abiertas:
    - **Pestaña siguiente**: <kbd>Ctrl+PageDown</kbd> o <kbd>Ctrl+Tab</kbd>.
    - **Pestaña anterior**: <kbd>Ctrl+PageUp</kbd> o <kbd>Ctrl+Shift+Tab</kbd>.
  - Comportamiento circular infinito (al llegar al final rota a la primera pestaña y viceversa).
  - Consciencia de panel en vista dividida (*Split View*), rotando en el panel activo (`activeFocusedPane`).
  - Normalización robusta de combinaciones y alias de teclas en `matchesShortcut` (`pageup`, `pgup`, `prior`, `pagedown`, `pgdn`, `next`).
  - Configurable en `shortcuts.jsonc` y registrado en la paleta de comandos (`Ctrl+P`).
- **Cierre en Cascada de Ventanas Desacopladas al Cerrar la Ventana Principal**:
  - Al cerrar la ventana principal (`main`) de Synapse, tanto desde los controles de ventana como por atajos o eventos del sistema, todas las ventanas secundarias desacopladas (`detached windows`) se cierran automáticamente.
  - Implementación coordinada a nivel de ciclo de vida del backend Tauri (`on_window_event` en `Destroyed` de la ventana `main`) y en el frontend de `WorkspacePage` (`onCloseRequested` y confirmación de cierre de ventana).

### Corregido

- **Fluidez de Navegación por Teclado, Rendimiento de Búsqueda y Corrección de Bucles Reactivos**:
  - **Corrección de `effect_update_depth_exceeded`**: Se eliminaron los ciclos de actualización infinita en `QuickOpen.svelte` y `WorkspacePage.svelte` aislando las mutaciones de estado reactivo mediante `untrack()` de Svelte 5 y guardas de cambio previo en los bloques `$effect` (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **Precompilación de Atajos `O(1)`**: En `shortcuts.svelte.ts`, se implementó un caché de combinaciones parseadas (`comboCache`) para evitar ejecutar `.split(',')`, `.split('+')` y `.toLowerCase()` en cada pulsación de tecla.
  - **Fast-Reject en Despachador de Teclado**: En `WorkspacePage.svelte`, se añadió una comprobación inmediata en `O(1)` que ignora teclas sin modificadores (`Ctrl`/`Meta`/`Alt`/`F12`), evitando evaluar las 19 comprobaciones de atajos al navegar con flechas o escribir texto.
  - **Aislamiento y Centralización de Listeners**: Se eliminó el listener redundante de `window.addEventListener('keydown')` en `CommandPalette.svelte`, centralizando los atajos globales en `WorkspacePage`.
  - **Optimización de Renderizado en WebKitGTK**: Se reemplazó el costoso `backdrop-filter: blur(10px)` de pantalla completa por fondos sólidos semitransparentes en `QuickOpen.svelte`, `CommandPalette.svelte` y `FullTextSearch.svelte`, suprimiendo la `transition` en filas de resultados y añadiendo `contain: layout style paint` para cambios de fila instantáneos (1 solo frame).
  - **Búsqueda Indexada en Fallback de QuickOpen**: Se indexaron los `vaultItems` en un `Map` para resolver búsquedas en $O(1)$ en lugar de ejecutar `.find()` en $O(N)$ repetidos.

- **Backend Rust Asíncrono fuera del Hilo Principal de GTK**:
  - Se convirtieron a `pub async fn` y `tauri::async_runtime::spawn_blocking` los comandos Tauri bloqueantes que operaban en el hilo principal de la UI: `get_system_theme` (detección del SO vía `busctl`/`gsettings`), `search_notes_command` (búsqueda difusa en Nucleo), `get_vault_git_status` (verificación de estado de Git en foco de ventana) y comandos de gestión de espacios de trabajo (`workspace.rs`).
  - Se derivó `Clone` en `GitUseCases` y `GitService` respetando la Arquitectura Cebolla.

- **Limpieza de Errores Falsos en Consola para Plugins de Comunidad**:
  - Se silenciaron los mensajes de error en `TauriVaultRepository.readNote` para archivos de configuración opcionales (`.synapse/` y `.obsidian/`) que no existen en bóvedas recién abiertas.
  - Se corrigió el valor por defecto de `getEnabledPlugins()` para retornar una lista vacía `[]` si la bóveda no tiene plugins comunitarios configurados ni instalados, evitando intentos de carga innecesarios.

- **Optimización de Rendimiento y Eliminación de Congelamientos ("Micro-Stutters") al Guardar y Modificar Archivos**:
  - **Backend Rust Asíncrono**: Se reemplazaron las llamadas sincrónicas y bloqueantes de `reconcile_sync()` por [`spawn_reconciliation()`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src-tauri/src/infrastructure/navigation/engine.rs#L313) en los comandos Tauri de guardado de notas (`save_note_content`), creación de carpetas (`create_vault_folder`), operaciones de archivos (`delete_vault_item`, `rename_vault_item`, `write_binary_file`, `save_binary_asset`) y cambio de ramas Git (`git_checkout_branch`, `git_create_branch`). Esto evita que el hilo de comandos de Tauri quede bloqueado esperando recorridos completos de disco (`jwalk`).
  - **Debounce en Eventos del Frontend**: Se añadió un temporizador de debounce (350 ms) en el listener de `vault:files-changed` dentro de `WorkspacePage.svelte`, evitando re-renderizados en cascada, recálculos pesados de Git y llamadas continuas a `fetchNotesFromBackend()` durante operaciones rápidas o ráfagas de auto-guardado.

- **Rendimiento y Prevención de Congelamiento en Visor de Diff de Texto (Excalidraw y Archivos Grandes)**:
  - Se condicionó la expansión automática de líneas en `DiffViewer.svelte` mediante un nuevo parámetro configurable `diff.max_expand_lines` (por defecto `800` líneas) en `config.jsonc`.
  - En archivos que superen este umbral (como dibujos Excalidraw o JSONs extensos), se renderizan únicamente los fragmentos modificados (*hunks* y líneas de contexto circundantes), evitando la saturación del hilo principal y la creación innecesaria de decenas de miles de nodos DOM reactivos.
  - La expansión o colapso completo permanece disponible bajo demanda mediante los controles de la barra de herramientas.
- **Restauración de la Acción Nativa de Escape en Excalidraw**:
  - Al presionar <kbd>Escape</kbd>, se deseleccionan completamente todos los elementos y grupos seleccionados, cancelando herramientas de dibujo activas y retornando a la herramienta de selección (`selection`), replicando fielmente la experiencia de Excalidraw web.
  - Preservación inmediata del foco en el canvas interactivo `.excalidraw` para permitir que atajos directos posteriores de una sola tecla sigan respondiendo sin requerir clics adicionales.
- **Protección de Guardado Manual y Manejo de Candado en Archivos Bloqueados**:
  - Se corrigió el comportamiento al presionar <kbd>Ctrl+S</kbd> o pulsar el botón de guardar en notas o diagramas bloqueados con candado: ahora se solicita confirmación explícita mediante un diálogo dedicado en vez de sobreescribir silenciosamente en disco.
  - Al descartar cambios en el diálogo de archivo bloqueado, se restaura en memoria el contenido original guardado en disco (`savedContent`).
  - El campo de título superior del editor se bloquea como de solo lectura (`readonly`) cuando el panel no está en modo de edición o el archivo está bloqueado.
  - Normalización estricta de rutas de bloqueo (`normalizeLockPath`) para evitar discrepancias entre prefijos de ruta relativa (`./` vs rutas limpias).
- **Estabilidad y Prevención de Bloqueo de Edición en Excalidraw**:
  - Optimización en `ExcalidrawViewer` actualizando dinámicamente las propiedades del componente React (`viewModeEnabled`) para evitar desincronizaciones de herramientas internas o bloqueos del canvas tras pulsar <kbd>Esc</kbd>.
  - Prevención de ciclos de auto-guardado redundantes y actualización protegida del visor.

## [2026.10.26.01-alpha] - 2026-10-26

### Añadido

- **Renderizado Embebido de Dibujos Excalidraw en Markdown con Caché**:
  - Soporte completo para renderizar visualmente archivos de dibujo Excalidraw (`.excalidraw`, `.excalidraw.json`, `.excalidraw.md`) como gráficos vectoriales SVG directamente dentro de las notas Markdown.
  - Compatibilidad total con transclusiones de wikilinks (`![[dibujo.excalidraw]]`, `![[esquema.excalidraw.md|500]]`), sintaxis estándar de imagen (`![](./diagrama.excalidraw)`) y bloques de código dedicados (````excalidraw ...````) con soporte para directivas `{{#include ...}}`.
  - **Sistema de Caché Multinivel de Alto Rendimiento**:
    - **Nivel 1 (RAM)**: Caché en memoria LRU de 0 ms para visualización instantánea.
    - **Nivel 2 (Disco)**: Persistencia de SVGs prerenderizados en `<vault>/.synapse/cache/excalidraw/<hash>_<theme>.svg`. Lee el SVG en menos de 2 ms sin invocar React ni volver a calcular la geometría si el contenido no ha cambiado.
    - **Nivel 3 (Motor)**: Renderizado bajo demanda con `exportToSvg` adaptado al tema activo claro u oscuro de Synapse.
  - Integración transparente en el editor en vivo CodeMirror (sustitución interactiva al salir del cursor con botón de edición) y en la vista de lectura `MarkdownReadingView`.
- **Visualizador y Editor de Diagramas D2 (`.d2`) con WebAssembly (`@terrastruct/d2`)**:
  - Soporte completo para abrir, editar y previsualizar archivos de diagramas `.d2` con renderizado SVG nativo de alto rendimiento vía WASM.
  - Reconocimiento de la extensión `.d2` en la categoría `diagrams` tanto en el backend Rust (`file_types.rs`) como en la configuración (`config.jsonc`) y entidades frontend (`file-types.svelte.ts`).
  - **Configuración Declarativa In-File**: Soporte nativo del bloque `vars: { d2-config: { ... } }` para definir motor de layout (`dagre`, `elk`), tema (`theme-id`), tema oscuro (`dark-theme-id`), modo boceto (`sketch`) y padding directamente en el código fuente.
  - Detección automática del tema claro u oscuro de Synapse como fallback si el archivo no especifica un `theme-id`.
  - Integración en `WorkspacePage`, comparador de diff (`VisualDiffViewer`), visor de revisiones Git (`RevisionViewer`) y renderizado de bloques de código ````d2` en notas Markdown (tanto en lectura como en edición en vivo).
- **Refactorización Unificada `DiagramSplitViewer` (`src/shared/ui/diagram-viewer/`)**:
  - Extracción de componentes comunes de visualización de diagramas en un único componente base reutilizable, reduciendo más de 1,300 líneas de código duplicado entre `MermaidViewer`, `MermanViewer` y `D2Viewer`.
  - Lienzo SVG unificado con zoom y paneo interactivo sobre `viewBox`, motor gráfico de Puntero Láser sobre canvas 2D con estela y controles flotantes con `bits-ui`.
- **Autocompletado y Linter Sintáctico en Tiempo Real para D2 y Mermaid (`CodeEditor.svelte`)**:
  - **Autocompletado Contextual (`@codemirror/autocomplete`)**:
    - En **D2**: sugerencias para motores (`dagre`, `elk`), catálogo de 20 temas oficiales con nombre descriptivo (`0` Neutral, `3` Flagship, `200` Dark Mauve, `300` Terminal, `303` C4, etc.), formas geométricas (`shape:`), direcciones (`direction:`), estilos (`style.*`) y snippets de `d2-config`.
    - En **Mermaid**: sugerencias para directivas `%%{init: ...}%%`, frontmatter `config:`, catálogo de temas de Mermaid.js y Merman, `look` (`classic`, `handDrawn`), `layout` (`dagre`, `elk`), `curve` y tipos de diagrama (`flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `erDiagram`, `architecture-beta`, etc.).
  - **Linter y Diagnóstico en Línea (`@codemirror/lint` + `lintGutter`)**:
    - Subrayado rojo ondulado en la línea y columna exacta del error sintáctico devuelto por los parsers de D2 (rangos AST) y Mermaid (`err.hash.loc`), con marcadores en el margen de línea y tooltips informativos.
- **Atajo de Teclado para Alternar/Entrar en Modo Edición**:
  - Nuevo atajo de teclado por defecto `Ctrl+E` (`toggle_edit_mode`) para alternar instantáneamente entre el modo lectura y el modo edición en el visor Markdown activo sin necesidad de hacer clic con el ratón.
  - Registro del atajo en la configuración global y en el mapa de atajos (`shortcuts.svelte.ts`).
- **Persistencia Global del Modo Vim**:
  - El estado del modo Vim (`vimMode`) ahora se almacena y recuerda permanentemente a través de `localStorage` y la configuración de la aplicación (`appSettings.vimMode`), manteniéndose activo al recargar o reiniciar Synapse.
- **Sincronización Bidireccional entre Modales de Búsqueda**:
  - Al presionar `Ctrl+Shift+F` teniendo abierto `QuickOpen` (`Ctrl+O`), el texto introducido se transfiere automáticamente a `FullTextSearch` y la ventana de `QuickOpen` se cierra.
  - De forma recíproca, al pulsar `Ctrl+O` desde `FullTextSearch`, la consulta se traslada instantáneamente a `QuickOpen` cerrando el diálogo de búsqueda full-text.
- **Autocompletado de Wikilinks (`[[...]]`)**:
  - Ventana emergente (*popup*) en tiempo real al escribir `[[` en el editor CodeMirror con sugerencias contextuales de notas de la bóveda.
  - Formateo limpio: la lista desplegable muestra únicamente el nombre base del archivo sin rutas de directorios ni extensión `.md`.
  - Ampliación de dimensiones visuales del menú emergente (`.cm-tooltip-autocomplete`) a 460px de ancho y hasta 420px de altura con scrolling suave.
- **Knowledge Graph 2D y 3D con Aceleración WebGL**:
  - Visualizador de grafo de conocimiento bidireccional local y global con simulación de fuerzas (*force-directed*).
  - Modo 3D WebGL impulsado por Three.js con botón conmutador 2D/3D accesible en la cabecera del visor de grafos.
  - Soporte de nodos fantasma (*ghost nodes*) para wikilinks referenciados pendientes de creación.
  - Paralelización multi-hilo con Rayon en Rust para análisis y cálculo ultra-rápido de enlaces y física de grafos en bóvedas masivas.
- **Explorador de Etiquetas e Inspector de Relaciones**:
  - Vista lateral `TagsExplorer` para explorar, contar y filtrar notas rápidamente según sus etiquetas (`#etiqueta`).
  - Panel `RelationsInspector` para auditar backlinks, enlaces salientes e hijos/padres de la nota activa.
- **Servidor MCP Local (Model Context Protocol)**:
  - Servidor stdio MCP integrado en Rust (`synapse-mcp`) para conectar agentes de IA locales con la bóveda de forma dedicada desde CLI (`synapsers --mcp`).
  - **Servidor MCP en Proceso (`in-process`) con Transporte HTTP**: Soporte para ejecutar el servidor MCP en segundo plano en la misma instancia activa de escritorio de Synapse (`http://127.0.0.1:27182/mcp`), vinculándose dinámicamente a la bóveda abierta en pantalla y compartiendo en memoria el índice de búsqueda Tantivy.
  - **Indicador e Interruptor en `StatusBar`**: Botón interactivo en la barra de estado inferior que muestra el estado en vivo (`MCP ON :27182` / `MCP OFF`), permite activar o apagar el servidor con un clic y expone información de la bóveda activa y peticiones atendidas vía tooltip.
  - **Comandos en Paleta (`Ctrl+P`)**: Acciones rápidas para alternar el servidor MCP local y copiar la URL de conexión al portapapeles.
  - **Persistencia de Estado**: Opción recordada en `localStorage` (`appSettings.mcpServerEnabled`) para restaurar automáticamente el servidor encendido al reabrir la app.
  - Herramientas MCP avanzadas para búsqueda full-text, inspección de relaciones/backlinks, lectura de notas, listado de contenido de carpetas (`list_folder_contents`), listado de notas vecinas en la misma carpeta (`list_sibling_notes`), movimiento en lote de notas entre carpetas (`move_notes`) y estado Git.
- **Soporte de Frontmatter YAML según OKF**:
  - Parseo, indexación y preservación bidireccional de metadatos YAML frontmatter alineados con las especificaciones Open Knowledge Format (OKF) y herramientas de IA.

### Corregido

- **Fluidez de Navegación por Teclado, Rendimiento de Búsqueda y Corrección de Bucles Reactivos**:
  - **Corrección de `effect_update_depth_exceeded`**: Se eliminaron los ciclos de actualización infinita en `QuickOpen.svelte` y `WorkspacePage.svelte` aislando las mutaciones de estado reactivo mediante `untrack()` de Svelte 5 y guardas de cambio previo en los bloques `$effect` (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **Precompilación de Atajos `O(1)`**: En `shortcuts.svelte.ts`, se implementó un caché de combinaciones parseadas (`comboCache`) para evitar ejecutar `.split(',')`, `.split('+')` y `.toLowerCase()` en cada pulsación de tecla.
  - **Fast-Reject en Despachador de Teclado**: En `WorkspacePage.svelte`, se añadió una comprobación inmediata en `O(1)` que ignora teclas sin modificadores (`Ctrl`/`Meta`/`Alt`/`F12`), evitando evaluar las 19 comprobaciones de atajos al navegar con flechas o escribir texto.
  - **Aislamiento y Centralización de Listeners**: Se eliminó el listener redundante de `window.addEventListener('keydown')` en `CommandPalette.svelte`, centralizando los atajos globales en `WorkspacePage`.
  - **Optimización de Renderizado en WebKitGTK**: Se reemplazó el costoso `backdrop-filter: blur(10px)` de pantalla completa por fondos sólidos semitransparentes en `QuickOpen.svelte`, `CommandPalette.svelte` y `FullTextSearch.svelte`, suprimiendo la `transition` en filas de resultados y añadiendo `contain: layout style paint` para cambios de fila instantáneos (1 solo frame).
  - **Búsqueda Indexada en Fallback de QuickOpen**: Se indexaron los `vaultItems` en un `Map` para resolver búsquedas en $O(1)$ en lugar de ejecutar `.find()` en $O(N)$ repetidos.

- **Backend Rust Asíncrono fuera del Hilo Principal de GTK**:
  - Se convirtieron a `pub async fn` y `tauri::async_runtime::spawn_blocking` los comandos Tauri bloqueantes que operaban en el hilo principal de la UI: `get_system_theme` (detección del SO vía `busctl`/`gsettings`), `search_notes_command` (búsqueda difusa en Nucleo), `get_vault_git_status` (verificación de estado de Git en foco de ventana) y comandos de gestión de espacios de trabajo (`workspace.rs`).
  - Se derivó `Clone` en `GitUseCases` y `GitService` respetando la Arquitectura Cebolla.

- **Limpieza de Errores Falsos en Consola para Plugins de Comunidad**:
  - Se silenciaron los mensajes de error en `TauriVaultRepository.readNote` para archivos de configuración opcionales (`.synapse/` y `.obsidian/`) que no existen en bóvedas recién abiertas.
  - Se corrigió el valor por defecto de `getEnabledPlugins()` para retornar una lista vacía `[]` si la bóveda no tiene plugins comunitarios configurados ni instalados, evitando intentos de carga innecesarios.

- **Prevención de Recarga Redundante y Bloqueo de Edición en Excalidraw al presionar Esc**:
  - Se corrigió un bug en `ExcalidrawViewer` donde al pulsar <kbd>Esc</kbd> o producirse una deselección con pérdida de foco (`focusout`), la serialización y guardado del archivo desencadenaba una actualización externa reactiva que sobreescribía la escena de Excalidraw (`updateScene`). Esto provocaba la pérdida del puntero interactivo del canvas y requería alternar a modo lectura y volver a edición para reactivar el dibujo.
  - Implementación de discriminación de cambios locales (`isLocalChange`) y comparación semántica con hash de versión de escena (`hasSceneDiverged` utilizando `getSceneVersion`) para impedir reescrituras de la escena viva generadas por el propio visor.
  - Restauración automática y garantizada de la herramienta de selección (`setActiveTool({ type: 'selection' })`) tras la captura del evento `Escape`.
- **Foco Automático en el Editor al Reabrir o Alternar Notas**:
  - Al abrir o reabrir una nota que quedó en modo edición, o alternar entre pestañas editables, el editor CodeMirror recibe el foco de entrada automáticamente (`focus()`), permitiendo escribir o desplazarse inmediatamente con las teclas sin necesidad de hacer clic manual previo.
- **Precisión y Ranking en Búsqueda Difusa (`search_with_stats`)**:
  - Corrección del algoritmo de ranking en el motor de navegación Nucleo: se otorgan prioridades y bonificaciones estrictas a coincidencias exactas y subcadenas completas en nombres de archivo y títulos, eliminando resultados espurios irrelevantes al buscar palabras en minúsculas comunes (ej. `todo`).
- **Respuesta Instantánea de QuickOpen (`Ctrl+O`)**:
  - Eliminación de listeners redundantes y configuración de `forceMount={true}` en los elementos de `Command.Item` para evitar que la lista se quede en blanco.
  - Retorno en memoria en 0 ms si la búsqueda en backend no ha completado la primera respuesta.
- **Aislamiento de Pestañas Sintéticas de Grafos**:
  - Se corrigió la asociación de rutas para evitar que URIs sintéticas como `synapse://graph` sean identificadas erróneamente como notas Markdown vacías.
  - Preservación de la nota activa previa (`lastActiveDocumentPath`) para centrar correctamente el grafo local incluso en notas aisladas sin enlaces.
- **Alineación Visual en Búsqueda Full-Text**:
  - Corrección de la posición y alineación del icono de lupa dentro de `FullTextSearch` y extracción del componente reutilizable `SearchBox`.
- **Prevención de Deadlock al Abrir o Alternar Bóvedas**:
  - Se corrigió un bloqueo de hilos (*mutex deadlock*) en `activate_vault` donde el bloqueo de `active_vault_path` y la notificación sincrónica del evento `"mcp:status-changed"` a través de `app_handle` competían por los recursos en Tauri, causando que la ventana se congelara al seleccionar una bóveda. La emisión de eventos MCP ahora se despacha desacoplada en segundo plano tras liberar los locks.

### Cambiado

- **Refactorización de Arquitectura Backend y Frontend**:
  - Descomposición modular de comandos Tauri en `session`, `file_ops`, `assets`, `search`, `git` y `workspace` respetando estrictamente la Arquitectura Cebolla.
  - Purificación del servicio de dominio `link_resolution` desacoplándolo de capas externas.
  - Reorganización del frontend bajo principios canónicos de Feature-Sliced Design (FSD), extrayendo diálogos modulares (`VaultDeleteDialog`, `VaultRestoreDialog`, `GitManagementDialogs`), APIs canónicas y modelos composables con Runes de Svelte 5.

## [2026.10.24.02-alpha] - 2026-10-24

### Corregido

- **Fluidez de Navegación por Teclado, Rendimiento de Búsqueda y Corrección de Bucles Reactivos**:
  - **Corrección de `effect_update_depth_exceeded`**: Se eliminaron los ciclos de actualización infinita en `QuickOpen.svelte` y `WorkspacePage.svelte` aislando las mutaciones de estado reactivo mediante `untrack()` de Svelte 5 y guardas de cambio previo en los bloques `$effect` (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **Precompilación de Atajos `O(1)`**: En `shortcuts.svelte.ts`, se implementó un caché de combinaciones parseadas (`comboCache`) para evitar ejecutar `.split(',')`, `.split('+')` y `.toLowerCase()` en cada pulsación de tecla.
  - **Fast-Reject en Despachador de Teclado**: En `WorkspacePage.svelte`, se añadió una comprobación inmediata en `O(1)` que ignora teclas sin modificadores (`Ctrl`/`Meta`/`Alt`/`F12`), evitando evaluar las 19 comprobaciones de atajos al navegar con flechas o escribir texto.
  - **Aislamiento y Centralización de Listeners**: Se eliminó el listener redundante de `window.addEventListener('keydown')` en `CommandPalette.svelte`, centralizando los atajos globales en `WorkspacePage`.
  - **Optimización de Renderizado en WebKitGTK**: Se reemplazó el costoso `backdrop-filter: blur(10px)` de pantalla completa por fondos sólidos semitransparentes en `QuickOpen.svelte`, `CommandPalette.svelte` y `FullTextSearch.svelte`, suprimiendo la `transition` en filas de resultados y añadiendo `contain: layout style paint` para cambios de fila instantáneos (1 solo frame).
  - **Búsqueda Indexada en Fallback de QuickOpen**: Se indexaron los `vaultItems` en un `Map` para resolver búsquedas en $O(1)$ en lugar de ejecutar `.find()` en $O(N)$ repetidos.

- **Backend Rust Asíncrono fuera del Hilo Principal de GTK**:
  - Se convirtieron a `pub async fn` y `tauri::async_runtime::spawn_blocking` los comandos Tauri bloqueantes que operaban en el hilo principal de la UI: `get_system_theme` (detección del SO vía `busctl`/`gsettings`), `search_notes_command` (búsqueda difusa en Nucleo), `get_vault_git_status` (verificación de estado de Git en foco de ventana) y comandos de gestión de espacios de trabajo (`workspace.rs`).
  - Se derivó `Clone` en `GitUseCases` y `GitService` respetando la Arquitectura Cebolla.

- **Limpieza de Errores Falsos en Consola para Plugins de Comunidad**:
  - Se silenciaron los mensajes de error en `TauriVaultRepository.readNote` para archivos de configuración opcionales (`.synapse/` y `.obsidian/`) que no existen en bóvedas recién abiertas.
  - Se corrigió el valor por defecto de `getEnabledPlugins()` para retornar una lista vacía `[]` si la bóveda no tiene plugins comunitarios configurados ni instalados, evitando intentos de carga innecesarios.

- **Auto-guardado al perder foco y persistencia del modo edición en Excalidraw y Obsidian Excalidraw**:
  - Se implementó guardado automático instantáneo al perder foco (`onfocusout`, `onblur` y `<svelte:window onblur>`) tanto en el editor interactivo de Excalidraw como en el visor/editor de Obsidian Excalidraw.
  - Se corrigió el bug por el cual, al guardar o desenfocar la pestaña, el modo activo de edición (`isEditing`) se revertía a solo lectura (`reading`) obligando al usuario a volver a pulsar el botón de lectura/edición.
  - Sincronización reactiva del `viewMode` de las notas abiertas en memoria con el estado activo de edición del panel, evitando desajustes con el observador del sistema de archivos (`vault:files-changed`).
  - Salvaguarda automática del contenido no persistido al desmontar componentes (`onDestroy`) y al alternar entre la vista de lienzo y código fuente en Obsidian Excalidraw.

## [2026.10.24.01-alpha] - 2026-10-24

### Añadido

- **Bloqueo de Archivos contra Modificaciones Accidentales (*File Lock*)**:
  - Opción de bloquear y desbloquear archivos desde el menú contextual de `VaultExplorer`.
  - Iconos de candado visibles en el árbol de archivos, en las pestañas (`EditorHeader`) y en la barra superior.
  - Inhibición estricta de auto-guardado en archivos bloqueados con persistencia en `VaultUiState.locked_files`.
  - Diálogo de confirmación accesible al intentar cerrar pestañas o la aplicación con cambios pendientes en archivos bloqueados, ofreciendo opciones para abortar, descartar cambios o forzar el guardado.
- **Modo Lectura por Defecto en Archivos Excalidraw**:
  - Tanto los dibujos de Excalidraw nativo como los de Obsidian Excalidraw (`.excalidraw`, `.excalidraw.md`) ahora se abren por defecto en modo lectura, evitando mutaciones o disparos accidentales en memoria.
- **Acciones del Sistema en Menús Contextuales**:
  - **"Abrir con la app por defecto"**: Permite abrir cualquier archivo con la aplicación configurada por omisión en el sistema operativo (Word, Excel, etc.) tanto desde `VaultExplorer` como desde las pestañas.
  - **"Mostrar en carpeta"**: Abre el gestor de archivos nativo del sistema y resalta el archivo o carpeta seleccionada.
  - Implementación multiplataforma en Rust siguiendo la especificación estándar FreeDesktop D-Bus (`org.freedesktop.FileManager1.ShowItems` con `busctl`/`dbus-send` y fallbacks en Linux, `explorer /select` en Windows y `open -R` en macOS).
- **Control de Lectura y Escritura Local en Visor de Diff**:
  - En `VisualDiffViewer`, el panel de trabajo local ahora inicia en modo lectura por defecto con un interruptor para alternar entre modo lectura y escritura exclusiva para el archivo local en disco, mientras que el panel de revisión Git permanece estrictamente en modo solo lectura.
- **Soporte de Imágenes y Archivos Incrustados en Diagramas Excalidraw**:
  - Reconocimiento y parseo bidireccional de la sección `## Embedded Files` en archivos Markdown de Obsidian Excalidraw (`.excalidraw.md`).
  - Resolución automática de rutas relativas y wikilinks (`fileId: [[ruta/a/imagen.png]]`) a URLs de datos (`dataURL`) leídas desde la bóveda activa.
  - Inyección dinámica en el registro de archivos del lienzo tanto en el visor de Excalidraw nativo (`api.addFiles`) como en el generador vectorial SVG de `ObsidianExcalidrawViewer` (`ExcalidrawAutomate.toSVG`), permitiendo la correcta visualización de imágenes y gráficos incrustados.
  - Preservación íntegra de la sección `## Embedded Files` durante el guardado y serialización de archivos.

### Corregido

- **Fluidez de Navegación por Teclado, Rendimiento de Búsqueda y Corrección de Bucles Reactivos**:
  - **Corrección de `effect_update_depth_exceeded`**: Se eliminaron los ciclos de actualización infinita en `QuickOpen.svelte` y `WorkspacePage.svelte` aislando las mutaciones de estado reactivo mediante `untrack()` de Svelte 5 y guardas de cambio previo en los bloques `$effect` (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **Precompilación de Atajos `O(1)`**: En `shortcuts.svelte.ts`, se implementó un caché de combinaciones parseadas (`comboCache`) para evitar ejecutar `.split(',')`, `.split('+')` y `.toLowerCase()` en cada pulsación de tecla.
  - **Fast-Reject en Despachador de Teclado**: En `WorkspacePage.svelte`, se añadió una comprobación inmediata en `O(1)` que ignora teclas sin modificadores (`Ctrl`/`Meta`/`Alt`/`F12`), evitando evaluar las 19 comprobaciones de atajos al navegar con flechas o escribir texto.
  - **Aislamiento y Centralización de Listeners**: Se eliminó el listener redundante de `window.addEventListener('keydown')` en `CommandPalette.svelte`, centralizando los atajos globales en `WorkspacePage`.
  - **Optimización de Renderizado en WebKitGTK**: Se reemplazó el costoso `backdrop-filter: blur(10px)` de pantalla completa por fondos sólidos semitransparentes en `QuickOpen.svelte`, `CommandPalette.svelte` y `FullTextSearch.svelte`, suprimiendo la `transition` en filas de resultados y añadiendo `contain: layout style paint` para cambios de fila instantáneos (1 solo frame).
  - **Búsqueda Indexada en Fallback de QuickOpen**: Se indexaron los `vaultItems` en un `Map` para resolver búsquedas en $O(1)$ en lugar de ejecutar `.find()` en $O(N)$ repetidos.

- **Backend Rust Asíncrono fuera del Hilo Principal de GTK**:
  - Se convirtieron a `pub async fn` y `tauri::async_runtime::spawn_blocking` los comandos Tauri bloqueantes que operaban en el hilo principal de la UI: `get_system_theme` (detección del SO vía `busctl`/`gsettings`), `search_notes_command` (búsqueda difusa en Nucleo), `get_vault_git_status` (verificación de estado de Git en foco de ventana) y comandos de gestión de espacios de trabajo (`workspace.rs`).
  - Se derivó `Clone` en `GitUseCases` y `GitService` respetando la Arquitectura Cebolla.

- **Limpieza de Errores Falsos en Consola para Plugins de Comunidad**:
  - Se silenciaron los mensajes de error en `TauriVaultRepository.readNote` para archivos de configuración opcionales (`.synapse/` y `.obsidian/`) que no existen en bóvedas recién abiertas.
  - Se corrigió el valor por defecto de `getEnabledPlugins()` para retornar una lista vacía `[]` si la bóveda no tiene plugins comunitarios configurados ni instalados, evitando intentos de carga innecesarios.

- **Prevención de Congelamiento y Optimización de Memoria en Obsidian Excalidraw**:
  - Se eliminó el filtro CSS `drop-shadow` en el contenedor SVG del visor de Obsidian Excalidraw, evitando que WebKitGTK asigne búfers rasterizados masivos de varios gigabytes (hasta 4 GB) para diagramas vectoriales extensos.
  - Se configuró el SVG generado con `width="100%" height="100%"` acoplado a su `viewBox`, permitiendo un renderizado vectorial escalado por GPU sin límites fijos hipertrofiados.
  - Se implementó un debounce de 300 ms en el evento `onChange` del editor nativo de Excalidraw para evitar ejecutar la compresión LZ de miles de elementos en cada micro-movimiento del puntero.
  - Se protegieron las imágenes cargadas para no persistir URLs de datos o activos en el bloque de archivos del JSON en documentos Markdown, respetando el formato estándar de `## Embedded Files`.
- **Apertura de Diagramas Excalidraw en Modo Lectura por Defecto**:
  - Detección precisa de archivos de dibujo a través de su metadata YAML frontmatter (`--- \n excalidraw-plugin: parsed \n ---`) independientemente del nombre o extensión `.md`.
  - Los diagramas se abren por omisión en modo lectura (`readOnly` y botón en modo "Lectura") para evitar modificaciones accidentales y consumo innecesario de recursos.

- **Persistencia del Modo Edición durante el Auto-guardado en Excalidraw**:
  - Se corrigió un bug donde, al guardar automáticamente un dibujo abierto en Excalidraw, la recarga del archivo y la actualización interna de escena revertían el visor al modo lectura o dejaban la herramienta activa en modo mano (`hand`), impidiendo continuar editando sin alternar el botón de la cabecera.
- **Protección Estricta contra Guardado en Archivos Bloqueados**:
  - Se corrigió un bug donde `persistVaultItemToRust` persistía archivos bloqueados al cerrar la ventana, desmontar componentes o disparar callbacks automáticos de visores. Ahora se exige `force = true` explícito (únicamente activable vía diálogo de confirmación, atajo `Ctrl+S` o botón Guardar manual).

## [2026.10.23.04-alpha] - 2026-10-23

### Añadido

- **Plugins Comunitarios de Obsidian** (SOLO SE VALIDO EL PLUGIN DE EXCALIDRAW DE OBSIDIAN):
  - Explorador e instalador integrado de plugins comunitarios desde el catálogo oficial de Obsidian.
  - Soporte de instalación, habilitación, deshabilitación, desinstalación y ejecución en entorno seguro.
  - Caché de doble nivel (RAM y disco) con TTL para navegación instantánea del catálogo sin saturar la red.
  - Detección automática de plugins instalados mediante escaneo directo en disco del vault.
  - Comandos y soporte en interfaz para verificar actualizaciones y actualizar plugins automáticamente.
- **Búsqueda Full-Text en Documentos PDF y Word (DOCX)**:
  - Extracción e indexación automática de texto para archivos `.pdf` y `.docx` en el motor Tantivy en Rust.
  - Configuración detallada en `config.jsonc` (`indexing`): límites de tamaño (`max_pdf_file_size`, `max_docx_file_size`), límite de páginas (`max_pdf_pages`), límite de caracteres (`max_extracted_chars`) y filtrado de palabras corruptas (`max_word_length`).
- **Navegación Fluida y Expansión desde Breadcrumb**:
  - Al hacer clic en un directorio dentro del `BreadCrumb` de una nota activa, `VaultExplorer` se expande y se muestra automáticamente si la barra lateral estaba colapsada u oculta.
  - La carpeta seleccionada en el breadcrumb despliega inmediatamente sus subdirectorios y archivos en el árbol del explorador.
- **Regiones de Arrastre de Ventana en Windows y Sistemas de Escritorio**:
  - Se habilitaron regiones de arrastre nativas de Tauri (`data-tauri-drag-region`) en las zonas vacías de la cabecera de pestañas (`EditorHeader`) y en la cabecera del explorador (`VaultExplorer`).
  - Espaciador de arrastre dedicado de 24px junto a los controles de ventana para garantizar un punto de agarre cómodo incluso con decenas de pestañas abiertas.
  - Soporte para maximizar y restaurar la ventana mediante doble clic en las áreas libres de la cabecera sin interferir con las pestañas ni los botones interactivos.

## [2026.10.23.03-alpha] - 2026-10-23

### Añadido

- Explorador, instalador y gestor de plugins comunitarios de Obsidian con optimizaciones de rendimiento y caché.

## [0.1.0-alpha] - 2026-10-23

### Primer Release Alpha

**Synapse.rs** es una aplicación de gestión de notas, conocimiento y diagramas hiper-rápida, privada y *offline-first*, construida con **Tauri v2**, **Rust** y **Svelte 5**. Diseñada para ser liviana, extensible y compatible con el ecosistema de Markdown y Obsidian.

### Añadido

- **Velocidad Extrema en Rust**: Indexación y navegación instantánea con búsqueda difusa en memoria (*Nucleo fuzzy matching*).
- **Editor Markdown y Diagramas**: Soporte nativo para Markdown, diagramas Mermaid, Merman y dibujos Excalidraw (`.excalidraw.md`).
- **Control de Versiones Git Integrado**: Historial de cambios, diff visual por archivo, branching y stashing directo desde la app.
- **Modo Archivo Puntual (*Standalone*)**: Abre y edita archivos individuales (`.md`, `.mermaid`, `.excalidraw`) directamente sin necesidad de crear un espacio de trabajo.
- **Soporte de Formatos**: Vista previa de imágenes, documentos PDF y acceso rápido a archivos externos (Word, Excel) con tu visor de sistema preferido.
- **Tema Dinámico**: Modo Oscuro, Claro y sincronización automática con el sistema operativo.
- **100% Privado y Local**: Tus notas nunca salen de tu máquina; tus archivos son simplemente carpetas en tu disco.

### Descargas portables (Sin instalador)

No requieren instalación de dependencias pesadas; simplemente descarga y ejecuta:

| Plataforma | Archivo | Instrucciones |
| :--- | :--- | :--- |
| **Linux (x64)** | `synapsers` | Dar permisos de ejecución: `chmod +x synapsers` y ejecutar `./synapsers` |
| **Windows (x64)** | `synapsers.exe` | Doble clic para ejecutar directamente |

> [!NOTE]
> Esta es una versión **Alpha**. Si encuentras algún comportamiento inesperado o tienes sugerencias, eres bienvenido a abrir un issue en el repositorio.

[1.0.0-beta.9]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.9
[1.0.0-beta.8]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.8
[1.0.0-beta.7]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.7
[1.0.0-beta.6]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.6
[1.0.0-beta.5]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.5
[1.0.0-beta.4]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.4
[1.0.0-beta.3]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.3
[1.0.0-beta.2]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.2
[1.0.0-beta.1]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.1
[1.0.0-alpha.5]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.5
[1.0.0-alpha.4]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.4
[1.0.0-alpha.3]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.3
[1.0.0-alpha.2]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.2
[1.0.0-alpha.1]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.1
[2026.10.27.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.04-alpha
[2026.10.27.03-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.03-alpha
[2026.10.27.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.02-alpha
[2026.10.27.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.01-alpha
[2026.10.26.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.26.01-alpha
[2026.10.24.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.02-alpha
[2026.10.24.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.01-alpha
[2026.10.23.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.04-alpha
[2026.10.23.03-alpha]: https://github.com/Miuler/synapse.rs/tree/2026.10.23.03-alpha
[0.1.0-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.02-alpha
