# Registro de cambios

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[2026.10.27.03-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.03-alpha
[2026.10.27.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.02-alpha
[2026.10.27.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.01-alpha
[2026.10.26.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.26.01-alpha
[2026.10.24.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.02-alpha
[2026.10.24.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.01-alpha
[2026.10.23.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.04-alpha
[2026.10.23.03-alpha]: https://github.com/Miuler/synapse.rs/tree/2026.10.23.03-alpha
[0.1.0-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.02-alpha
