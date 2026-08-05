# Reglas de Arquitectura de Synapse.rs 🏗️

Este proyecto combina un backend potente en **Rust** (Tauri v2) con un frontend moderno en **Svelte 5** (Runes), siguiendo estándares arquitectónicos estrictos para garantizar escalabilidad, mantenibilidad y desacoplamiento.

---

## 🎨 1. Arquitectura Frontend: Feature-Sliced Design (FSD)

El código web ubicado en `src/` sigue estrictamente **Feature-Sliced Design (FSD)**.

### Capas (Layers) de FSD (de menor a mayor jerarquía):

```
src/
├── app/              # Inicialización de la aplicación, estilos globales y montaje raíz
│   ├── styles/       # Tokens de diseño y CSS global (app.css)
│   └── App.svelte    # Componente raíz de Svelte
│
├── pages/            # Composición de páginas/vistas completas a partir de widgets
│   └── workspace/    # Página principal del área de trabajo (WorkspacePage.svelte)
│
├── widgets/          # Componentes complejos y autónomos UI que combinan features/entities
│   ├── ribbon/       # Barra lateral de herramientas e iconos
│   ├── editor-header/# Barra superior de pestañas, toggle edición/lectura y controles de ventana
│   ├── status-bar/   # Barra inferior con contadores, indicador de guardado y modo VIM
│   └── command-palette/ # Modal flotante para ejecución de comandos difusos (Ctrl+P)
│
├── features/         # Interacciones del usuario que aportan valor de negocio concreto
│   ├── vault-explorer/  # Explorador de archivos de la bóveda de notas
│   ├── markdown-editor/ # Visor/editor Markdown enriquecido con Milkdown
│   ├── merman-editor/   # Editor/visor interactivo de diagramas Mermaid + puntero láser
│   └── excalidraw-editor/# Lienzo de dibujos a mano alzada con Excalidraw
│
├── entities/         # Entidades del dominio de negocio (modelos de datos y registros)
│   ├── note/         # Modelo de datos de Notas (`NoteItem`, `TabItem`)
│   └── command/      # Registro global y lógica de búsqueda de comandos (`commandRegistry`)
│
└── shared/           # Infraestructura reutilizable sin lógica de dominio
    ├── api/          # Abstracción IPC de comunicación con Rust (`invokeTauri`)
    ├── ui/           # Componentes UI genéricos independientes (`CodeEditor`)
    └── assets/       # Imágenes, fuentes e iconos estáticos
```

### Reglas de importación en FSD:
- **Flujo Unidireccional**: Una capa solo puede importar elementos de capas inferiores (`app` → `pages` → `widgets` → `features` → `entities` → `shared`).
- **Aislamiento entre Slices**: Un slice no debe importar directamente de otro slice del mismo nivel (ej: un feature no importa otro feature). Los componentes compartidos de bajo nivel (como `CodeEditor`) residen en `shared/ui`.
- **Public API (`index.ts`)**: Cada slice expone únicamente su API pública mediante su archivo `index.ts`.

---

## 🧅 2. Arquitectura Backend: Arquitectura Cebolla (Onion Architecture)

El backend en **Rust** (ubicado en `src-tauri/`) sigue **Onion Architecture**:

- **Domain (Core Interno)**: Modelos de datos del vault, estructuras de notas e interfaces base de servicios.
- **Application Services**: Casos de uso de la aplicación (crear nota, leer bóveda, persistencia en disco).
- **Adapters / Infrastructure (Capa Exterior)**: Integración con el sistema de archivos del SO y la API de Tauri IPC.

---

## ⚡ Alias de Rutas Configurados

Para mantener importaciones limpias y alineadas con FSD, se utilizan los siguientes alias en TypeScript y Vite:

- `@app/*` → `src/app/*`
- `@pages/*` → `src/pages/*`
- `@widgets/*` → `src/widgets/*`
- `@features/*` → `src/features/*`
- `@entities/*` → `src/entities/*`
- `@shared/*` → `src/shared/*`
