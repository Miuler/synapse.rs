<script lang="ts">
  import { onMount, tick } from "svelte";
  import { Ribbon } from "@widgets/ribbon";
  import { EditorHeader } from "@widgets/editor-header";
  import { StatusBar } from "@widgets/status-bar";
  import { CommandPalette } from "@widgets/command-palette";
  import { VaultExplorer } from "@features/vault-explorer";
  import { MarkdownViewer } from "@features/markdown-editor";
  import { MermanViewer } from "@features/merman-editor";
  import { ExcalidrawViewer } from "@features/excalidraw-editor";
  import type { NoteItem } from "@entities/note";
  import { commandRegistry } from "@entities/command";
  import { invokeTauri, isTauriEnvironment } from "@shared/api";

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state("files");
  let isPaletteOpen = $state(false);
  let isEditing = $state(true);
  let isVimMode = $state(false);
  let isConnectedToRust = $state(false);
  let syncState = $state<"synced" | "saving" | "error">("synced");

  // Estado reactivo dinámico
  let notes = $state<NoteItem[]>([]);
  let openTabPaths = $state<string[]>([]);
  let activeTabPath = $state<string | null>(null);
  let sidebarWidth = $state(240);
  let isResizingSidebar = $state(false);

  function selectTab(path: string) {
    if (!openTabPaths.includes(path)) {
      openTabPaths.push(path);
    }
    activeTabPath = path;
  }

  function closeTab(path: string) {
    const idx = openTabPaths.indexOf(path);
    if (idx !== -1) {
      openTabPaths.splice(idx, 1);
      if (activeTabPath === path) {
        if (openTabPaths.length > 0) {
          const nextIdx = Math.min(idx, openTabPaths.length - 1);
          activeTabPath = openTabPaths[nextIdx];
        } else {
          activeTabPath = null;
        }
      }
    }
  }

  let savedContents = $state<Record<string, string>>({});

  let currentNote = $derived(
    activeTabPath && notes.length > 0
      ? notes.find((n) => n.relative_path === activeTabPath) || { id: "0", title: "", content: "", relative_path: "" }
      : { id: "0", title: "", content: "", relative_path: "" }
  );

  let tabsInfo = $derived(
    openTabPaths.map((path) => {
      const n = notes.find((item) => item.relative_path === path);
      const isDirty = n ? (savedContents[path] !== undefined && n.content !== savedContents[path]) : false;
      return {
        path,
        title: n ? (n.title || n.relative_path) : path,
        isDirty,
      };
    })
  );

  // Contadores calculados reactivamente
  let wordCount = $derived(
    currentNote.content && currentNote.content.trim()
      ? currentNote.content.trim().split(/\s+/).length
      : 0,
  );
  let charCount = $derived(
    currentNote.content ? currentNote.content.length : 0,
  );

  let editorContainerRef = $state<HTMLDivElement | null>(null);

  // Volver al inicio del documento cada vez que se abre una nota distinta
  $effect(() => {
    activeTabPath;
    tick().then(() => {
      if (editorContainerRef) editorContainerRef.scrollTop = 0;
    });
  });

  // Carga estrictamente dinámica desde Rust (Tauri IPC) al montar
  onMount(() => {
    async function fetchNotesFromBackend() {
      if (isTauriEnvironment()) {
        try {
          const realNotes =
            await invokeTauri<
              Array<{
                relative_path: { 0?: string } | string;
                title: string;
                content: string;
              }>
            >("get_vault_notes");

          if (realNotes && Array.isArray(realNotes)) {
            isConnectedToRust = true;
            const newSavedMap: Record<string, string> = {};
            notes = realNotes.map((n, index) => {
              let relPath = `${n.title}.md`;
              if (typeof n.relative_path === "string") {
                relPath = n.relative_path;
              } else if (
                n.relative_path &&
                typeof n.relative_path === "object" &&
                n.relative_path[0]
              ) {
                relPath = n.relative_path[0];
              }

              newSavedMap[relPath] = n.content;

              return {
                id: String(index + 1),
                title: n.title,
                content: n.content,
                relative_path: relPath,
              };
            });
            savedContents = newSavedMap;
            openTabPaths = [];
            activeTabPath = null;
          } else {
            notes = [];
            savedContents = {};
            openTabPaths = [];
            activeTabPath = null;
          }
        } catch (e) {
          console.warn("Error al cargar notas de Rust:", e);
          isConnectedToRust = false;
          notes = [];
          savedContents = {};
        }
      } else {
        notes = [];
        savedContents = {};
      }
    }

    fetchNotesFromBackend();
  });

  // Estado de Auto-Guardado
  let autoSave = $state(true);
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  function debouncedPersistNoteToRust(note: NoteItem, delay = 600) {
    if (!autoSave) return;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      persistNoteToRust(note);
    }, delay);
  }

  async function persistNoteToRust(note: NoteItem) {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }
    if (!isConnectedToRust || !note.title) return;
    syncState = "saving";
    try {
      await invokeTauri("save_note_content", {
        relativePath: note.relative_path || `${note.title}.md`,
        title: note.title,
        content: note.content,
      });
      savedContents[note.relative_path] = note.content;
      syncState = "synced";
    } catch (e) {
      console.error("Error al guardar la nota en Rust:", e);
      syncState = "error";
    }
  }

  async function createNewNote() {
    const newTitle = `Nueva Nota ${notes.length + 1}`;
    const newRelPath = `${newTitle}.md`;
    const newNote: NoteItem = {
      id: String(notes.length + 1),
      title: newTitle,
      content: "# Nueva Nota\n\nEscribe tu contenido aquí...",
      relative_path: newRelPath,
    };
    notes.push(newNote);
    selectTab(newRelPath);
    await persistNoteToRust(newNote);
  }

  // Registrar comandos por defecto al iniciar
  onMount(() => {
    commandRegistry.registerMany([
      {
        id: "cmd-new-note",
        name: "Crear nueva nota",
        category: "Archivo",
        shortcut: "Ctrl+N",
        action: createNewNote,
      },
      {
        id: "cmd-open-palette",
        name: "Abrir paleta de comandos",
        category: "Sistema",
        shortcut: "Ctrl+P",
        action: () => {
          isPaletteOpen = true;
        },
      },
      {
        id: "cmd-toggle-view",
        name: "Alternar entre modo Edición y Lectura",
        category: "Vista",
        shortcut: "Ctrl+E",
        action: () => {
          isEditing = !isEditing;
        },
      },
      {
        id: "cmd-save-note",
        name: "Guardar / Grabar nota actual",
        category: "Archivo",
        shortcut: "Ctrl+S",
        action: () => {
          if (activeTabPath && currentNote.relative_path) {
            persistNoteToRust(currentNote);
          }
        },
      },
    ]);

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        if (activeTabPath && currentNote.relative_path) {
          persistNoteToRust(currentNote);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });

  function handleRibbonAction(actionId: string) {
    if (actionId === "command-palette") {
      isPaletteOpen = true;
    } else if (actionId === "new-note") {
      createNewNote();
    }
  }

  async function handleOpenVaultFolder() {
    if (!isTauriEnvironment()) return;
    try {
      const newNotes = await invokeTauri<Array<{ relative_path: { 0?: string } | string; title: string; content: string }> | null>('select_vault_folder');
      if (newNotes && Array.isArray(newNotes)) {
        const newSavedMap: Record<string, string> = {};
        notes = newNotes.map((n, index) => {
          let relPath = `${n.title}.md`;
          if (typeof n.relative_path === 'string') {
            relPath = n.relative_path;
          } else if (n.relative_path && typeof n.relative_path === 'object' && n.relative_path[0]) {
            relPath = n.relative_path[0];
          }

          newSavedMap[relPath] = n.content;

          return {
            id: String(index + 1),
            title: n.title,
            content: n.content,
            relative_path: relPath
          };
        });
        savedContents = newSavedMap;
        openTabPaths = [];
        activeTabPath = null;
      }
    } catch (e) {
      console.error('Error al abrir la carpeta de la bóveda:', e);
    }
  }

  function handleSidebarResizeStart(e: PointerEvent) {
    isResizingSidebar = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handleSidebarResizeMove(e: PointerEvent) {
    if (isResizingSidebar) {
      const newWidth = e.clientX - 48;
      sidebarWidth = Math.max(150, Math.min(newWidth, 600));
    }
  }

  function handleSidebarResizeEnd(e: PointerEvent) {
    if (isResizingSidebar) {
      isResizingSidebar = false;
      try {
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      } catch {
        // Ignorar si el puntero se libera automáticamente
      }
    }
  }
</script>

<div class="workspace-layout">
  <!-- 1. BARRA RIBBON IZQUIERDA -->
  <Ribbon bind:activeTab={activeRibbonTab} onAction={handleRibbonAction} />

  <!-- 2. PANEL LATERAL (EXPLORADOR DE ARCHIVOS DE LA BÓVEDA) -->
  <VaultExplorer
    {activeRibbonTab}
    {sidebarWidth}
    {isResizingSidebar}
    {isConnectedToRust}
    {notes}
    {activeTabPath}
    onSelectTab={selectTab}
    onOpenVaultFolder={handleOpenVaultFolder}
    onResizeStart={handleSidebarResizeStart}
    onResizeMove={handleSidebarResizeMove}
    onResizeEnd={handleSidebarResizeEnd}
  />

  <!-- 3. ÁREA DE TRABAJO PRINCIPAL -->
  <main class="main-workspace">
    <!-- BARRA SUPERIOR DE PESTAÑAS Y HERRAMIENTAS -->
    <EditorHeader
      bind:isEditing
      tabs={tabsInfo}
      {activeTabPath}
      title={currentNote.relative_path || currentNote.title}
      showSaveButton={isEditing &&
        !!activeTabPath &&
        (!currentNote.relative_path ||
          currentNote.relative_path.endsWith(".md") ||
          currentNote.relative_path.endsWith(".markdown") ||
          currentNote.relative_path.endsWith(".excalidraw") ||
          currentNote.relative_path.endsWith(".excalidraw.json"))}
      onSelectTab={(path) => selectTab(path)}
      onCloseTab={(path) => closeTab(path)}
      onSave={() => {
        if (activeTabPath && currentNote.relative_path) {
          persistNoteToRust(currentNote);
        }
      }}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />

    <!-- CONTENEDOR DEL EDITOR CON MULTI-TAB -->
    <div class="editor-container" bind:this={editorContainerRef}>
      {#if openTabPaths.length === 0 || !activeTabPath}
        <div class="empty-workspace">
          <svg
            class="empty-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
          >
            <path
              d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
            />
            <polyline points="14 2 14 8 20 8" />
            <line x1="9" y1="15" x2="15" y2="15" />
          </svg>
          <h2>
            {notes.length === 0
              ? "No hay archivos en la bóveda"
              : "Ningún archivo abierto"}
          </h2>
          <p>
            {notes.length === 0
              ? "Crea una nueva nota para comenzar a escribir."
              : "Selecciona un archivo del panel lateral para abrirlo."}
          </p>
          <button class="create-btn" onclick={createNewNote}>
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            <span>Crear nueva nota</span>
          </button>
        </div>
      {:else}
        {#each openTabPaths as tabPath (tabPath)}
          {@const note = notes.find((n) => n.relative_path === tabPath)}
          {#if note}
            <div
              class="tab-pane"
              class:hidden={tabPath !== activeTabPath}
              class:full-pane={tabPath.endsWith(".mmd") ||
                tabPath.endsWith(".mermaid") ||
                tabPath.endsWith(".excalidraw") ||
                tabPath.endsWith(".excalidraw.json")}
            >
              {#if tabPath.endsWith(".mmd") || tabPath.endsWith(".mermaid")}
                <MermanViewer
                  content={note.content}
                  readOnly={!isEditing}
                  vimMode={isVimMode}
                  onChange={(updatedContent) => {
                    note.content = updatedContent;
                    debouncedPersistNoteToRust(note);
                  }}
                />
              {:else if tabPath.endsWith(".excalidraw") || tabPath.endsWith(".excalidraw.json")}
                <ExcalidrawViewer
                  content={note.content}
                  readOnly={!isEditing}
                  onChange={(updatedContent) => {
                    note.content = updatedContent;
                    debouncedPersistNoteToRust(note);
                  }}
                />
              {:else}
                <input
                  type="text"
                  class="editor-title-input"
                  bind:value={note.title}
                  oninput={() => persistNoteToRust(note)}
                  placeholder="Título de la nota..."
                />

                <div class="editor-main-content">
                  <MarkdownViewer
                    content={note.content}
                    readOnly={!isEditing}
                    onChange={(updatedMarkdown) => {
                      note.content = updatedMarkdown;
                      debouncedPersistNoteToRust(note);
                    }}
                    isMarkdown={!note.relative_path ||
                      note.relative_path.endsWith(".md") ||
                      note.relative_path.endsWith(".markdown")}
                  />
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    </div>

    <!-- BARRA DE ESTADO INFERIOR -->
    <StatusBar
      wordCount={notes.length > 0 ? wordCount : 0}
      charCount={notes.length > 0 ? charCount : 0}
      line={notes.length > 0 ? 1 : 0}
      col={notes.length > 0 && currentNote.content
        ? currentNote.content.length
        : 0}
      syncStatus={syncState}
      isVimMode={isVimMode}
      onToggleVim={() => (isVimMode = !isVimMode)}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />
  </main>

  <!-- 4. PALETA DE COMANDOS (OVERLAY CTRL+P) -->
  <CommandPalette bind:isOpen={isPaletteOpen} />
</div>

<style>
  .workspace-layout {
    display: flex;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background-color: var(--bg-primary, #ffffff);
  }

  .main-workspace {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .editor-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    flex: 1;
    overflow: hidden;
  }

  .tab-pane.hidden {
    display: none !important;
  }

  .tab-pane.full-pane {
    padding: 0;
  }

  .editor-title-input {
    font-size: 24px;
    font-weight: 700;
    color: var(--text-primary, #1f2328);
    background: transparent;
    border: none;
    outline: none;
    padding: 16px 24px 8px 24px;
    font-family: inherit;
  }

  .editor-title-input::placeholder {
    color: var(--text-secondary, #656d76);
  }

  .editor-main-content {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .empty-workspace {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    color: var(--text-secondary);
    text-align: center;
    user-select: none;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    color: var(--border-primary);
    margin-bottom: 8px;
  }

  .empty-workspace h2 {
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .empty-workspace p {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
  }

  .create-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 16px;
    background-color: var(--accent-bg);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .create-btn:hover {
    background-color: rgba(9, 105, 218, 0.15);
    border-color: var(--accent);
  }
</style>
