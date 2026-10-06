<script lang="ts">
  import {onMount, tick} from "svelte";
  import {Ribbon} from "@widgets/ribbon";
  import {EditorHeader} from "@widgets/editor-header";
  import {StatusBar, type MarkdownViewMode} from "@widgets/status-bar";
  import {CommandPalette} from "@widgets/command-palette";
  import {QuickOpen} from "@widgets/quick-open";
  import {EmptyWorkspace} from "@widgets/empty-workspace";
  import {VaultExplorer} from "@features/vault-explorer";
  import {MarkdownViewer} from "@features/markdown-editor";
  import {MermanViewer} from "@features/merman-editor";
  import {MermaidViewer} from "@features/mermaid-editor";
  import {ExcalidrawViewer} from "@features/excalidraw-editor";
  import {ImageViewer} from "@features/image-viewer";
  import {appSettings} from "@entities/settings";
  import {
    isImageFile,
    isMarkdownFile,
    isDiagramFile,
    isDrawingFile,
  } from "@entities/file-type";
  import {loadSupportedFileTypesUseCase} from "@shared/use-cases";
  import type {VaultItem, OpenedNote} from "@entities/vault-item";
  import {commandRegistry} from "@entities/command";
  import {vaultRepository, toggleDevtools, type GitFileStatusKind} from "@shared/repositories";

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state("files");
  let isPaletteOpen = $state(false);
  let isQuickOpenOpen = $state(false);
  let isEditing = $state(false);
  let isVimMode = $state(false);
  let markdownViewMode = $state<MarkdownViewMode>("reading");
  let isConnectedToRust = $state(false);
  let syncState = $state<"synced" | "saving" | "error">("synced");

  // Estado de control de versiones Git
  let isGitRepo = $state(false);
  let gitBranch = $state<string | null>(null);
  let gitStatuses = $state<Record<string, GitFileStatusKind>>({});

  async function refreshGitStatus() {
    if (!vaultRepository.isConnected()) return;
    try {
      const status = await vaultRepository.getGitStatus();
      if (status && status.is_repo) {
        isGitRepo = true;
        gitBranch = status.branch ?? null;
        gitStatuses = status.statuses ?? {};
      } else {
        isGitRepo = false;
        gitBranch = null;
        gitStatuses = {};
      }
    } catch (e) {
      console.warn("Error al refrescar estado de Git:", e);
    }
  }

  async function handleGitRestoreFiles(paths: string[]) {
    for (const p of paths) {
      if (openedNotes[p]) {
        try {
          const noteData = await vaultRepository.readNote(p);
          if (noteData) {
            openedNotes[p] = {
              ...openedNotes[p],
              content: noteData.content ?? "",
              savedContent: noteData.content ?? "",
              isLoading: false,
            };
          }
        } catch (e) {
          console.error("Error al recargar archivo restaurado:", e);
        }
      }
    }
    await refreshGitStatus();
  }

  function getInitialSidebarWidth(): number {
    try {
      const saved = localStorage.getItem('synapse_sidebar_width');
      if (saved) {
        const parsed = parseInt(saved, 10);
        if (!isNaN(parsed) && parsed >= 140 && parsed <= 900) return parsed;
      }
    } catch {}
    return 240;
  }

  async function restoreSidebarWidth() {
    try {
      const uiState = await vaultRepository.getVaultUiState();
      if (uiState?.sidebar_width && uiState.sidebar_width >= 140 && uiState.sidebar_width <= 900) {
        sidebarWidth = uiState.sidebar_width;
      }
    } catch (e) {
      console.error("Error al restaurar ancho del panel lateral:", e);
    }
  }

  // Lista de metadatos de elementos de la bóveda (VaultItem[])
  let vaultItems = $state<VaultItem[]>([]);
  let openTabPaths = $state<string[]>([]);
  let activeTabPath = $state<string | null>(null);
  let sidebarWidth = $state<number>(getInitialSidebarWidth());
  let isResizingSidebar = $state(false);

  // Lista de rutas de archivos abiertos recientemente para Ctrl+O (Quick Open)
  let recentFiles = $state<string[]>([]);

  // Mapa reactivo unificado de notas abiertas bajo demanda con todos sus datos consolidados
  let openedNotes = $state<Record<string, OpenedNote>>({});

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

  // Límite máximo de entradas en el historial para control óptimo de memoria (~5-10 KB)
  const MAX_TAB_HISTORY = 100;

  // Historial de navegación de pestañas (Back / Forward)
  let tabHistory = $state<string[]>([]);
  let tabHistoryIndex = $state<number>(-1);
  let isNavigatingHistory = false;
  let lastNavTime = 0;

  // Registro de la posición física (índice en la barra de pestañas) de los archivos cerrados
  let lastClosedTabIndex = $state<Record<string, number>>({});

  let canGoBack = $derived(tabHistoryIndex > 0);

  let canGoForward = $derived(
    tabHistoryIndex >= 0 && tabHistoryIndex < tabHistory.length - 1
  );

  function recordTabVisit(path: string) {
    if (isNavigatingHistory || !path) return;
    if (tabHistoryIndex >= 0 && tabHistoryIndex < tabHistory.length && tabHistory[tabHistoryIndex] === path) {
      return;
    }
    let truncated = tabHistory.slice(0, tabHistoryIndex + 1);
    truncated.push(path);
    if (truncated.length > MAX_TAB_HISTORY) {
      truncated = truncated.slice(truncated.length - MAX_TAB_HISTORY);
    }
    tabHistory = truncated;
    tabHistoryIndex = truncated.length - 1;
  }

  function navigateBack() {
    if (tabHistoryIndex <= 0) return;
    const targetIndex = tabHistoryIndex - 1;
    tabHistoryIndex = targetIndex;
    const targetPath = tabHistory[targetIndex];
    isNavigatingHistory = true;
    selectTab(targetPath);
    isNavigatingHistory = false;
  }

  function navigateForward() {
    if (tabHistoryIndex >= tabHistory.length - 1) return;
    const targetIndex = tabHistoryIndex + 1;
    tabHistoryIndex = targetIndex;
    const targetPath = tabHistory[targetIndex];
    isNavigatingHistory = true;
    selectTab(targetPath);
    isNavigatingHistory = false;
  }

  function safeNavigateBack() {
    const now = performance.now();
    if (now - lastNavTime < 150) return;
    lastNavTime = now;
    navigateBack();
  }

  function safeNavigateForward() {
    const now = performance.now();
    if (now - lastNavTime < 150) return;
    lastNavTime = now;
    navigateForward();
  }

  function removeTabFromHistory(path: string) {
    const newHistory: string[] = [];
    let newIndex = -1;
    for (let i = 0; i < tabHistory.length; i++) {
      const item = tabHistory[i];
      if (item !== path) {
        if (newHistory.length === 0 || newHistory[newHistory.length - 1] !== item) {
          if (i <= tabHistoryIndex) {
            newIndex = newHistory.length;
          }
          newHistory.push(item);
        }
      }
    }
    tabHistory = newHistory;
    if (tabHistory.length === 0) {
      tabHistoryIndex = -1;
    } else {
      tabHistoryIndex = Math.max(0, Math.min(newIndex, tabHistory.length - 1));
      if (activeTabPath) {
        const found = tabHistory.lastIndexOf(activeTabPath);
        if (found !== -1) {
          tabHistoryIndex = found;
        }
      }
    }
  }

  let saveTabsTimeout: ReturnType<typeof setTimeout> | null = null;
  function persistTabsState() {
    if (!isConnectedToRust) return;
    if (saveTabsTimeout) clearTimeout(saveTabsTimeout);
    saveTabsTimeout = setTimeout(() => {
      const tabs = openTabPaths
        .filter((p) => p && !p.startsWith("empty:"))
        .map((p) => ({
          path: p,
          view_mode: openedNotes[p]?.viewMode || "reading",
        }));
      const active = activeTabPath && !activeTabPath.startsWith("empty:") ? activeTabPath : null;
      vaultRepository.saveOpenTabsState(tabs, active);
    }, 250);
  }

  async function restoreOpenTabsState(): Promise<boolean> {
    try {
      const state = await vaultRepository.getOpenTabsState();
      if (state && state.open_tabs && state.open_tabs.length > 0) {
        openTabPaths = [];
        for (const tab of state.open_tabs) {
          if (!tab.path) continue;
          openTabPaths.push(tab.path);
          const mode = (tab.view_mode as MarkdownViewMode) || "reading";
          const vItem = vaultItems.find((v) => v.relative_path === tab.path);
          if (!openedNotes[tab.path]) {
            openedNotes[tab.path] = {
              relative_path: tab.path,
              abs_path: vItem?.abs_path,
              title: vItem?.title || tab.path,
              content: "",
              savedContent: "",
              encoding: "---",
              isLoading: false,
              viewMode: mode,
            };
          } else {
            openedNotes[tab.path].viewMode = mode;
          }
        }

        if (openTabPaths.length > 0) {
          const targetActive = state.active_tab && openTabPaths.includes(state.active_tab)
            ? state.active_tab
            : openTabPaths[0];

          activeTabPath = targetActive;
          const activeMode = openedNotes[targetActive]?.viewMode || "reading";
          markdownViewMode = activeMode;
          isEditing = activeMode !== "reading";
          ensureContentLoaded(targetActive);
          return true;
        }
      }
    } catch (e) {
      console.warn("Error al restaurar pestañas abiertas desde DashMap:", e);
    }
    return false;
  }

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
    if (activeTabPath && !activeTabPath.startsWith("empty:")) {
      if (openedNotes[activeTabPath]) {
        openedNotes[activeTabPath].viewMode = newMode;
      }
      persistTabsState();
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
    if (!path || path.startsWith("empty:")) return;
    if (openedNotes[path] && !openedNotes[path].isLoading && openedNotes[path].content !== undefined) return;

    const vaultItem = vaultItems.find((v) => v.relative_path === path);
    const initialAbsPath = vaultItem?.abs_path;

    // Para imágenes no se requiere leer contenido como texto; resolveAssetUrl se encarga
    if (isImageFile(path)) {
      const currentMode = openedNotes[path]?.viewMode || "reading";
      openedNotes[path] = {
        relative_path: path,
        abs_path: initialAbsPath,
        title: vaultItem?.title || path,
        content: "",
        savedContent: "",
        encoding: "binary",
        isLoading: false,
        viewMode: currentMode,
      };
      return;
    }

    if (!openedNotes[path]) {
      openedNotes[path] = {
        relative_path: path,
        abs_path: initialAbsPath,
        title: vaultItem?.title || path,
        content: "",
        savedContent: "",
        encoding: "---",
        isLoading: true,
        viewMode: "reading",
      };
    } else {
      openedNotes[path].isLoading = true;
    }

    try {
      const noteData = await vaultRepository.readNote(path);
      if (noteData) {
        const fetchedContent = noteData.content ?? "";
        const fetchedEncoding = noteData.encoding && noteData.encoding.trim() !== "" ? noteData.encoding : "---";
        const fetchedAbsPath = noteData.abs_path || initialAbsPath;
        const currentMode = openedNotes[path]?.viewMode || "reading";

        openedNotes[path] = {
          relative_path: path,
          abs_path: fetchedAbsPath,
          title: noteData.title || vaultItem?.title || path,
          content: fetchedContent,
          savedContent: fetchedContent,
          encoding: fetchedEncoding,
          isLoading: false,
          viewMode: currentMode,
        };
      } else {
        if (openedNotes[path]) {
          openedNotes[path].isLoading = false;
        }
      }
    } catch (e) {
      console.error(`Error al cargar contenido de ${path} desde la bóveda:`, e);
      if (openedNotes[path]) {
        openedNotes[path].isLoading = false;
      }
    }
  }

  function handleNewEmptyTab() {
    emptyTabCounter += 1;
    const emptyTabPath = `empty://${emptyTabCounter}-${Date.now()}`;
    openTabPaths.push(emptyTabPath);
    activeTabPath = emptyTabPath;
    openedNotes[emptyTabPath] = {
      relative_path: emptyTabPath,
      abs_path: undefined,
      title: "Nueva pestaña",
      content: "",
      savedContent: "",
      encoding: "---",
      isLoading: false,
    };
    recordTabVisit(emptyTabPath);
  }

  function selectTab(path: string) {
    if (!path) return;
    const previousActivePath = activeTabPath;

    // Si la pestaña actual es una pestaña vacía y seleccionamos un archivo nuevo, lo sustituye en esa pestaña
    if (activeTabPath && activeTabPath.startsWith("empty:") && !openTabPaths.includes(path)) {
      const idx = openTabPaths.indexOf(activeTabPath);
      if (idx !== -1) {
        openTabPaths[idx] = path;
        delete openedNotes[activeTabPath];
        delete tabSelections[activeTabPath];
        if (previousActivePath) {
          tabHistory = tabHistory.map((p) => (p === previousActivePath ? path : p));
        }
      } else {
        openTabPaths.push(path);
      }
    } else if (!openTabPaths.includes(path)) {
      const savedIndex = lastClosedTabIndex[path];
      if (savedIndex !== undefined && savedIndex >= 0) {
        const insertAt = Math.min(savedIndex, openTabPaths.length);
        openTabPaths.splice(insertAt, 0, path);
      } else {
        openTabPaths.push(path);
      }
    }
    activeTabPath = path;
    if (!path.startsWith("empty:")) {
      const mode = openedNotes[path]?.viewMode || "reading";
      markdownViewMode = mode;
      isEditing = mode !== "reading";
      ensureContentLoaded(path);
      recentFiles = [path, ...recentFiles.filter((p) => p !== path)].slice(0, 15);
      vaultRepository.recordNoteOpened(path);
    }
    recordTabVisit(path);
    persistTabsState();
  }

  function closeTab(path: string) {
    const idx = openTabPaths.indexOf(path);
    if (idx !== -1) {
      openTabPaths.splice(idx, 1);
      // Guardar la posición previa en la barra de pestañas para reabrirla exactamente donde estaba
      if (!path.startsWith("empty:")) {
        lastClosedTabIndex[path] = idx;
      }

      if (activeTabPath === path) {
        if (openTabPaths.length > 0) {
          const nextIdx = Math.min(idx, openTabPaths.length - 1);
          activeTabPath = openTabPaths[nextIdx];
          if (activeTabPath && !activeTabPath.startsWith("empty:")) {
            const nextMode = openedNotes[activeTabPath]?.viewMode || "reading";
            markdownViewMode = nextMode;
            isEditing = nextMode !== "reading";
            ensureContentLoaded(activeTabPath);
          }
        } else {
          activeTabPath = null;
        }

        // Si la pestaña cerrada era la activa, sincronizar tabHistoryIndex con la nueva pestaña activa
        if (activeTabPath && !path.startsWith("empty:")) {
          const foundIdx = tabHistory.slice(0, tabHistoryIndex).lastIndexOf(activeTabPath);
          if (foundIdx !== -1) {
            tabHistoryIndex = foundIdx;
          }
        }
      }
    }
    // Liberar memoria consolidada del contenido de la nota para mantener bajo consumo de RAM
    delete openedNotes[path];
    // Se conserva tabSelections[path] para preservar la posición del cursor si se reabre

    // Solo las pestañas temporales vacías se retiran del historial
    if (path.startsWith("empty:")) {
      removeTabFromHistory(path);
      delete lastClosedTabIndex[path];
      delete tabSelections[path];
    }

    // Si no queda ningún tab abierto, crear automáticamente una pestaña vacía
    if (openTabPaths.length === 0) {
      handleNewEmptyTab();
    }
    persistTabsState();
  }

  function closeAllTabs() {
    openTabPaths = [];
    activeTabPath = null;
    openedNotes = {};
    tabSelections = {};
    tabHistory = [];
    tabHistoryIndex = -1;
    handleNewEmptyTab();
    persistTabsState();
  }

  let activeNote = $derived(
    activeTabPath ? openedNotes[activeTabPath] : undefined
  );

  let currentVaultItem = $derived(
    activeTabPath && !activeTabPath.startsWith("empty:") && vaultItems.length > 0
      ? vaultItems.find((vaultItem) => vaultItem.relative_path === activeTabPath) || {
      id: "0",
      title: activeNote?.title || "",
      relative_path: activeTabPath,
      abs_path: activeNote?.abs_path,
    }
      : {id: "0", title: "", relative_path: "", abs_path: undefined}
  );

  let activeContent = $derived(
    activeNote ? activeNote.content : ""
  );

  let currentEncoding = $derived(
    activeNote ? activeNote.encoding : "---"
  );

  let isMarkdownTab = $derived(
    Boolean(
      activeTabPath &&
      !activeTabPath.startsWith("empty:") &&
      (
        isMarkdownFile(activeTabPath) ||
        (currentVaultItem.relative_path && isMarkdownFile(currentVaultItem.relative_path))
      )
    )
  );

  let hasActiveContent = $derived(
    Boolean(
      activeTabPath &&
      openTabPaths.includes(activeTabPath) &&
      !isImageFile(activeTabPath) &&
      activeNote &&
      !activeNote.isLoading &&
      typeof activeNote.content === "string" &&
      activeNote.content.trim().length > 0
    )
  );

  let tabsInfo = $derived(
    openTabPaths.map((path) => {
      if (path.startsWith("empty:")) {
        return {
          path,
          title: "Nueva pestaña",
          abs_path: undefined,
          isDirty: false,
        };
      }
      const vaultItem = vaultItems.find((item) => item.relative_path === path);
      const note = openedNotes[path];
      const isDirty = note ? note.content !== note.savedContent : false;
      return {
        path,
        abs_path: note?.abs_path || vaultItem?.abs_path,
        title: note?.title || vaultItem?.title || path,
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

  async function fetchNotesFromBackend() {
    try {
      if (appSettings.lastOpenedFolder) {
        try {
          await vaultRepository.setActiveVaultPath(appSettings.lastOpenedFolder);
        } catch (e) {
          console.warn("No se pudo configurar la ruta de la bóveda desde settings:", e);
          try {
            const currentPath = await vaultRepository.getActiveVaultPath();
            if (currentPath) {
              appSettings.setLastOpenedFolder(currentPath);
            }
          } catch {
            // ignorar si no está disponible
          }
        }
      } else {
        // Si no hay setting guardado aún, sincronizar el path actual de Rust en settings
        try {
          const currentPath = await vaultRepository.getActiveVaultPath();
          if (currentPath) {
            appSettings.setLastOpenedFolder(currentPath);
          }
        } catch {
          // ignorar si no está disponible
        }
      }

      const notes = await vaultRepository.getNotes();
      isConnectedToRust = vaultRepository.isConnected();
      await refreshGitStatus();

      if (notes && Array.isArray(notes) && notes.length > 0) {
        vaultItems = notes.map((n, index) => {
          let relPath = `${n.title}.md`;
          if (typeof n.relative_path === "string") {
            relPath = n.relative_path;
          } else if (
            n.relative_path &&
            typeof n.relative_path === "object" &&
            (n.relative_path as unknown as string[])[0]
          ) {
            relPath = (n.relative_path as unknown as string[])[0];
          }

          return {
            id: String(index + 1),
            title: n.title,
            relative_path: relPath,
            abs_path: n.abs_path,
          };
        });

        if (recentFiles.length === 0) {
          const loadedRecent = await vaultRepository.getRecentNotes(15);
          if (loadedRecent && loadedRecent.length > 0) {
            recentFiles = loadedRecent;
          }
        }

        if (openTabPaths.length === 0) {
          await restoreSidebarWidth();
          const restored = await restoreOpenTabsState();
          if (!restored) {
            handleNewEmptyTab();
          }
          await refreshGitStatus();
        }
      } else {
        vaultItems = [];
        if (openTabPaths.length === 0) {
          handleNewEmptyTab();
        }
      }
    } catch (e) {
      console.warn("Error al cargar lista de archivos de la bóveda:", e);
      isConnectedToRust = false;
      vaultItems = [];
      if (openTabPaths.length === 0) {
        handleNewEmptyTab();
      }
    }
  }

  async function handleDeleteItem(relativePath: string, isFolder: boolean) {
    try {
      await vaultRepository.deleteItem(relativePath);

      // Cerrar las pestañas abiertas que correspondan al archivo o estén dentro de la carpeta eliminada
      const tabsToClose = openTabPaths.filter((path) => {
        if (isFolder) {
          return path === relativePath || path.startsWith(`${relativePath}/`);
        }
        return path === relativePath;
      });

      for (const path of tabsToClose) {
        closeTab(path);
      }
      removeTabFromHistory(relativePath);
      delete lastClosedTabIndex[relativePath];
      delete tabSelections[relativePath];

      // Actualizar la lista de archivos de la bóveda y el estado de Git
      await fetchNotesFromBackend();
    } catch (e) {
      console.error("Error al eliminar elemento de la bóveda:", e);
      throw e;
    }
  }

  async function handleDeleteItems(items: Array<{ relativePath: string; isFolder: boolean }>) {
    try {
      for (const item of items) {
        await vaultRepository.deleteItem(item.relativePath);

        const tabsToClose = openTabPaths.filter((path) => {
          if (item.isFolder) {
            return path === item.relativePath || path.startsWith(`${item.relativePath}/`);
          }
          return path === item.relativePath;
        });

        for (const path of tabsToClose) {
          closeTab(path);
        }
        removeTabFromHistory(item.relativePath);
        delete lastClosedTabIndex[item.relativePath];
        delete tabSelections[item.relativePath];
      }

      await fetchNotesFromBackend();
    } catch (e) {
      console.error("Error al eliminar elementos de la bóveda:", e);
      throw e;
    }
  }

  // Carga únicamente de metadatos desde la bóveda (repositorio) al montar
  onMount(() => {
    window.scrollTo(0, 0);

    // Cargar la configuración de tipos soportados a través del use case
    loadSupportedFileTypesUseCase();

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

    // Proteger archivos binarios o imágenes de sobreescrituras accidentales
    if (isImageFile(path)) return;

    const note = openedNotes[path];
    const contentToSave = note ? note.content : "";
    const currentTabEnc = note ? note.encoding : "---";
    const enc = targetEncoding || (currentTabEnc && currentTabEnc !== "---" ? currentTabEnc : "UTF-8");

    syncState = "saving";
    try {
      await vaultRepository.saveNote({
        relativePath: path,
        title: vaultItem.title,
        content: contentToSave,
        encoding: enc,
      });
      if (openedNotes[path]) {
        openedNotes[path].savedContent = contentToSave;
        if (targetEncoding) {
          openedNotes[path].encoding = targetEncoding;
        }
      }
      syncState = "synced";
      refreshGitStatus();
    } catch (e) {
      console.error("Error al guardar el archivo en la bóveda:", e);
      syncState = "error";
    }
  }

  async function handleChangeEncoding(newEncoding: string) {
    if (!activeTabPath || activeTabPath.startsWith("empty:")) {
      return;
    }
    if (openedNotes[activeTabPath]) {
      openedNotes[activeTabPath].encoding = newEncoding;
    }
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
      abs_path: undefined,
    };
    const initialContent = "# Nuevo Archivo\n\nEscribe tu contenido aquí...";
    vaultItems.push(newVaultItem);
    openedNotes[newRelPath] = {
      relative_path: newRelPath,
      abs_path: undefined,
      title: newTitle,
      content: initialContent,
      savedContent: "",
      encoding: "---",
      isLoading: false,
    };

    const targetEmptyPath = emptyTabPathToReplace || (activeTabPath && activeTabPath.startsWith("empty:") ? activeTabPath : null);
    if (targetEmptyPath && openTabPaths.includes(targetEmptyPath)) {
      const idx = openTabPaths.indexOf(targetEmptyPath);
      openTabPaths[idx] = newRelPath;
      delete openedNotes[targetEmptyPath];
      delete tabSelections[targetEmptyPath];
      tabHistory = tabHistory.map((p) => (p === targetEmptyPath ? newRelPath : p));
    } else {
      openTabPaths.push(newRelPath);
    }
    activeTabPath = newRelPath;
    recentFiles = [newRelPath, ...recentFiles.filter((p) => p !== newRelPath)].slice(0, 15);
    recordTabVisit(newRelPath);
    await persistVaultItemToRust(newVaultItem);
    refreshGitStatus();
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
        id: "cmd-open-folder",
        name: "Abrir carpeta / bóveda",
        category: "Archivo",
        shortcut: "Ctrl+Shift+O",
        action: handleOpenVaultFolder,
      },
      {
        id: "cmd-new-file",
        name: "Crear nuevo archivo",
        category: "Archivo",
        shortcut: "Ctrl+N",
        action: () => createNewVaultItem(),
      },
      {
        id: "cmd-close-tab",
        name: "Cerrar pestaña actual",
        category: "Pestañas",
        shortcut: "Ctrl+W, Ctrl+F4",
        action: () => {
          if (activeTabPath) closeTab(activeTabPath);
        },
      },
      {
        id: "cmd-close-all-tabs",
        name: "Cerrar todas las pestañas",
        category: "Pestañas",
        action: closeAllTabs,
      },
      {
        id: "cmd-toggle-vim",
        name: "Alternar modo Vim",
        category: "Editor",
        action: () => {
          isVimMode = !isVimMode;
        },
      },
      {
        id: "cmd-toggle-markdown-view",
        name: "Alternar vista Markdown (En vivo / Fuente / Lectura)",
        category: "Vista",
        action: toggleMarkdownViewMode,
      },
      {
        id: "cmd-toggle-sidebar",
        name: "Mostrar u ocultar explorador de archivos",
        category: "Vista",
        shortcut: "Alt+1, Ctrl+B",
        action: toggleExplorer,
      },
      {
        id: "cmd-toggle-mermaid-engine",
        name: "Alternar motor de diagramas Mermaid (Mermaid.js / Merman)",
        category: "Configuración",
        action: () => {
          appSettings.toggleMermaidRenderer();
        },
      },
      {
        id: "cmd-set-mermaid-engine-mermaidjs",
        name: "Configurar motor Mermaid: Usar Mermaid.js",
        category: "Configuración",
        action: () => {
          appSettings.setMermaidRenderer("mermaidjs");
        },
      },
      {
        id: "cmd-set-mermaid-engine-merman",
        name: "Configurar motor Mermaid: Usar Merman (WASM)",
        category: "Configuración",
        action: () => {
          appSettings.setMermaidRenderer("merman");
        },
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
      {
        id: "cmd-toggle-devtools",
        name: "Alternar herramientas de desarrollo (DevTools)",
        category: "Desarrollo",
        shortcut: "F12",
        action: toggleDevtools,
      },
      {
        id: "cmd-refresh-git",
        name: "Actualizar estado de Git",
        category: "Git",
        action: refreshGitStatus,
      },
      {
        id: "cmd-git-add-active",
        name: "Git: Preparar archivo actual (git add)",
        category: "Git",
        action: async () => {
          if (!activeTabPath || activeTabPath.startsWith("empty:")) return;
          try {
            await vaultRepository.gitAdd([activeTabPath]);
            await refreshGitStatus();
          } catch (e) {
            console.error("Error en git add:", e);
          }
        },
      },
      {
        id: "cmd-git-restore-staged-active",
        name: "Git: Despreparar archivo actual (git restore --staged)",
        category: "Git",
        action: async () => {
          if (!activeTabPath || activeTabPath.startsWith("empty:")) return;
          try {
            await vaultRepository.gitRestoreStaged([activeTabPath]);
            await refreshGitStatus();
          } catch (e) {
            console.error("Error en git restore --staged:", e);
          }
        },
      },
      {
        id: "cmd-git-restore-active",
        name: "Git: Restaurar archivo actual (git restore)",
        category: "Git",
        action: async () => {
          if (!activeTabPath || activeTabPath.startsWith("empty:")) return;
          try {
            await vaultRepository.gitRestore([activeTabPath]);
            await handleGitRestoreFiles([activeTabPath]);
          } catch (e) {
            console.error("Error en git restore:", e);
          }
        },
      },
      {
        id: "cmd-nav-back",
        name: "Navegar atrás entre pestañas",
        category: "Navegación",
        shortcut: "Alt+Left",
        action: navigateBack,
      },
      {
        id: "cmd-nav-forward",
        name: "Navegar adelante entre pestañas",
        category: "Navegación",
        shortcut: "Alt+Right",
        action: navigateForward,
      },
      {
        id: "cmd-toggle-theme",
        name: "Alternar tema (Claro / Oscuro)",
        category: "Apariencia",
        shortcut: "Ctrl+Shift+T",
        action: () => appSettings.toggleTheme(),
      },
      {
        id: "cmd-theme-system",
        name: "Tema: Seguir sistema operativo (Auto)",
        category: "Apariencia",
        action: () => appSettings.setTheme('system'),
      },
      {
        id: "cmd-theme-dark",
        name: "Tema: Modo oscuro",
        category: "Apariencia",
        action: () => appSettings.setTheme('dark'),
      },
      {
        id: "cmd-theme-light",
        name: "Tema: Modo claro",
        category: "Apariencia",
        action: () => appSettings.setTheme('light'),
      },
    ]);

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F12") {
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
        toggleDevtools();
        return;
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
        if (e.key === '1' || e.code === 'Digit1' || e.code === 'Numpad1') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          toggleExplorer();
          return;
        } else if (e.key === 'ArrowLeft') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          navigateBack();
          return;
        } else if (e.key === 'ArrowRight') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          navigateForward();
          return;
        }
      }
      if ((e.ctrlKey || e.metaKey) && !e.altKey) {
        const key = e.key.toLowerCase();
        if (e.shiftKey) {
          if (key === 'o') {
            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();
            handleOpenVaultFolder();
            return;
          } else if (key === 'i') {
            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();
            toggleDevtools();
            return;
          } else if (key === 't') {
            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();
            appSettings.toggleTheme();
            return;
          }
          return;
        }
        if (key === 's') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          if (activeTabPath && currentVaultItem.relative_path) {
            persistVaultItemToRust(currentVaultItem);
          }
          return;
        } else if (key === 'n') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          createNewVaultItem();
          return;
        } else if (key === 't') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          handleNewEmptyTab();
          return;
        } else if (key === 'w' || key === 'f4') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          if (activeTabPath) {
            closeTab(activeTabPath);
          }
          return;
        } else if (key === 'o') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          isQuickOpenOpen = true;
          return;
        } else if (key === 'p') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          isPaletteOpen = true;
          return;
        } else if (key === 'b') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          toggleExplorer();
          return;
        }
      }
    };

    // Control de navegación con botones laterales del ratón (Back / Forward)
    // Código Wayland / Linux: 275 (BTN_SIDE) -> Mouse button 3
    // Código Wayland / Linux: 276 (BTN_EXTRA) -> Mouse button 4
    const handleMouseNavDown = (e: MouseEvent | PointerEvent) => {
      if (e.button === 3 || e.button === 4) {
        e.preventDefault();
      }
    };

    const handleMouseNavUp = (e: MouseEvent | PointerEvent) => {
      if (e.button === 3) {
        e.preventDefault();
        e.stopPropagation();
        safeNavigateBack();
      } else if (e.button === 4) {
        e.preventDefault();
        e.stopPropagation();
        safeNavigateForward();
      }
    };

    const handleAuxClick = (e: MouseEvent) => {
      if (e.button === 3 || e.button === 4) {
        e.preventDefault();
        e.stopPropagation();
      }
    };

    const handleFocus = () => {
      refreshGitStatus();
    };
    window.addEventListener('focus', handleFocus);
    window.addEventListener('keydown', handleKeyDown, true);
    window.addEventListener('mousedown', handleMouseNavDown, true);
    window.addEventListener('mouseup', handleMouseNavUp, true);
    window.addEventListener('pointerup', handleMouseNavUp, true);
    window.addEventListener('auxclick', handleAuxClick, true);

    return () => {
      window.removeEventListener('focus', handleFocus);
      window.removeEventListener('keydown', handleKeyDown, true);
      window.removeEventListener('mousedown', handleMouseNavDown, true);
      window.removeEventListener('mouseup', handleMouseNavUp, true);
      window.removeEventListener('pointerup', handleMouseNavUp, true);
      window.removeEventListener('auxclick', handleAuxClick, true);
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
    try {
      const result = await vaultRepository.selectVaultFolder(appSettings.lastOpenedFolder);
      if (result) {
        if (result.folder_path) {
          appSettings.setLastOpenedFolder(result.folder_path);
        }
        const newNotes = result.notes || [];
        vaultItems = newNotes.map((n, index) => {
          let relPath = `${n.title}.md`;
          if (typeof n.relative_path === 'string') {
            relPath = n.relative_path;
          } else if (
            n.relative_path &&
            typeof n.relative_path === 'object' &&
            (n.relative_path as unknown as string[])[0]
          ) {
            relPath = (n.relative_path as unknown as string[])[0];
          }

          return {
            id: String(index + 1),
            title: n.title,
            relative_path: relPath,
            abs_path: n.abs_path,
          };
        });
        openTabPaths = [];
        activeTabPath = null;
        openedNotes = {};
        tabSelections = {};
        recentFiles = [];
        vaultRepository.getRecentNotes(15).then((recents) => {
          if (recents && recents.length > 0) {
            recentFiles = recents;
          }
        });
        tabHistory = [];
        tabHistoryIndex = -1;
        await restoreSidebarWidth();
        const restored = await restoreOpenTabsState();
        if (!restored) {
          handleNewEmptyTab();
        }
      }
    } catch (e) {
      console.error('Error al abrir la carpeta de la bóveda:', e);
    }
  }

  function toggleExplorer() {
    if (activeRibbonTab === 'files') {
      activeRibbonTab = '';
    } else {
      if (sidebarWidth < 140) {
        sidebarWidth = 240;
      }
      activeRibbonTab = 'files';
    }
  }

  function toggleSidebar() {
    toggleExplorer();
  }

  function handleSidebarResizeStart(e: MouseEvent | PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    isResizingSidebar = true;
    document.body.classList.add('is-resizing-col');

    const handlePointerMove = (ev: MouseEvent | PointerEvent) => {
      const newWidth = ev.clientX - 48; // 48px ancho del ribbon
      if (newWidth < 70) {
        // Colapsar si se arrastra hacia el extremo izquierdo
        activeRibbonTab = '';
      } else {
        if (!activeRibbonTab) {
          activeRibbonTab = 'files';
        }
        const maxWidth = Math.max(300, window.innerWidth - 200);
        sidebarWidth = Math.max(140, Math.min(newWidth, Math.min(900, maxWidth)));
      }
    };

    const handlePointerUp = () => {
      isResizingSidebar = false;
      document.body.classList.remove('is-resizing-col');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
      window.removeEventListener('mousemove', handlePointerMove);
      window.removeEventListener('mouseup', handlePointerUp);
      try {
        localStorage.setItem('synapse_sidebar_width', String(sidebarWidth));
        if (sidebarWidth >= 140) {
          vaultRepository.saveVaultUiState(sidebarWidth);
        }
      } catch {}
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
    window.addEventListener('mousemove', handlePointerMove);
    window.addEventListener('mouseup', handlePointerUp);
  }
</script>

<div class="workspace-layout">
  <!-- 1. BARRA RIBBON IZQUIERDA -->
  <Ribbon bind:activeTab={activeRibbonTab} onAction={handleRibbonAction}/>

  <!-- 2. PANEL LATERAL (EXPLORADOR DE ARCHIVOS DE LA BÓVEDA) -->
  <VaultExplorer
    {activeRibbonTab}
    {sidebarWidth}
    {isResizingSidebar}
    {isConnectedToRust}
    {vaultItems}
    {activeTabPath}
    vaultPath={appSettings.lastOpenedFolder}
    {gitStatuses}
    {isGitRepo}
    {gitBranch}
    onSelectTab={selectTab}
    onOpenVaultFolder={handleOpenVaultFolder}
    onDeleteItem={handleDeleteItem}
    onDeleteItems={handleDeleteItems}
    onResizeStart={handleSidebarResizeStart}
    onCollapse={toggleSidebar}
    onRefreshGit={refreshGitStatus}
    onGitRestore={handleGitRestoreFiles}
  />

  <!-- 3. ÁREA DE TRABAJO PRINCIPAL -->
  <main class="main-workspace" class:is-resizing={isResizingSidebar}>
    <!-- BARRA SUPERIOR DE PESTAÑAS Y HERRAMIENTAS -->
    <EditorHeader
      bind:isEditing
      tabs={tabsInfo}
      {activeTabPath}
      {canGoBack}
      {canGoForward}
      onNavigateBack={navigateBack}
      onNavigateForward={navigateForward}
      isMarkdownFile={isMarkdownTab}
      showViewToggle={hasActiveContent}
      markdownViewMode={!isEditing ? "reading" : markdownViewMode}
      onChangeMarkdownView={handleChangeMarkdownView}
      title={activeTabPath?.startsWith("empty:") ? "Nueva pestaña" : (currentVaultItem.relative_path || currentVaultItem.title)}
      showSaveButton={isEditing &&
        !!activeTabPath &&
        !activeTabPath.startsWith("empty:") &&
        (!currentVaultItem.relative_path ||
          isMarkdownFile(currentVaultItem.relative_path) ||
          isDrawingFile(currentVaultItem.relative_path))}
      onSelectTab={(path) => selectTab(path)}
      onCloseTab={(path) => closeTab(path)}
      onCloseAllTabs={closeAllTabs}
      onNewTab={handleNewEmptyTab}
      onNewFile={() => createNewVaultItem()}
      onOpenQuickOpen={() => (isQuickOpenOpen = true)}
      onAction={(actionId) => {
        if (actionId === 'nav-back') navigateBack();
        else if (actionId === 'nav-forward') navigateForward();
        else if (actionId === 'new-file') createNewVaultItem();
        else if (actionId === 'quick-open') isQuickOpenOpen = true;
      }}
      onToggleView={() => {
        toggleMarkdownViewMode();
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
            {@const vaultItem = vaultItems.find((item) => item.relative_path === tabPath) || {
              id: "0",
              title: openedNotes[tabPath]?.title || tabPath,
              relative_path: tabPath,
              abs_path: openedNotes[tabPath]?.abs_path,
            }}
            {@const note = openedNotes[tabPath]}
            {@const content = note?.content ?? ""}
            {@const isLoading = note?.isLoading ?? false}

            <div
              class="tab-pane"
              class:hidden={tabPath !== activeTabPath}
              class:full-pane={isDiagramFile(tabPath) ||
                isImageFile(tabPath) ||
                isDrawingFile(tabPath)}
            >
              {#if isLoading}
                <div class="content-loading">
                  <span class="spinner"></span>
                  <span>Cargando contenido desde disco...</span>
                </div>
              {:else if isDiagramFile(tabPath)}
                {#if appSettings.mermaidRenderer === 'mermaidjs'}
                  <MermaidViewer
                    {content}
                    readOnly={!isEditing}
                    vimMode={isVimMode}
                    onChange={(updatedContent) => {
                      if (openedNotes[tabPath]) {
                        openedNotes[tabPath].content = updatedContent;
                      }
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    onSelectionChange={(info: SelectionInfo) => handleSelectionChange(tabPath, info)}
                  />
                {:else}
                  <MermanViewer
                    {content}
                    readOnly={!isEditing}
                    vimMode={isVimMode}
                    onChange={(updatedContent) => {
                      if (openedNotes[tabPath]) {
                        openedNotes[tabPath].content = updatedContent;
                      }
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    onSelectionChange={(info: SelectionInfo) => handleSelectionChange(tabPath, info)}
                  />
                {/if}
              {:else if isDrawingFile(tabPath)}
                <ExcalidrawViewer
                  {content}
                  readOnly={!isEditing}
                  onChange={(updatedContent) => {
                    if (openedNotes[tabPath]) {
                      openedNotes[tabPath].content = updatedContent;
                    }
                    debouncedPersistVaultItemToRust(vaultItem);
                  }}
                />
              {:else if isMarkdownFile(tabPath)}
                <input
                  type="text"
                  class="editor-title-input"
                  bind:value={vaultItem.title}
                  oninput={() => {
                    if (openedNotes[tabPath]) {
                      openedNotes[tabPath].title = vaultItem.title;
                    }
                    persistVaultItemToRust(vaultItem);
                  }}
                  placeholder="Título del archivo..."
                />

                <div class="editor-main-content">
                  <MarkdownViewer
                    {content}
                    filePath={tabPath}
                    readOnly={!isEditing}
                    vimMode={isVimMode}
                    viewMode={!isEditing ? 'reading' : markdownViewMode}
                    onChange={(updatedMarkdown: string) => {
                      if (openedNotes[tabPath]) {
                        openedNotes[tabPath].content = updatedMarkdown;
                      }
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    onSelectionChange={(info: SelectionInfo) => handleSelectionChange(tabPath, info)}
                    isMarkdown={true}
                  />
                </div>
              {:else if isImageFile(tabPath)}
                <ImageViewer
                  src={note?.abs_path ? vaultRepository.resolveAssetUrl(note.abs_path) : (content || tabPath)}
                  alt={vaultItem.title || tabPath}
                  {content}
                />
              {:else}
                <div class="editor-main-content">
                  <pre
                    style="padding: 24px; font-family: var(--code-font, monospace); white-space: pre-wrap; overflow-y: auto; height: 100%;">{content}</pre>
                </div>
              {/if}
            </div>
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
      isMarkdownFile={isMarkdownTab}
      markdownViewMode={!isEditing ? 'reading' : markdownViewMode}
      onToggleVim={() => (isVimMode = !isVimMode)}
      onToggleMarkdownView={toggleMarkdownViewMode}
      onChangeMarkdownView={handleChangeMarkdownView}
      onOpenCommandPalette={() => (isPaletteOpen = true)}
      onChangeEncoding={handleChangeEncoding}
    />
  </main>

  <!-- 4. PALETA DE COMANDOS (OVERLAY CTRL+P) -->
  <CommandPalette bind:isOpen={isPaletteOpen}/>

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
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: row;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    overscroll-behavior: none;
    background-color: var(--bg-primary, #ffffff);
  }

  :global(body.is-resizing-col) {
    cursor: col-resize !important;
    user-select: none !important;
  }

  .main-workspace {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .main-workspace.is-resizing {
    pointer-events: none;
    user-select: none;
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
    to {
      transform: rotate(360deg);
    }
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
    height: 100%;
    overflow: hidden;
  }
</style>
