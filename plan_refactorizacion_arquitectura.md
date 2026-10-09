# Plan de Refactorización Arquitectónica: FSD y Arquitectura Cebolla

Este plan establece una hoja de ruta progresiva para subsanar las desviaciones arquitectónicas detectadas en el proyecto **Synapse.rs**, ordenado rigurosamente **de menor a mayor riesgo**.

Cada paso es atómico, autónomo y cuenta con su respectivo comando de verificación independiente para asegurar que la aplicación permanezca estable en todo momento.

---

## Matriz de Riesgo y Orden de Ejecución

```
┌────────────────────────────────────────────────────────────────────────┐
│ PASO 1 (Riesgo Nulo)   : Desacoplar MarkdownViewMode (FSD Widgets)    │
│ PASO 2 (Riesgo Muy Bajo): Extraer servicios de diagramas (FSD Features) │
│ PASO 3 (Riesgo Bajo)   : Mover repo y use-case a entities/file-type    │
│ PASO 4 (Riesgo Bajo-Med): Mover FileIcon a entities/file-type/ui       │
│ PASO 5 (Riesgo Medio)  : Mover navigation a infrastructure (Rust)     │
│ PASO 6 (Riesgo Bajo)   : Casos de uso de Git y FileSystem (Rust)       │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Fase 1: Frontend (FSD) — Eliminación de Acoplamientos Horizontales

### Paso 1: Desacoplar el tipo `MarkdownViewMode` entre Widgets
> **Riesgo:** ⚪ Nulo (sólo reubicación de tipos TypeScript)  
> **Problema FSD:** `widgets/editor-header` importa `type { MarkdownViewMode }` de su widget hermano `@widgets/status-bar`. Además, el tipo está triplicado en varios archivos.

- [ ] **Acción 1.1:** Definir la fuente única de verdad en `src/entities/vault-item/model/types.ts`:
  ```ts
  export type MarkdownViewMode = 'live' | 'source' | 'reading';
  ```
  Asegurar que se exporte desde `src/entities/vault-item/index.ts`.
- [ ] **Acción 1.2:** En `src/widgets/status-bar/ui/StatusBar.svelte`, reemplazar la definición local por la importación desde `@entities/vault-item`.
- [ ] **Acción 1.3:** En `src/widgets/editor-header/ui/EditorHeader.svelte`, cambiar el import de `@widgets/status-bar` por `@entities/vault-item`.
- [ ] **Acción 1.4:** En `src/features/markdown-editor/ui/MarkdownViewer.svelte` y `src/pages/workspace/ui/WorkspacePage.svelte`, importar `MarkdownViewMode` desde `@entities/vault-item`.
- [ ] **Verificación independiente:**
  ```bash
  bun run check
  ```
  *(Resultado esperado: 0 errores, 0 advertencias)*

---

### Paso 2: Extraer Servicios de Diagramas a `shared/lib/diagrams/`
> **Riesgo:** 🟢 Muy bajo (código puramente utilitario de renderizado headless, sin JSX/Svelte ni estado)  
> **Problema FSD:** `features/markdown-editor/lib/render-diagram.ts` importa directamente desde `@features/mermaid-editor` y `@features/merman-editor`.

- [ ] **Acción 2.1:** Crear el directorio `src/shared/lib/diagrams/`.
- [ ] **Acción 2.2:** Mover `src/features/mermaid-editor/lib/mermaid-service.ts` a `src/shared/lib/diagrams/mermaid-service.ts`.
- [ ] **Acción 2.3:** Mover `src/features/merman-editor/lib/merman-service.ts` a `src/shared/lib/diagrams/merman-service.ts`.
- [ ] **Acción 2.4:** Mover `src/features/markdown-editor/lib/render-diagram.ts` a `src/shared/lib/diagrams/render-diagram.ts` (ya que es un orquestador unificado de renderizado de diagramas).
- [ ] **Acción 2.5:** Crear `src/shared/lib/diagrams/index.ts` re-exportando los tres servicios.
- [ ] **Acción 2.6:** Actualizar las importaciones en:
  - `src/features/mermaid-editor/ui/MermaidViewer.svelte`
  - `src/features/merman-editor/ui/MermanViewer.svelte`
  - `src/features/markdown-editor/lib/mermaid-extension.ts`
  Eliminar re-exportaciones de servicios de diagramas en los `index.ts` de dichas features (dejando sólo los visores/componentes UI).
- [ ] **Verificación independiente:**
  ```bash
  bun run check && bun run build
  ```
  *(Resultado esperado: compilación exitosa y renderizado funcional de diagramas)*

---

## Fase 2: Frontend (FSD) — Eliminación de Dependencias Invertidas (Hacia Arriba)

### Paso 3: Trasladar Repositorio y Carga de Tipos a `entities/file-type/api/`
> **Riesgo:** 🟢 Bajo (reubicación de repositorio IPC específico de una entidad)  
> **Problema FSD:** 
> 1. `shared/repositories/file-type.repository.ts` importa tipos de `@entities/file-type`.
> 2. `shared/use-cases/load-supported-file-types.use-case.ts` importa de `@entities/file-type` e introduce la subcapa no-FSD `use-cases` en `shared`.

- [ ] **Acción 3.1:** Crear el segmento API en `src/entities/file-type/api/`:
  - Mover `src/shared/repositories/file-type.repository.ts` a `src/entities/file-type/api/file-type.repository.ts`.
  - Mover `src/shared/use-cases/load-supported-file-types.use-case.ts` a `src/entities/file-type/api/load-supported-file-types.ts`.
- [ ] **Acción 3.2:** Eliminar la carpeta vacía `src/shared/use-cases/`.
- [ ] **Acción 3.3:** Remover `file-type.repository` de `src/shared/repositories/index.ts`.
- [ ] **Acción 3.4:** Exportar el repositorio y la función de carga desde `src/entities/file-type/index.ts`.
- [ ] **Acción 3.5:** Actualizar `src/pages/workspace/ui/WorkspacePage.svelte` para importar `loadSupportedFileTypes` desde `@entities/file-type`.
- [ ] **Verificación independiente:**
  ```bash
  bun run check && bun run build
  ```

---

### Paso 4: Mover `FileIcon.svelte` a `entities/file-type/ui/`
> **Riesgo:** 🟡 Bajo-Medio (componente visual usado en navegación y encabezados)  
> **Problema FSD:** `src/shared/ui/icons/FileIcon.svelte` consulta activamente las reglas de dominio de `fileTypesManager` de `@entities/file-type`. Por ende, es un componente de UI de la entidad `file-type`, no un icono genérico agnóstico de `shared`.

- [ ] **Acción 4.1:** Mover `src/shared/ui/icons/FileIcon.svelte` a `src/entities/file-type/ui/FileIcon.svelte`.
- [ ] **Acción 4.2:** Exportar `FileIcon` desde `src/entities/file-type/index.ts`.
- [ ] **Acción 4.3:** En `src/shared/ui/icons/index.ts`, retirar `FileIcon` y mantener únicamente iconos neutros como `FolderIcon`.
- [ ] **Acción 4.4:** Actualizar las referencias a `FileIcon` en:
  - `src/features/vault-explorer/ui/VaultExplorer.svelte`
  - `src/widgets/editor-header/ui/EditorHeader.svelte`
  - `src/widgets/breadcrumb/ui/BreadCrumb.svelte`
  - `src/widgets/quick-open/ui/QuickOpen.svelte`
  - `src/widgets/full-text-search/ui/FullTextSearch.svelte`
  Cambiando `@shared/ui/icons` por `@entities/file-type`.
- [ ] **Acción 4.5:** Ejecutar script auditor de FSD para confirmar **0 violaciones** de capas y dependencias.
- [ ] **Verificación independiente:**
  ```bash
  bun run check && bun run build
  ```

---

## Fase 3: Backend Rust (Arquitectura Cebolla) — Infraestructura y Casos de Uso

### Paso 5: Reubicar el subsistema `navigation` dentro de `infrastructure/navigation/`
> **Riesgo:** 🟡 Medio (reorganización de módulos en Rust; garantizada por 50 pruebas unitarias e integración)  
> **Problema Arquitectura Cebolla:** `src-tauri/src/navigation/` está ubicado en la raíz junto a `domain`, `application` e `infrastructure`, a pesar de ser un subsistema de caché de concurrencia (`DashMap`), persistencia a disco (`bincode`), detección de FS (`notify`) y búsqueda difusa (`nucleo`).

- [ ] **Acción 5.1:** Mover la carpeta completa `src-tauri/src/navigation/` a `src-tauri/src/infrastructure/navigation/`.
- [ ] **Acción 5.2:** En `src-tauri/src/infrastructure/mod.rs`, declarar:
  ```rust
  pub mod navigation;
  ```
- [ ] **Acción 5.3:** En `src-tauri/src/lib.rs`:
  - Retirar `pub mod navigation;`.
  - Re-exportar o actualizar accesos a través de `infrastructure::navigation`.
- [ ] **Acción 5.4:** En `src-tauri/src/infrastructure/tauri/commands.rs`, actualizar:
  ```rust
  use crate::infrastructure::navigation::engine::{NavigationEngine, OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
  ```
- [ ] **Acción 5.5:** En `tests/navigation_tests.rs`, actualizar las rutas de importación a `app_lib::infrastructure::navigation::...`.
- [ ] **Verificación independiente:**
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```
  *(Resultado esperado: Los 50 tests pasan al 100%)*

---

### Paso 6: Encapsular Git y FileSystem en Casos de Uso de Aplicación
> **Riesgo:** 🟢 Bajo (abstracción limpia en la capa `application`)  
> **Problema Arquitectura Cebolla:** En `src-tauri/src/infrastructure/tauri/commands.rs`, comandos como `git_commit_paths`, `git_add_paths` o `delete_vault_item` instancian directamente los servicios de infraestructura saltándose la capa `application`.

- [ ] **Acción 6.1:** Crear `src-tauri/src/application/use_cases/git_use_cases.rs`:
  - Casos de uso: `add_paths`, `commit_paths`, `restore_paths`, `restore_staged_paths`, `get_status`.
- [ ] **Acción 6.2:** Crear `src-tauri/src/application/use_cases/file_system_use_cases.rs`:
  - Casos de uso: `delete_item`, `rename_item`, `copy_items`.
- [ ] **Acción 6.3:** En `commands.rs`, delegar la ejecución a través de estos casos de uso en vez de instanciar `GitService` o `FileSystemService` directamente.
- [ ] **Verificación independiente:**
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

---

## Verificación Global Final

Al completar los 6 pasos, ejecutar el pipeline completo de validación:

```bash
# 1. Verificación de tipos y lint de frontend
bun run check

# 2. Compilación de producción de frontend
bun run build

# 3. Suite completa de pruebas unitarias y de integración de backend Rust
cargo test --manifest-path src-tauri/Cargo.toml
```
