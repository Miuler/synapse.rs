# Synapse.rs 🧠

Aplicación de notas de conocimiento personal desarrollada en **Rust** (Tauri v2) y **Svelte 5** (Runes) con **Bun**, basada en arquitectura cebolla (Onion Architecture).

## ✨ Características

- 📝 **Editor Markdown**: Integrado con Milkdown y vista previa en tiempo real.
- 📊 **Editor Mermaid**: Edición avanzada con CodeMirror 6, resaltado de sintaxis nativo para diagramas Mermaid y visualizador interactivo con puntero láser.
- 🎨 **Visualizador Excalidraw**: Soporte completo para gráficos dibujados a mano con pestañas independientes y guardado configurable.
- ⚡ **Modo VIM**: Integración con `@replit/codemirror-vim` activable/desactivable en tiempo real desde el botón de la barra de estado.
- 🗂️ **Arquitectura Multipestaña (Tabs)**: Contexto aislado por archivo abierto e indicador visual de cambios sin guardar (`*`).

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

# Iniciar la aplicación en modo desarrollo
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
