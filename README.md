# Synapse.rs 🧠

Aplicación de notas de conocimiento personal desarrollada en **Rust** (Tauri v2) y **Svelte 5** (Runes) con **Bun**, estructurada bajo **Feature-Sliced Design (FSD)** en el frontend y **Arquitectura Cebolla (Onion Architecture)** en el backend de Rust.

## ✨ Características

- 📝 **Editor Markdown**: Integrado con Milkdown y vista previa en tiempo real.
- 📊 **Editor Mermaid**: Edición avanzada con CodeMirror 6, resaltado de sintaxis nativo para diagramas Mermaid y visualizador interactivo con puntero láser.
- 🎨 **Visualizador Excalidraw**: Soporte completo para gráficos dibujados a mano con pestañas independientes y guardado configurable.
- ⚡ **Modo VIM**: Integración con `@replit/codemirror-vim` activable/desactivable en tiempo real desde el botón de la barra de estado.
- 🗂️ **Arquitectura Multipestaña (Tabs)**: Contexto aislado por archivo abierto e indicador visual de cambios sin guardar (`*`).
- 🏗️ **Feature-Sliced Design (FSD)**: Organización modular por capas (`app`, `pages`, `widgets`, `features`, `entities`, `shared`).

---

## 🏛️ Arquitectura Frontend: Feature-Sliced Design (FSD)

El código frontend dentro de `src` sigue rigurosamente los principios de **Feature-Sliced Design (FSD)** para maximizar la mantenibilidad, escalabilidad y aislamiento entre módulos.

### 📐 Estructura de Capas (Layers) y Slices

```
src/
├── app/                      # Capa App: Inicialización de la aplicación, estilos globales y punto de entrada raíz
│   ├── styles/               # CSS global y sistema de tokens (--bg-primary, --accent, etc.)
│   └── App.svelte            # Montaje raíz de la aplicación Svelte
│
├── pages/                    # Capa Pages: Composición de vistas y páginas completas
│   └── workspace/            # Página principal del área de trabajo (WorkspacePage)
│
├── widgets/                  # Capa Widgets: Bloques UI complejos y autónomos
│   ├── ribbon/               # Barra de herramientas e iconos laterales (Ribbon)
│   ├── editor-header/        # Encabezado superior con navegación de pestañas y controles de ventana
│   ├── status-bar/           # Barra de estado inferior con contadores, sync e indicador VIM
│   └── command-palette/      # Modal superpuesto para la paleta de comandos (Ctrl+P)
│
├── features/                 # Capa Features: Funcionalidades orientadas a la interacción del usuario
│   ├── vault-explorer/       # Explorador de archivos de la bóveda (Markdown, Mermaid, Excalidraw, PDFs, etc.)
│   ├── markdown-editor/      # Editor y visor de notas Markdown con Milkdown
│   ├── merman-editor/        # Editor/visor interactivo de diagramas Mermaid + Láser
│   └── excalidraw-editor/    # Visualizador e integrador de lienzos Excalidraw
│
├── entities/                 # Capa Entities: Entidades de negocio y modelos del dominio
│   ├── vault-item/           # Modelo de elemento de bóveda (`VaultItem`, `VaultItemKind`, `TabItem`)
│   └── command/              # Registro de comandos y motor de búsqueda difusa (`commandRegistry`)
│
└── shared/                   # Capa Shared: Reutilizables de infraestructura sin lógica de negocio
    ├── api/                  # Invocación IPC Tauri (`invokeTauri`, `isTauriEnvironment`)
    ├── ui/code-editor/       # Envoltorio genérico de editor de código CodeMirror 6
    └── assets/               # Imágenes, logotipos y recursos estáticos
```

### 🔒 Reglas de Arquitectura FSD

1. **Jerarquía Unidireccional de Importaciones**: Las capas superiores solo pueden importar elementos de capas inferiores.
   $$\text{app} \longrightarrow \text{pages} \longrightarrow \text{widgets} \longrightarrow \text{features} \longrightarrow \text{entities} \longrightarrow \text{shared}$$
2. **Sin Importaciones Horizontales entre Slices**: Un slice dentro de `features/` o `widgets/` no puede importar directamente de otro slice de la misma capa. Si se requiere compartir componentes base (como el editor CodeMirror), residen en `shared/ui`.
3. **Puntos de Entrada Públicos (`index.ts`)**: Cada slice expone únicamente su interfaz pública mediante su correspondiente `index.ts`.

### ⚡ Alias de Rutas Configurados

Para evitar rutas relativas compuestas (`..`), el proyecto cuenta con alias explícitos en `tsconfig.app.json` y `vite.config.ts`:

- `@app/*` → `src/app/*`
- `@pages/*` → `src/pages/*`
- `@widgets/*` → `src/widgets/*`
- `@features/*` → `src/features/*`
- `@entities/*` → `src/entities/*`
- `@shared/*` → `src/shared/*`

---

## 🧅 Arquitectura Backend: Onion Architecture

El código Rust en `src-tauri` implementa **Arquitectura Cebolla (Onion Architecture)**:

- **Domain**: Tipos del sistema de archivos, bóveda y structs base.
- **Application Services**: Comandos IPC (`get_vault_notes`, `save_note_content`, `select_vault_folder`).
- **Adapters / Infrastructure**: Sistema de archivos del sistema operativo e integración con Tauri v2 Core.

---

## 🚀 Requisitos Previos

- [Bun](https://bun.sh/) `>= 1.0`
- [Rust & Cargo](https://www.rust-lang.org/)
- *(Opcional para compilación cruzada a Windows)*: `cargo-xwin` (`cargo install cargo-xwin`)

---

## 🛠️ Desarrollo Local

```bash
# Instalar dependencias del proyecto
bun install

# Verificar tipos y sintaxis Svelte/TypeScript
bun check

# Iniciar la aplicación en modo desarrollo con Tauri
bun tauri dev
```

---

## 📦 Compilación (Build)

### 1. Compilación nativa (Linux / macOS / Windows)

```bash
bun tauri build
```

### 2. Compilación cruzada para Windows desde Linux (`x86_64-pc-windows-msvc`)

Para generar el archivo ejecutable binario `.exe` directamente desde Linux:

```bash
bun tauri build --target x86_64-pc-windows-msvc --runner cargo-xwin --no-bundle
```

> **Ubicación del binario generado:**  
> `src-tauri/target/x86_64-pc-windows-msvc/release/app.exe`

