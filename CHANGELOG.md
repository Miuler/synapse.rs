# Registro de cambios

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2026.10.26.01-alpha] - 2026-10-26

### Añadido

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
  - Herramientas MCP avanzadas para búsqueda full-text, inspección de relaciones/backlinks, lectura de notas y estado Git.
- **Soporte de Frontmatter YAML según OKF**:
  - Parseo, indexación y preservación bidireccional de metadatos YAML frontmatter alineados con las especificaciones Open Knowledge Format (OKF) y herramientas de IA.

### Corregido

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

[2026.10.26.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.26.01-alpha
[2026.10.24.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.02-alpha
[2026.10.24.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.01-alpha
[2026.10.23.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.04-alpha
[2026.10.23.03-alpha]: https://github.com/Miuler/synapse.rs/tree/2026.10.23.03-alpha
[0.1.0-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.02-alpha
