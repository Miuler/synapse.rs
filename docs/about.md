# Synapse.rs 🧠⚡

> **Tu segundo cerebro a la velocidad de la luz.**  
> Gestión de conocimiento personal, notas y arquitectura visual desarrollada en **Rust (Tauri v2)** y **Svelte 5**.  
> *100% Local-First. Zero-Telemetry. Rendimiento Nativo.*

---

## 🌟 La Visión: Redefiniendo el Conocimiento Personal

En un mundo saturado de herramientas lentas basadas en Electron o plataformas en la nube que encierran tus ideas tras suscripciones mensuales y muros de datos (*vendor lock-in*), **Synapse** nace como una declaración de principios:

> **Tus pensamientos merecen la velocidad del metal y la soberanía absoluta de tus archivos.**

Synapse combina la potencia, seguridad de memoria y velocidad extrema de **Rust** con la reactividad y elegancia de **Svelte 5 (Runes)**. El resultado es un entorno de trabajo cognitivo donde la fricción desaparece: abre vaults masivos en milisegundos, dibuja conceptos complejos al instante y localiza cualquier pensamiento en tus notas, diagramas, libros PDF o documentos Word a la velocidad del pensamiento.

---

## 🚀 ¿Por qué Synapse? Argumentos de Venta y Propuesta de Valor

### 1. 🏎️ Rendimiento Descomunal: Adiós al "Bloatware"
- **Inicio en milisegundos:** Olvídate de pantallas de carga eternas. Synapse se inicia al instante.
- **Consumo mínimo de RAM:** Construido sobre **Tauri v2** en lugar de empaquetar un navegador Chromium completo por ventana. Synapse consume una fracción insignificante de la memoria que exigen herramientas como Obsidian, Notion o Logseq.
- **Motor de indexación en Rust:** Indexación paralela con hilos nativos (`rayon` + `jwalk`), procesando decenas de miles de archivos en segundos sin congelar la interfaz.

### 2. 🛡️ Soberanía de Datos y Privacidad Radical (Local-First)
- **Tus archivos son tuyos:** Tus notas son simples archivos en tu disco duro (Markdown estándar, Mermaid, Excalidraw, imágenes). Sin formatos propietarios ni bases de datos opacas.
- **100% Offline:** Funciona en un avión, en la montaña o en una red aislada. No requiere conexión a internet ni autenticación en servidores de terceros.
- **Cero Telemetría:** Nadie analiza tus notas para entrenar modelos sin tu permiso ni monetizar tu atención.

### 3. 🔍 Búsqueda Cognitiva de Nivel Industrial (Tantivy + Nucleo)
- **Búsqueda difusa en memoria (Nucleo Fuzzy Matcher):** El mismo motor de búsqueda interactiva ultrarrápida usado en los editores de terminal más modernos, encontrando cualquier archivo en tu vault en menos de 2 ms conforme escribes.
- **Motor Full-Text estilo Lucene en Rust (Tantivy):** Busca dentro del contenido real de tus notas, pero además...
- **¡Indexación de adjuntos (PDFs y Word DOCX)!:** Synapse extrae e indexa de forma inteligente el texto de tus libros, artículos académicos (`.pdf`) y documentos de oficina (`.docx`). Tu vault se convierte en un archivo vivo donde nada se pierde.

### 4. 🎨 Tríada de Pensamiento Visual Integrada
- **Markdown Enriquecido:** Editor híbrido fluido con soporte CodeMirror 6, renderizado dinámico y modo de lectura/edición.
- **Diagramas Mermaid & Merman con Puntero Láser:** Convierte texto en diagramas de flujo, secuencias y arquitecturas. Incluye un puntero láser interactivo diseñado para presentaciones en vivo y explicaciones de arquitectura.
- **Lienzos Infinitos con Excalidraw:** Bocetos a mano alzada, mapas mentales y prototipos embebidos directamente en tus notas (`.excalidraw.md`).

### 5. 🐙 Git Nativo en Puro Rust (Sin Dependencias Externas)
- **Control de versiones con `gix`:** Historial de revisiones, branching, estados de sincronización y visor visual de diferencias (*diff viewer*) sin necesidad de tener git instalado en el sistema operativo.

### 6. 🤖 Servidor MCP Integrado de Alto Rendimiento (Model Context Protocol)
- **Tu conocimiento al servicio de la IA:** Conecta agentes de IA (Claude Code, Google Antigravity, Cursor, Windsurf, Roo Code, etc.) directamente a tu bóveda personal como un servidor MCP nativo.
- **Búsqueda Cognitiva por Sub-tags Jerárquicos:** Los agentes pueden filtrar notas con etiquetas jerárquicas como `#linux/cachyos` buscando directamente por tags raíz o sub-etiquetas (`tags: ["cachyos"]`), obteniendo snippets inmediatos y conteo de tokens sin descargas innecesarias.
- **Lectura Eficiente y Ahorro Masivo de Tokens:** Soporte para lectura de tabla de contenidos (`outline_only: true`) calculada en tiempo real (< 0.5 ms en Rust) y extracción quirúrgica por sección (`section`). Los LLMs consumen una fracción de contexto y tiempo de respuesta sin desincronización de caché.
- **Edición Quirúrgica Tolerante y Concurrencia Optimista (OCC):** `patch_note` con verificación por hash criptográfico y anclaje difuso tolerante a espacios y saltos de línea (CRLF/LF), evitando colisiones y errores de edición entre el usuario y la IA.

### 7. 🔌 Ecosistema Extensible: Plugins Comunitarios de Obsidian
- **No empiezas de cero:** Explora, instala y ejecuta plugins directamente desde el catálogo oficial de la comunidad de Obsidian.
- **Caché inteligente de doble nivel:** Navegación fluida por el catálogo sin saturar la red ni bloquear el trabajo.

### 8. ⌨️ Diseñado para la Productividad de Teclado
- **Modo VIM nativo:** Actívalo y desactívalo al vuelo desde la barra de estado para una edición táctil y precisa.
- **Paleta de Comandos Unificada (Ctrl+P):** Controla toda la aplicación sin levantar las manos del teclado.
- **Modo Standalone:** ¿Solo quieres editar un archivo `.md` o `.excalidraw` suelto en tu disco? Ábrelo directamente sin verte forzado a crear un workspace.

---

## 📊 Tabla Comparativa: Synapse frente al Mercado

| Característica | **Synapse.rs** 🧠 | **Obsidian** 💎 | **Notion** ☁️ | **Logseq** 📝 | **VS Code** 💻 |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Tecnología Base** | **Rust + Tauri v2** | Electron | Electron / Web | Electron | Electron |
| **Consumo de Memoria** | **Ultra Bajo (~50-120 MB)** | Medio (~250-600 MB) | Muy Alto (~500 MB-1.5 GB) | Alto (~350-800 MB) | Alto (~400 MB-1 GB) |
| **Velocidad de Arranque** | **Instantánea (< 0.2s)** | 2 - 5s | Lenta (depende de red) | 3 - 6s | 2 - 4s |
| **Soberanía / Local-First** | **100% Local / Zero Lock-in** | Local-First | En la nube (Propietario) | Local-First | Local-First |
| **Búsqueda en PDFs y DOCX** | **Nativa en Rust (Tantivy)** | Requiere Plugins | Indexado en nube | Básico | Requiere Extensiones |
| **Diagramas Mermaid Interactivos** | **Nativo + Puntero Láser** | Básico (estático) | Básico | Básico | Con extensiones |
| **Lienzo Excalidraw Integrado** | **Nativo (.excalidraw.md)** | Vía Plugin | No | Vía Plugin | Con extensiones |
| **Git Nativo Integrado** | **Puro Rust (`gix`)** | Requiere Git CLI | No | Vía Git local | Requiere Git CLI |
| **Servidor MCP Nativo (AI Ready)** | **Sí (Sub-tags, OCC, Outline & Snippets)** | Requiere Plugins de terceros | No | No | Vía extensiones |
| **Modo VIM al vuelo** | **Sí (Toggle instantáneo)** | Sí (Configuración) | No | Experimental | Vía extensión |
| **Modo Archivo Único (Standalone)** | **Sí (Abre archivos sueltos)** | No (Exige Vault) | No | No (Exige Grafo) | Sí |
| **Distribución Portable** | **Binario Único sin instalador** | Instalador/AppImage | Instalador | Instalador/AppImage | Instalador/Tarball |

---

## 🛠️ Características Detalladas

### 🗂️ 1. Navegación y Gestión de Espacios de Trabajo
- **Multipestaña inteligente:** Abre notas, diagramas, imágenes y documentos en pestañas paralelas con estado de cambios sin guardar (`*`).
- **Navegación Fluida por Breadcrumbs:** Haz clic en cualquier nivel de la ruta superior para desplegar y revelar automáticamente el árbol del directorio en el explorador lateral.
- **Áreas de arrastre de ventana integradas:** Experiencia de escritorio moderna con soporte de maximizado con doble clic y áreas de arrastre nativas de Tauri (`data-tauri-drag-region`).
- **Workspaces Recientes:** Pantalla de bienvenida con acceso rápido a proyectos recientes, clonación directa de repositorios Git y creación guiada de vaults.

### 📝 2. Suite de Edición Integral
- **Markdown Multi-Modo:** Conmutación fluida entre modo edición y modo vista previa renderizada.
- **Diagramación como Código:** Sintaxis Mermaid coloreada con CodeMirror 6, visualización en tiempo real y herramientas de presentación interactiva con puntero láser.
- **Lienzos Creativos:** Integración profunda con el motor de Excalidraw para arquitectura de sistemas, diagramas libres y notas visuales.
- **Visor Multimedia y de Documentos:**
  - Lector embebido de PDFs de alta fidelidad.
  - Visor de imágenes con soporte de diff visual pixel a pixel (`pixelmatch`) para ver cambios en diseños gráficos.
  - Integración nativa con visor externo del sistema operativo para hojas de cálculo Excel y documentos Word.

### ⚡ 3. Núcleo Tecnológico de Vanguardia
- **Frontend con Svelte 5 (Runes):** Cero overhead de Virtual DOM; reactividad pura a nivel de compilador para transiciones instantáneas y una interfaz fluida a 120 FPS.
- **Feature-Sliced Design (FSD):** Arquitectura frontend altamente estructurada en capas (`app`, `pages`, `widgets`, `features`, `entities`, `shared`), facilitando una base de código limpia y libre de acoplamientos indeseados.
- **Onion Architecture en Rust:** Backend limpio dividido en Dominio, Casos de Uso y Adaptadores de Infraestructura, garantizando estabilidad y testabilidad de clase mundial.
- **Bun como Entorno de Ejecución:** Tiempos de empaquetado, resolución de dependencias y comprobación de tipos ultrarrápidos.

---

## 🎯 ¿Para quién es Synapse?

### 💻 Desarrolladores de Software y DevOps
Gestiona tus notas técnicas, specs de producto, diagramas de arquitectura de sistemas y *cheat sheets* directamente sincronizados con tus repositorios Git, sin salir de tu flujo de trabajo de teclado y VIM.

### 🏗️ Arquitectos de Soluciones y Tech Leads
Diseña y documenta flujos complejos usando Mermaid y Excalidraw, presenta diagramas a stakeholders usando el modo interactivo con puntero láser, y mantén decisiones de diseño (ADRs) bajo control de versiones.

### 🔬 Investigadores, Académicos y Estudiantes
Almacena cientos de papers en PDF, guías y notas Markdown. Usa la búsqueda Full-Text en Rust para encontrar citas, conceptos y términos específicos en milisegundos a través de todo tu archivo documental.

### ✍️ Creadores y Escritores Técnicos
Disfruta de un entorno limpio, libre de distracciones, con atajos rápidos, guardado automático configurable y un tema oscuro pulido que cuida tu vista en sesiones prolongadas.

---

## 📦 Distribución y Portabilidad

Synapse no requiere engorrosos instaladores ni permisos administrativos para ejecutarse:
- **Portabilidad Absoluta:** Un único archivo ejecutable (`synapsers` en Linux / `synapsers.exe` en Windows).
- **Cero Dependencias en Tiempo de Ejecución:** Descarga, otorga permisos y comienza a organizar tu conocimiento.

---

## 💬 El Manifiesto de Synapse

> *"El software moderno ha olvidado el valor de la ligereza y el respeto por los recursos del usuario. Creemos que una herramienta para pensar no debe secuestrar tu memoria RAM ni requerir una conexión a internet para validar tu creatividad.*  
> 
> *Synapse está construido para ser rápido hoy, mañana y dentro de veinte años, preservando tus ideas en formatos abiertos, universales y bajo tu absoluto control."*
