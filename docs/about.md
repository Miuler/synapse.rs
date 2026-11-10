# Synapse.rs 🧠⚡

> **Gestión de notas, conocimiento personal y arquitectura visual.**  
> Desarrollado en **Rust (Tauri v2)** y **Svelte 5**.  
> *100% Local-first, sin telemetría y con rendimiento nativo.*

---

![Synapse.rs Interfaz](synapsers.png)

---

## 💡 ¿Qué es Synapse?

Synapse es un espacio de trabajo para pensar, escribir notas y modelar arquitecturas de software sin intermediarios ni dependencias en la nube.

La mayoría de herramientas modernas sufren del mismo problema: consumen cientos de megabytes de RAM por usar Electron o bloquean tus datos tras formatos propietarios y suscripciones en la nube. Synapse está construido sobre una premisa simple: **tus archivos son tuyos y la herramienta debe ser inmediata.**

Combina un backend en **Rust** (con Arquitectura Cebolla) y una interfaz en **Svelte 5** (Feature-Sliced Design). Se inicia en milisegundos, consume una fracción mínima de memoria y busca sobre decenas de miles de notas, diagramas y documentos a la velocidad del teclado.

---

## ⚡ Capacidades Principales

### 1. 🏎️ Arquitectura Ligera y Rendimiento
- **Arranque optimizado:** Montaje de pestañas bajo demanda, árbol del explorador virtualizado y comandos de lectura asíncronos fuera del hilo principal de interfaz.
- **Gestión eficiente de memoria:** Basado en Tauri v2 en vez de empaquetar un navegador Chromium completo por ventana. Tras abrir y recorrer vaults grandes, el backend libera la memoria del heap devuelta al sistema operativo (`malloc_trim`).
- **Indexación nativa en paralelo:** Indexa decenas de miles de archivos usando hilos nativos (`rayon` + `jwalk`) sin congelar la interfaz ni bloquear el editor.

### 2. 🛡️ Soberanía de Datos (100% Local)
- **Tus archivos se quedan en tu disco:** Las notas son archivos estándar (`.md`, `.excalidraw.md`, `.d2`, `.puml`, `.mmd`). Sin bases de datos opacas ni sincronizaciones obligatorias.
- **Totalmente funcional offline:** No requiere internet ni cuentas de terceros.
- **Cero telemetría:** Ningún dato, nota o diagrama sale de tu máquina.

### 3. 🔍 Búsqueda Rápida e Indexación Profunda
- **Quick Open (`Ctrl+O`):** Búsqueda difusa en memoria mediante Nucleo Matcher (< 2 ms de respuesta).
- **Filtros por categoría (`kind:`):** Acota búsquedas por tipo de recurso: `kind:pdf`, `kind:markdown`, `kind:d2`, `kind:mermaid`, `kind:excalidraw`, `kind:word`, `kind:excel` o `kind:code`.
- **Filtro de encabezados (`hs:` / `headings:`):** Busca secciones y subtítulos directamente desde `Ctrl+O` o la búsqueda global (`Ctrl+Shift+F`), mostrando el título indexado y el texto inmediato.
- **Búsqueda Full-Text (Tantivy):** Motor de texto completo en Rust estilo Lucene con ponderación en títulos y soporte para operadores y frases.
- **Indexación de documentos adjuntos:** Extrae e indexa el texto dentro de archivos `.pdf` y documentos de Word `.docx`.

### 4. 🎨 Notas y Diagramas como Código
- **Markdown fluido:** Editor híbrido (CodeMirror 6), alternancia rápida lectura/edición (`F2`), enlaces a encabezados exactos (`[[nota#Título]]`) e incrustación de secciones completas con transclusión (`![[nota#Título]]`).
- **Lienzo libre con Excalidraw:** Mapas mentales, esquemas y bocetos interactivos embebidos directamente en notas (`.excalidraw.md`).
- **Diagramas como código (Mermaid, D2 y PlantUML):**
  - **Mermaid & Merman:** Diagramas de flujo y secuencias con puntero láser para presentaciones técnicas.
  - **D2 Lang:** Sintaxis declarativa moderna para diagramas de arquitectura.
  - **PlantUML:** Soporte nativo (`.puml`, `.plantuml`) con servidor configurable y validación en línea.
  - **Render optimizado:** Planificador con debounce dinámico, `Ctrl+Enter` para forzar redibujado y monitor de tiempo de render en la barra de estado.
  - **Exportación:** Guarda diagramas en formatos **PNG** o **SVG** desde el menú contextual.

### 5. 🐙 Control de Versiones Git y Comparación Visual (Diff Multi-Modal)
- **Gestión de Git integrada:** Soporte para el flujo de trabajo de versiones (commits, historial, ramas, stage/unstage, push con `Ctrl+Shift+K`) combinando lectura de estado con `gix` y comandos Git del sistema.
- **Comparación entre archivos arbitrarios:** Selecciona dos archivos cualesquiera en el explorador y compáralos al instante.
- **Diff de imágenes binarias:**
  - *2 Paneles:* Vista lado a lado entre disco local y la versión en Git con zoom independiente.
  - *Píxel a Píxel:* Resaltado visual en magenta de cada píxel alterado con porcentaje y conteo exacto de diferencias.
- **Diff dual para diagramas (Excalidraw, D2, Mermaid, PlantUML):**
  - *Como Diagrama:* Visualiza ambos estados en paralelo con la posibilidad de alternar el panel local a modo *Escritura* para editar sobre la marcha.
  - *Como Texto:* Inspecciona diferencias línea por línea en el código fuente o estructura JSON.

### 6. 🤖 Servidor MCP Integrado (Model Context Protocol)
- **Conexión nativa para agentes de IA:** Permite a clientes como Claude Code, Google Antigravity, Cursor, Windsurf o Roo Code consultar tu vault como servidor MCP local.
- **Concurrencia Optimista (OCC):** Modificación segura mediante `expected_hash` u `overwrite` explícito para evitar sobreescrituras accidentales.
- **Ahorro de contexto y tokens:** Búsqueda jerárquica por tags (`#linux/cachyos`), lectura del esquema de títulos (`outline_only: true`) y lectura quirúrgica por secciones (`section`).
- **Persistencia por vault:** Recuerda la activación de MCP por bóveda y la reactiva automáticamente.

### 7. ⌨️ Flujo de Trabajo y Teclado
- **Modo Vim:** Activación instantánea con soporte de comandos, búsqueda (`/`), rehacer (`Ctrl+R`) y plegado por niveles (`zc`, `zo`, `za`, `zM`, `zR`).
- **Autocompletado del documento:** Sugerencias automáticas basadas en palabras ya escritas en la nota o diagrama activo.
- **Zoom de texto:** Ajuste rápido de tamaño con `Ctrl+Rueda`, `Ctrl++`, `Ctrl+-` y `Ctrl+0`.
- **Atajos configurables:** Personalización de combinaciones mediante `shortcuts.jsonc` (conservando comentarios).
- **Modo Archivo Único:** Abre y edita archivos individuales `.md`, `.d2` o `.excalidraw` sin necesidad de crear un workspace.
- **Plugins de Obsidian:** Compatibilidad con extensiones comunitarias verificadas a través de una fachada de su API.

---

## 📊 Comparativa

| Aspecto | **Synapse.rs** 🧠 | **Obsidian** 💎 | **Notion** ☁️ | **VS Code** 💻 |
| :--- | :---: | :---: | :---: | :---: |
| **Arquitectura de ejecución** | **Rust + Tauri v2 (Webview nativo)** | Electron (Chromium empaquetado) | Web / Electron | Electron (Chromium empaquetado) |
| **Soberanía de archivos** | **100% Local / Formatos abiertos** | 100% Local | En la nube (Propietario) | 100% Local |
| **Búsqueda en PDFs / DOCX** | **Nativa en Rust (Tantivy)** | Requiere Plugins | En la nube | Requiere extensiones |
| **Filtros (`kind:`, `hs:`)** | **Nativos en Quick Open y FTS** | Plugins de terceros | No | Extensiones |
| **Diagramas D2, Mermaid, PlantUML** | **Nativos con exportación PNG/SVG** | Plugins de terceros | Básico | Extensiones |
| **Lienzo Excalidraw** | **Nativo (.excalidraw.md)** | Vía plugin | No | Extensiones |
| **Control de versiones Git** | **Integrado en UI (`gix` + Git)** | Requiere Git CLI | No | Requiere Git CLI |
| **Diff visual de diagramas e imágenes** | **Nativo (Lado a lado, Píxel a píxel)** | No | No | Extensiones |
| **Servidor MCP para IA** | **Nativo (18 herramientas, OCC)** | Plugins de terceros | No | Extensiones |
| **Portabilidad** | **Binario único sin instalación** | Instalador / AppImage | Instalador | Instalador |

---

## 🔍 Comparación Visual en Acción

### Diff de Imágenes (Lado a Lado vs. Píxel a Píxel)

Panel a panel con controles de encuadre y zoom:

![Diff Binario de Imagen en 2 Paneles](arquitectura_diff__binario_2_paneles.png)

Detección automática de cambios resaltando los píxeles modificados en magenta:

![Diff Binario Píxel a Píxel](arquitectura_diff__binario_pixel_x_pixel.png)

---

### Diff de Diagramas (Como Diagrama vs. Como Texto)

Comparación visual interactiva en dos paneles con modo de edición local:

![Diff de Excalidraw como Diagrama](arquitectura_diff__excalidraw_as_diagram.png)

Vista de código para auditar cambios a nivel de texto o estructura JSON:

![Diff de Excalidraw como Texto](arquitectura_diff__excalidraw_as_text.png)

---

## 🏛️ Arquitectura Técnica

Synapse sigue una estricta separación de responsabilidades para garantizar mantenibilidad, velocidad y estabilidad a largo plazo:

- **Frontend (Svelte 5 + Bun):** Diseñado con **Feature-Sliced Design (FSD)** en capas unidireccionales (`app` ➔ `pages` ➔ `widgets` ➔ `features` ➔ `entities` ➔ `shared`).
- **Backend (Rust + Tauri v2):** Construido con **Arquitectura Cebolla**. El núcleo (`domain/`) no contiene frameworks ni I/O; la capa de orquestación (`application/`) contiene 48 casos de uso atómicos, y los adaptadores técnicos (`infrastructure/`) implementan los contratos (persistencia en disco, Tantivy, Nucleo, Gix, servidor MCP).

![Mapa de Arquitectura Global de Synapse.rs](arquitectura.png)

---

## 📦 Distribución

- **Binario portable:** Un único archivo ejecutable (`synapsers` en Linux / `synapsers.exe` en Windows).
- **Sin instaladores obligatorios:** No requiere dependencias del sistema ni permisos administrativos para ejecutarse.
