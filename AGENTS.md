# Guía para Agentes de IA (AGENTS.md) 🧠

Este documento establece las **reglas de oro, principios arquitectónicos y estándares de diseño** que cualquier agente de inteligencia artificial (o desarrollador) **DEBE cumplir estrictamente** al crear, modificar o refactorizar código en **Synapse.rs**.

---

## 🎯 Preferencias del Desarrollador (Developer Mandates)

1. **Scripts y Automatización**: Priorizar **Nushell** (`nu`) para cualquier script o comando de automatización.
2. **Programación Backend**: **Rust** como lenguaje principal del backend (Tauri v2), con pruebas exhaustivas (`cargo test`).
3. **Programación Frontend**: **Svelte 5 (Runes)** y **TypeScript** ejecutados con **Bun**.
4. **Arquitecturas Mandatarias**:
   - **Backend**: **Arquitectura Cebolla (Onion Architecture)**.
   - **Frontend**: **Feature-Sliced Design (FSD)**.

---

## 🧅 1. Backend Rust: Principios de Arquitectura Cebolla

El backend reside en `src-tauri/src` y se organiza en capas concéntricas. La regla fundamental es la **Inversión de Dependencias**: las dependencias siempre apuntan hacia el centro (Dominio).

```
┌────────────────────────────────────────────────────────┐
│ INFRASTRUCTURE (src-tauri/src/infrastructure/)         │
│   ├── tauri/ (commands.rs, AppState)                   │
│   ├── repositories/ (FileNoteRepository)               │
│   ├── services/ (GitService, FileSystemService)        │
│   ├── search/ (TantivyIndex, FtsIndexer)               │
│   └── navigation/ (DashMap, storage, watcher, nucleo)  │
│   ┌──────────────────────────────────────────────┐     │
│   │ APPLICATION (src-tauri/src/application/)     │     │
│   │   └── use_cases/ (NoteUseCases, etc.)        │     │
│   │   ┌────────────────────────────────────┐     │     │
│   │   │ DOMAIN (src-tauri/src/domain/)     │     │     │
│   │   │   ├── models/ (Note, etc.)         │     │     │
│   │   │   ├── value_objects/ (NotePath)    │     │     │
│   │   │   ├── repositories/ (traits)       │     │     │
│   │   │   ├── services/ (Domain Services)  │     │     │
│   │   │   └── events/ (VaultEvents)        │     │     │
│   │   └────────────────────────────────────┘     │     │
│   └──────────────────────────────────────────────┘     │
└────────────────────────────────────────────────────────┘
```

### Reglas para cualquier nueva creación en Backend:

1. **Capa `domain` (Núcleo Puro)**:
   - Debe contener únicamente entidades de negocio, value objects validados, traits/puertos de repositorios y servicios de pura lógica.
   - **PROHIBIDO**: Importar `application`, `infrastructure`, `navigation` o `tauri`.
   - **PROHIBIDO**: Realizar operaciones directas de Entrada/Salida (I/O), llamadas de red, comandos del sistema o acceso directo a disco dentro del dominio.

2. **Capa `application` (Casos de Uso)**:
   - Orquesta los flujos de la aplicación mediante casos de uso (`use_cases/`).
   - Solo depende de los traits y modelos definidos en `domain`.
   - **PROHIBIDO**: Importar implementaciones concretas de `infrastructure` o depender de `tauri`. Debe recibir los puertos mediante genéricos (`<R: NoteRepository>`) o inyección de dependencias (`Arc<dyn Trait>`).

3. **Capa `infrastructure` (Adaptadores y Framework)**:
   - Implementa los traits de `domain` interactuando con el sistema operativo (sistema de archivos, Git con `gix`, Tantivy, persistencia `bincode`, eventos con `notify`).
   - Contiene la capa de presentación/IPC de Tauri en `infrastructure/tauri/commands.rs`.
   - Los comandos de Tauri deben delegar la lógica de negocio en los casos de uso de `application` y orquestar llamadas de infraestructura.

4. **Flujo para agregar una nueva funcionalidad en Rust**:
   1. ¿Hay nuevas entidades o reglas de negocio? → Definir modelos o value objects en `domain/models/` o `domain/value_objects/`.
   2. ¿Se requiere persistencia o servicio externo? → Definir el trait/puerto en `domain/repositories/` o `domain/services/`.
   3. Crear el caso de uso en `application/use_cases/` que coordine la acción.
   4. Implementar el adaptador técnico en `infrastructure/`.
   5. Exponer el comando `#[tauri::command]` en `infrastructure/tauri/commands.rs` y registrarlo en `src-tauri/src/lib.rs`.
   6. Escribir pruebas unitarias e integración en el módulo correspondiente.

---

## 🏛️ 2. Frontend Svelte/TypeScript: Principios de Feature-Sliced Design (FSD)

El código frontend reside en `src/` y sigue estrictamente **Feature-Sliced Design**.

### Jerarquía de Capas (de arriba hacia abajo):
$$\text{app} \longrightarrow \text{pages} \longrightarrow \text{widgets} \longrightarrow \text{features} \longrightarrow \text{entities} \longrightarrow \text{shared}$$

```
src/
├── app/          # Inicialización global, estilos base (app.css), montaje raíz (App.svelte)
├── pages/        # Vistas de página completa (workspace)
├── widgets/      # Bloques visuales compuestos autónomos (ribbon, status-bar, editor-header, etc.)
├── features/     # Casos de uso e interacciones del usuario (vault-explorer, markdown-editor, etc.)
├── entities/     # Entidades de negocio, sus tipos, stores y UI específica (vault-item, file-type, etc.)
└── shared/       # Código genérico reutilizable sin lógica de negocio (ui kit, api IPC base, lib)
```

### Reglas de Oro FSD:

#### 1. Jerarquía Unidireccional de Importaciones (NUNCA hacia arriba)
- Una capa solo puede importar elementos de capas situadas **estrictamente por debajo** en la jerarquía:
  - `pages` puede importar de `widgets`, `features`, `entities`, `shared`.
  - `widgets` puede importar de `features`, `entities`, `shared`.
  - `features` puede importar de `entities`, `shared`.
  - `entities` **SOLO** puede importar de `shared`.
  - `shared` **NUNCA** puede importar de ninguna otra capa (`entities`, `features`, `widgets`, etc.).

#### 2. Cero Importaciones Horizontales (Cross-slice imports)
- **PROHIBIDO**: Una `feature` **NO** puede importar de otra `feature` (`features/A` ❌ `features/B`).
- **PROHIBIDO**: Un `widget` **NO** puede importar de otro `widget` (`widgets/A` ❌ `widgets/B`).
- **PROHIBIDO**: Una `entity` **NO** puede importar de otra `entity` (`entities/A` ❌ `entities/B`).
- **Solución ante código compartido**:
  - Si dos widgets o features comparten un tipo o estado de negocio → Mover a `entities`.
  - Si dos features comparten un servicio o lógica de renderizado headless (ej. diagramas Mermaid/Merman) → Mover a `shared/lib/`.
  - Si se comparten componentes de UI genéricos (botones, diálogos, iconos neutros) → Mover a `shared/ui/`.

#### 3. Estructura Interna Canónica de cada Slice
Cada slice dentro de `features/`, `widgets/` o `entities/` debe organizarse en los siguientes segmentos estándar:
- `ui/`: Componentes Svelte de la slice.
- `model/`: Tipos, estados reactivos, stores (`.svelte.ts`, `.ts`).
- `api/`: Repositorios o llamadas IPC Tauri específicas del slice.
- `lib/`: Utilidades internas y algoritmos propios del slice.
- `index.ts`: **Punto de entrada público obligatorio**.
  - **Solo se debe importar a través del `index.ts`** utilizando los alias de capa (`@entities/vault-item`, `@features/markdown-editor`, etc.).
  - **PROHIBIDO** importar archivos internos profundos de otro slice (ej. ❌ `@features/markdown-editor/ui/MarkdownViewer.svelte`).

#### 4. Terminología FSD
- **PROHIBIDO** crear carpetas `use-cases` en `shared/`. En FSD, la lógica de entidades va en `entities/<slice>/api` o `model`, y los flujos de usuario en `features`.

---

## 🎨 3. Estándares de Código y Calidad

### Svelte 5 (Runes)
- **OBLIGATORIO**: Usar siempre la sintaxis de **Runes**:
  - `$state(...)` para estado reactivo.
  - `$derived(...)` para valores calculados.
  - `$props()` para propiedades de componentes.
  - `$effect(...)` para efectos secundarios controlados.
  - `$bindable()` para props enlazables bidireccionalmente.
  - `{#snippet ...}` y `{@render ...}` para renderizado modular de bloques.
- **PROHIBIDO**: Usar sintaxis heredada de Svelte 3/4 (`export let`, `$: reactive`, `createEventDispatcher`).

### Tokens y Estilos Visuales
- Utilizar siempre las variables CSS del sistema para garantizar soporte dual de temas claro y oscuro:
  - `--bg-primary`, `--bg-secondary`, `--bg-tertiary`
  - `--text-primary`, `--text-secondary`, `--text-muted`
  - `--border-primary`, `--border-secondary`
  - `--accent`
- Componentes accesibles y modales mediante `bits-ui`.

---

## ✅ 4. Verificación Obligatoria de Cada Cambio

Antes de dar por completada cualquier tarea o modificación, el agente debe verificar de forma independiente los siguientes comandos:

```bash
# 1. Comprobación de tipos Svelte 5 y TypeScript (debe dar 0 errores y 0 warnings)
bun run check

# 2. Verificación de build de producción del Frontend
bun run build

# 3. Verificación de la suite de pruebas del Backend Rust (todos deben pasar)
cargo test --manifest-path src-tauri/Cargo.toml
```
