<script lang="ts">
  import { onMount, tick } from "svelte";
  import { Ribbon } from "@widgets/ribbon";
  import { EditorHeader } from "@widgets/editor-header";
  import { StatusBar, type MarkdownViewMode } from "@widgets/status-bar";
  import { CommandPalette } from "@widgets/command-palette";
  import { QuickOpen } from "@widgets/quick-open";
  import { EmptyWorkspace } from "@widgets/empty-workspace";
  import { VaultExplorer } from "@features/vault-explorer";
  import { MarkdownViewer } from "@features/markdown-editor";
  import { MermanViewer } from "@features/merman-editor";
  import { ExcalidrawViewer } from "@features/excalidraw-editor";
  import { ImageViewer } from "@features/image-viewer";
  import type { VaultItem } from "@entities/vault-item";
  import { commandRegistry } from "@entities/command";
  import { invokeTauri, isTauriEnvironment } from "@shared/api";

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state("files");
  let isPaletteOpen = $state(false);
  let isQuickOpenOpen = $state(false);
  let isEditing = $state(true);
  let isVimMode = $state(false);
  let markdownViewMode = $state<MarkdownViewMode>("live");
  let isConnectedToRust = $state(false);
  let syncState = $state<"synced" | "saving" | "error">("synced");

  // Lista de metadatos de elementos de la bóveda (VaultItem[])
  let vaultItems = $state<VaultItem[]>([]);
  let openTabPaths = $state<string[]>([]);
  let activeTabPath = $state<string | null>(null);
  let sidebarWidth = $state(240);
  let isResizingSidebar = $state(false);

  // Lista de rutas de archivos abiertos recientemente para Ctrl+O (Quick Open)
  let recentFiles = $state<string[]>([]);

  // Mapa reactivo de contenidos cargados BAJO DEMANDA sólo para las pestañas abiertas
  let loadedContents = $state<Record<string, string>>({});
  let savedContents = $state<Record<string, string>>({});
  let loadingPaths = $state<Record<string, boolean>>({});
  let tabEncodings = $state<Record<string, string>>({});

  // Contador para generar IDs únicos de pestañas vacías
  let emptyTabCounter = 0;

  // Seguimiento de selección y cursor por cada pestaña
  interface SelectionInfo {
    hasSelection: boolean;
    selectedWords: number;
    selectedChars: number;
    selectedLines: number;
    selectedCols: number;
    cursorLine: number;
    cursorCol: number;
  }

  const defaultSelection: SelectionInfo = {
    hasSelection: false,
    selectedWords: 0,
    selectedChars: 0,
    selectedLines: 1,
    selectedCols: 0,
    cursorLine: 1,
    cursorCol: 1,
  };

  let tabSelections = $state<Record<string, SelectionInfo>>({});

  function handleSelectionChange(path: string, info: SelectionInfo) {
    tabSelections[path] = info;
  }

  function handleChangeMarkdownView(newMode: MarkdownViewMode) {
    markdownViewMode = newMode;
    if (newMode === "reading") {
      isEditing = false;
    } else {
      isEditing = true;
    }
  }

  function toggleMarkdownViewMode() {
    const current = !isEditing ? "reading" : markdownViewMode;
    if (current === "live") {
      handleChangeMarkdownView("source");
    } else if (current === "source") {
      handleChangeMarkdownView("reading");
    } else {
      handleChangeMarkdownView("live");
    }
  }

  async function ensureContentLoaded(path: string) {
    if (!path || path.startsWith("empty:") || loadedContents[path] !== undefined || loadingPaths[path]) return;

    if (isTauriEnvironment()) {
      loadingPaths[path] = true;
      try {
        const noteData = await invokeTauri<{
          relative_path: string;
          abs_path: string;
          title: string;
          content: string;
          encoding?: string;
        }>("read_note_content", { relativePath: path });

        const fetchedContent = noteData?.content ?? "";
        const fetchedEncoding = noteData?.encoding && noteData.encoding.trim() !== "" ? noteData.encoding : "---";
        loadedContents[path] = fetchedContent;
        savedContents[path] = fetchedContent;
        tabEncodings[path] = fetchedEncoding;
      } catch (e) {
        console.error(`Error al cargar contenido de ${path} desde Rust:`, e);
        loadedContents[path] = "";
        savedContents[path] = "";
        tabEncodings[path] = "---";
      } finally {
        loadingPaths[path] = false;
      }
    } else {
      loadedContents[path] = "";
      savedContents[path] = "";
      tabEncodings[path] = "---";
    }
  }

  function handleNewEmptyTab() {
    emptyTabCounter += 1;
    const emptyTabPath = `empty://${emptyTabCounter}-${Date.now()}`;
    openTabPaths.push(emptyTabPath);
    activeTabPath = emptyTabPath;
  }

  function selectTab(path: string) {
    // Si la pestaña actual es una pestaña vacía y seleccionamos un archivo nuevo, lo sustituye en esa pestaña
    if (activeTabPath && activeTabPath.startsWith("empty:") && !openTabPaths.includes(path)) {
      const idx = openTabPaths.indexOf(activeTabPath);
      if (idx !== -1) {
        openTabPaths[idx] = path;
        delete tabSelections[activeTabPath];
      } else {
        openTabPaths.push(path);
      }
    } else if (!openTabPaths.includes(path)) {
      openTabPaths.push(path);
    }
    activeTabPath = path;
    if (!path.startsWith("empty:")) {
      ensureContentLoaded(path);
      recentFiles = [path, ...recentFiles.filter((p) => p !== path)];
    }
  }

  function closeTab(path: string) {
    const idx = openTabPaths.indexOf(path);
    if (idx !== -1) {
      openTabPaths.splice(idx, 1);
      if (activeTabPath === path) {
        if (openTabPaths.length > 0) {
          const nextIdx = Math.min(idx, openTabPaths.length - 1);
          activeTabPath = openTabPaths[nextIdx];
          if (activeTabPath && !activeTabPath.startsWith("empty:")) {
            ensureContentLoaded(activeTabPath);
          }
        } else {
          activeTabPath = null;
        }
      }
    }
    // Liberar memoria del contenido y selecciones cuando la pestaña se cierra
    delete loadedContents[path];
    delete savedContents[path];
    delete loadingPaths[path];
    delete tabSelections[path];
    delete tabEncodings[path];

    // Si no queda ningún tab abierto, crear automáticamente una pestaña vacía
    if (openTabPaths.length === 0) {
      handleNewEmptyTab();
    }
  }

  function closeAllTabs() {
    openTabPaths = [];
    activeTabPath = null;
    loadedContents = {};
    savedContents = {};
    loadingPaths = {};
    tabSelections = {};
    tabEncodings = {};
    handleNewEmptyTab();
  }

  let currentVaultItem = $derived(
    activeTabPath && !activeTabPath.startsWith("empty:") && vaultItems.length > 0
      ? vaultItems.find((vaultItem) => vaultItem.relative_path === activeTabPath) || { id: "0", title: "", relative_path: "" }
      : { id: "0", title: "", relative_path: "" }
  );

  let activeContent = $derived(
    activeTabPath && !activeTabPath.startsWith("empty:") ? loadedContents[activeTabPath] ?? "" : ""
  );

  let currentEncoding = $derived(
    activeTabPath && !activeTabPath.startsWith("empty:") && tabEncodings[activeTabPath]
      ? tabEncodings[activeTabPath]
      : "---"
  );

  let isMarkdownFile = $derived(
    Boolean(
      activeTabPath &&
      !activeTabPath.startsWith("empty:") &&
      (
        activeTabPath.endsWith(".md") ||
        activeTabPath.endsWith(".markdown") ||
        (currentVaultItem.relative_path && (
          currentVaultItem.relative_path.endsWith(".md") ||
          currentVaultItem.relative_path.endsWith(".markdown")
        ))
      )
    )
  );

  let tabsInfo = $derived(
    openTabPaths.map((path) => {
      if (path.startsWith("empty:")) {
        return {
          path,
          title: "Nueva pestaña",
          isDirty: false,
        };
      }
      const vaultItem = vaultItems.find((item) => item.relative_path === path);
      const isDirty = vaultItem
        ? loadedContents[path] !== undefined &&
          savedContents[path] !== undefined &&
          loadedContents[path] !== savedContents[path]
        : false;
      return {
        path,
        title: vaultItem ? vaultItem.title || vaultItem.relative_path : path,
        isDirty,
      };
    })
  );

  // Selección activa actual
  let currentSelection = $derived<SelectionInfo>(
    (activeTabPath && tabSelections[activeTabPath])
      ? tabSelections[activeTabPath]
      : defaultSelection
  );

  // Contadores calculados reactivamente sobre todo el documento
  let docWordCount = $derived(
    activeContent && activeContent.trim()
      ? activeContent.trim().split(/\s+/).length
      : 0
  );
  let docCharCount = $derived(activeContent ? activeContent.length : 0);

  // Dos comportamientos:
  // 1) Si no hay selección: cuenta sobre todo el documento y posición del cursor (Lín, Col)
  // 2) Si hay texto seleccionado: cuenta palabras, caracteres, líneas y columnas sobre la selección
  let displayWordCount = $derived(
    currentSelection.hasSelection ? currentSelection.selectedWords : docWordCount
  );
  let displayCharCount = $derived(
    currentSelection.hasSelection ? currentSelection.selectedChars : docCharCount
  );
  let displayLine = $derived(
    currentSelection.hasSelection ? currentSelection.selectedLines : currentSelection.cursorLine
  );
  let displayCol = $derived(
    currentSelection.hasSelection ? currentSelection.selectedCols : currentSelection.cursorCol
  );

  let editorContainerRef = $state<HTMLDivElement | null>(null);

  // Volver al inicio del documento y asegurar carga al cambiar de pestaña
  $effect(() => {
    const path = activeTabPath;
    if (path && !path.startsWith("empty:")) {
      ensureContentLoaded(path);
    }
    tick().then(() => {
      if (editorContainerRef) editorContainerRef.scrollTop = 0;
    });
  });

  // Carga únicamente de metadatos desde Rust (Tauri IPC) al montar
  onMount(() => {
    async function fetchNotesFromBackend() {
      if (isTauriEnvironment()) {
        try {
          const realNotes =
            await invokeTauri<
              Array<{
                relative_path: { 0?: string } | string;
                title: string;
                content?: string;
                encoding?: string;
              }>
            >("get_vault_notes");

          if (realNotes && Array.isArray(realNotes)) {
            isConnectedToRust = true;
            vaultItems = realNotes.map((n, index) => {
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

              return {
                id: String(index + 1),
                title: n.title,
                relative_path: relPath,
              };
            });

            if (recentFiles.length === 0 && vaultItems.length > 0) {
              recentFiles = vaultItems.slice(0, 10).map((v) => v.relative_path);
            }

            if (openTabPaths.length === 0) {
              handleNewEmptyTab();
            }
          } else {
            vaultItems = [];
            if (openTabPaths.length === 0) {
              handleNewEmptyTab();
            }
          }
        } catch (e) {
          console.warn("Error al cargar lista de archivos de Rust:", e);
          isConnectedToRust = false;
          vaultItems = [];
          if (openTabPaths.length === 0) {
            handleNewEmptyTab();
          }
        }
      } else {
        vaultItems = [];
        if (openTabPaths.length === 0) {
          handleNewEmptyTab();
        }
      }
    }

    // Si al arrancar no hay ningún tab seleccionado/abierto, abrir uno vacío
    if (openTabPaths.length === 0) {
      handleNewEmptyTab();
    }

    fetchNotesFromBackend();
  });

  // Estado de Auto-Guardado
  let autoSave = $state(true);
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  function debouncedPersistVaultItemToRust(vaultItem: VaultItem, delay = 600) {
    if (!autoSave) return;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      persistVaultItemToRust(vaultItem);
    }, delay);
  }

  async function persistVaultItemToRust(vaultItem: VaultItem, targetEncoding?: string) {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }
    const path = vaultItem.relative_path;
    if (!isConnectedToRust || !vaultItem.title || !path) return;
    const contentToSave = loadedContents[path] ?? "";
    const currentTabEnc = tabEncodings[path];
    const enc = targetEncoding || (currentTabEnc && currentTabEnc !== "---" ? currentTabEnc : "UTF-8");

    syncState = "saving";
    try {
      await invokeTauri("save_note_content", {
        relativePath: path,
        title: vaultItem.title,
        content: contentToSave,
        encoding: enc,
      });
      savedContents[path] = contentToSave;
      tabEncodings[path] = targetEncoding || (currentTabEnc && currentTabEnc !== "---" ? currentTabEnc : enc);
      syncState = "synced";
    } catch (e) {
      console.error("Error al guardar el archivo en Rust:", e);
      syncState = "error";
    }
  }

  async function handleChangeEncoding(newEncoding: string) {
    if (!activeTabPath || activeTabPath.startsWith("empty:")) {
      return;
    }
    tabEncodings[activeTabPath] = newEncoding;
    if (currentVaultItem && currentVaultItem.relative_path) {
      await persistVaultItemToRust(currentVaultItem, newEncoding);
    }
  }

  async function createNewVaultItem(emptyTabPathToReplace?: string) {
    const newTitle = `Nuevo Archivo ${vaultItems.length + 1}`;
    const newRelPath = `${newTitle}.md`;
    const newVaultItem: VaultItem = {
      id: String(vaultItems.length + 1),
      title: newTitle,
      relative_path: newRelPath,
    };
    const initialContent = "# Nuevo Archivo\n\nEscribe tu contenido aquí...";
    vaultItems.push(newVaultItem);
    loadedContents[newRelPath] = initialContent;
    tabEncodings[newRelPath] = "---";

    const targetEmptyPath = emptyTabPathToReplace || (activeTabPath && activeTabPath.startsWith("empty:") ? activeTabPath : null);
    if (targetEmptyPath && openTabPaths.includes(targetEmptyPath)) {
      const idx = openTabPaths.indexOf(targetEmptyPath);
      openTabPaths[idx] = newRelPath;
      delete tabSelections[targetEmptyPath];
    } else {
      openTabPaths.push(newRelPath);
    }
    activeTabPath = newRelPath;
    recentFiles = [newRelPath, ...recentFiles.filter((p) => p !== newRelPath)];
    await persistVaultItemToRust(newVaultItem);
  }

  // Registrar comandos por defecto al iniciar
  onMount(() => {
    commandRegistry.registerMany([
      {
        id: "cmd-new-tab",
        name: "Nueva pestaña vacía",
        category: "Pestañas",
        shortcut: "Ctrl+T",
        action: handleNewEmptyTab,
      },
      {
        id: "cmd-new-file",
        name: "Crear nuevo archivo / nota",
        category: "Archivo",
        shortcut: "Ctrl+N",
        action: () => createNewVaultItem(),
      },
      {
        id: "cmd-quick-open",
        name: "Abrir archivo rápidamente...",
        category: "Archivo",
        shortcut: "Ctrl+O",
        action: () => {
          isQuickOpenOpen = true;
        },
      },
      {
        id: "cmd-close-tab",
        name: "Cerrar pestaña activa",
        category: "Pestañas",
        shortcut: "Ctrl+W",
        action: () => {
          if (activeTabPath) closeTab(activeTabPath);
        },
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
          if (!isEditing) {
            markdownViewMode = "reading";
          } else if (markdownViewMode === "reading") {
            markdownViewMode = "live";
          }
        },
      },
      {
        id: "cmd-toggle-markdown-view",
        name: "Alternar vista Markdown (En vivo / Fuente / Lectura)",
        category: "Vista",
        action: toggleMarkdownViewMode,
      },
      {
        id: "cmd-save-file",
        name: "Guardar / Grabar archivo actual",
        category: "Archivo",
        shortcut: "Ctrl+S",
        action: () => {
          if (activeTabPath && currentVaultItem.relative_path) {
            persistVaultItemToRust(currentVaultItem);
          }
        },
      },
    ]);

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey) {
        const key = e.key.toLowerCase();
        if (key === 's') {
          e.preventDefault();
          if (activeTabPath && currentVaultItem.relative_path) {
            persistVaultItemToRust(currentVaultItem);
          }
        } else if (key === 'n') {
          e.preventDefault();
          createNewVaultItem();
        } else if (key === 'w') {
          e.preventDefault();
          if (activeTabPath) {
            closeTab(activeTabPath);
          }
        } else if (key === 'o') {
          e.preventDefault();
          isQuickOpenOpen = true;
        } else if (key === 'p') {
          e.preventDefault();
          isPaletteOpen = true;
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
      createNewVaultItem();
    }
  }

  async function handleOpenVaultFolder() {
    if (!isTauriEnvironment()) return;
    try {
      const newNotes = await invokeTauri<Array<{ relative_path: { 0?: string } | string; title: string; content?: string }> | null>('select_vault_folder');
      if (newNotes && Array.isArray(newNotes)) {
        vaultItems = newNotes.map((n, index) => {
          let relPath = `${n.title}.md`;
          if (typeof n.relative_path === 'string') {
            relPath = n.relative_path;
          } else if (n.relative_path && typeof n.relative_path === 'object' && n.relative_path[0]) {
            relPath = n.relative_path[0];
          }

          return {
            id: String(index + 1),
            title: n.title,
            relative_path: relPath
          };
        });
        openTabPaths = [];
        activeTabPath = null;
        loadedContents = {};
        savedContents = {};
        tabSelections = {};
        tabEncodings = {};
        recentFiles = vaultItems.map(v => v.relative_path);
        handleNewEmptyTab();
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
    {vaultItems}
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
      {isMarkdownFile}
      markdownViewMode={!isEditing ? "reading" : markdownViewMode}
      onChangeMarkdownView={handleChangeMarkdownView}
      title={activeTabPath?.startsWith("empty:") ? "Nueva pestaña" : (currentVaultItem.relative_path || currentVaultItem.title)}
      showSaveButton={isEditing &&
        !!activeTabPath &&
        !activeTabPath.startsWith("empty:") &&
        (!currentVaultItem.relative_path ||
          currentVaultItem.relative_path.endsWith(".md") ||
          currentVaultItem.relative_path.endsWith(".markdown") ||
          currentVaultItem.relative_path.endsWith(".excalidraw") ||
          currentVaultItem.relative_path.endsWith(".excalidraw.json"))}
      onSelectTab={(path) => selectTab(path)}
      onCloseTab={(path) => closeTab(path)}
      onCloseAllTabs={closeAllTabs}
      onNewTab={handleNewEmptyTab}
      onNewFile={() => createNewVaultItem()}
      onOpenQuickOpen={() => (isQuickOpenOpen = true)}
      onAction={(actionId) => {
        if (actionId === 'new-file') createNewVaultItem();
        else if (actionId === 'quick-open') isQuickOpenOpen = true;
      }}
      onToggleView={() => {
        if (!isEditing) {
          markdownViewMode = "reading";
        } else if (markdownViewMode === "reading") {
          markdownViewMode = "live";
        }
      }}
      onSave={() => {
        if (activeTabPath && currentVaultItem.relative_path) {
          persistVaultItemToRust(currentVaultItem);
        }
      }}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
    />

    <!-- CONTENEDOR DEL EDITOR CON MULTI-TAB Y CARGA BAJO DEMANDA -->
    <div class="editor-container" bind:this={editorContainerRef}>
      {#if openTabPaths.length === 0 || !activeTabPath}
        <EmptyWorkspace
          hasVaultItems={vaultItems.length > 0}
          onCreateNew={() => createNewVaultItem()}
        />
      {:else}
        {#each openTabPaths as tabPath (tabPath)}
          {#if tabPath.startsWith("empty:")}
            <div
              class="tab-pane"
              class:hidden={tabPath !== activeTabPath}
            >
              <EmptyWorkspace
                hasVaultItems={vaultItems.length > 0}
                onCreateNew={() => createNewVaultItem(tabPath)}
              />
            </div>
          {:else}
            {@const vaultItem = vaultItems.find((item) => item.relative_path === tabPath)}
            {@const content = loadedContents[tabPath]}
            {@const isLoading = loadingPaths[tabPath]}

            {#if vaultItem}
              <div
                class="tab-pane"
                class:hidden={tabPath !== activeTabPath}
                class:full-pane={tabPath.endsWith(".mmd") ||
                  tabPath.endsWith(".mermaid") ||
                  tabPath.endsWith(".png") ||
                  tabPath.endsWith(".webp") ||
                  tabPath.endsWith(".jpg") ||
                  tabPath.endsWith(".jpeg") ||
                  tabPath.endsWith(".gif") ||
                  tabPath.endsWith(".bmp") ||
                  tabPath.endsWith(".svg") ||
                  tabPath.endsWith(".ico") ||
                  tabPath.endsWith(".avif") ||
                  tabPath.endsWith(".excalidraw") ||
                  tabPath.endsWith(".excalidraw.json")}
              >
                {#if isLoading || content === undefined}
                  <div class="content-loading">
                    <span class="spinner"></span>
                    <span>Cargando contenido desde disco...</span>
                  </div>
                {:else if tabPath.endsWith(".mmd") || tabPath.endsWith(".mermaid")}
                  <MermanViewer
                    {content}
                    readOnly={!isEditing}
                    vimMode={isVimMode}
                    onChange={(updatedContent) => {
                      loadedContents[tabPath] = updatedContent;
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    onSelectionChange={(info: SelectionInfo) => handleSelectionChange(tabPath, info)}
                  />
                {:else if tabPath.endsWith(".excalidraw") || tabPath.endsWith(".excalidraw.json")}
                  <ExcalidrawViewer
                    {content}
                    readOnly={!isEditing}
                    onChange={(updatedContent) => {
                      loadedContents[tabPath] = updatedContent;
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                  />
                {:else if tabPath.endsWith(".md") || tabPath.endsWith(".markdown")}
                  <input
                    type="text"
                    class="editor-title-input"
                    bind:value={vaultItem.title}
                    oninput={() => persistVaultItemToRust(vaultItem)}
                    placeholder="Título del archivo..."
                  />

                  <div class="editor-main-content">
                    <MarkdownViewer
                      {content}
                      readOnly={!isEditing}
                      vimMode={isVimMode}
                      viewMode={!isEditing ? 'reading' : markdownViewMode}
                      onChange={(updatedMarkdown: string) => {
                        loadedContents[tabPath] = updatedMarkdown;
                        debouncedPersistVaultItemToRust(vaultItem);
                      }}
                      onSelectionChange={(info: SelectionInfo) => handleSelectionChange(tabPath, info)}
                      isMarkdown={!vaultItem.relative_path ||
                        vaultItem.relative_path.endsWith(".md") ||
                        vaultItem.relative_path.endsWith(".markdown")}
                    />
                  </div>
                {:else if tabPath.endsWith(".png") ||
                  tabPath.endsWith(".webp") ||
                  tabPath.endsWith(".jpg") ||
                  tabPath.endsWith(".jpeg") ||
                  tabPath.endsWith(".gif") ||
                  tabPath.endsWith(".bmp") ||
                  tabPath.endsWith(".svg") ||
                  tabPath.endsWith(".ico") ||
                  tabPath.endsWith(".avif")}
                  <ImageViewer src={`file:///${tabPath}`} alt={vaultItem.title || tabPath} />
                {:else}
                  <div class="editor-main-content">
                    <pre style="padding: 24px; font-family: var(--code-font, monospace); white-space: pre-wrap;">{content}</pre>
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        {/each}
      {/if}
    </div>

    <!-- BARRA DE ESTADO INFERIOR -->
    <StatusBar
      wordCount={vaultItems.length > 0 && activeTabPath ? displayWordCount : 0}
      charCount={vaultItems.length > 0 && activeTabPath ? displayCharCount : 0}
      line={vaultItems.length > 0 && activeTabPath ? displayLine : 0}
      col={vaultItems.length > 0 && activeTabPath ? displayCol : 0}
      hasSelection={vaultItems.length > 0 && !!activeTabPath && currentSelection.hasSelection}
      syncStatus={syncState}
      isVimMode={isVimMode}
      encoding={currentEncoding}
      isMarkdownFile={isMarkdownFile}
      markdownViewMode={!isEditing ? 'reading' : markdownViewMode}
      onToggleVim={() => (isVimMode = !isVimMode)}
      onToggleMarkdownView={toggleMarkdownViewMode}
      onChangeMarkdownView={handleChangeMarkdownView}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
      onChangeEncoding={handleChangeEncoding}
    />
  </main>

  <!-- 4. PALETA DE COMANDOS (OVERLAY CTRL+P) -->
  <CommandPalette bind:isOpen={isPaletteOpen} />

  <!-- 5. BUSCADOR RÁPIDO DE ARCHIVOS (OVERLAY CTRL+O) -->
  <QuickOpen
    bind:isOpen={isQuickOpenOpen}
    {vaultItems}
    {recentFiles}
    onSelectFile={(path) => selectTab(path)}
  />
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

  .content-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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
</style>
