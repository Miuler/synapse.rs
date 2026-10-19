<script lang="ts">
  import {debug, info, warn, error} from '@tauri-apps/plugin-log';
  import {onMount, tick} from "svelte";
  import {Ribbon} from "@widgets/ribbon";
  import {EditorHeader} from "@widgets/editor-header";
  import {StatusBar} from "@widgets/status-bar";
  import {CommandPalette} from "@widgets/command-palette";
  import {QuickOpen} from "@widgets/quick-open";
  import {FullTextSearch} from "@widgets/full-text-search";
  import {EmptyWorkspace} from "@widgets/empty-workspace";
  import {VaultExplorer} from "@features/vault-explorer";
  import {MarkdownViewer} from "@features/markdown-editor";
  import {MermanViewer} from "@features/merman-editor";
  import {MermaidViewer} from "@features/mermaid-editor";
  import {ExcalidrawViewer} from "@features/excalidraw-editor";
  import {ImageViewer} from "@features/image-viewer";
  import {PdfViewer} from "@features/pdf-viewer";
  import {DiffViewer} from "@features/diff-viewer";
  import {appSettings} from "@entities/settings";
  import {
    isImageFile,
    isMarkdownFile,
    isDiagramFile,
    isDrawingFile,
    isPdfFile,
    loadSupportedFileTypes,
  } from "@entities/file-type";
  import type {VaultItem, OpenedNote, MarkdownViewMode} from "@entities/vault-item";
  import {commandRegistry} from "@entities/command";
  import {vaultRepository, searchRepository, toggleDevtools, isTauriEnvironment, type GitFileStatusKind} from "@shared/repositories";
  import {listen, emit} from "@tauri-apps/api/event";
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";
  import {BreadCrumb} from "@widgets/breadcrumb";
  import {Columns2} from "lucide-svelte";

  interface VaultFsChangeEvent {
    paths: string[];
    deleted: string[];
  }

  function isSyntheticTab(p: string | null | undefined): boolean {
    if (!p) return false;
    return p.startsWith("empty:") || p.startsWith("diff:") || p.startsWith("diff-staged:");
  }

  function parseTabDragData(e: DragEvent): { path: string; windowLabel: string; title?: string } | null {
    try {
      const rawCustom = e.dataTransfer?.getData('application/x-synapse-tab');
      if (rawCustom) return JSON.parse(rawCustom);
      const rawText = e.dataTransfer?.getData('text/plain');
      if (rawText && rawText.startsWith('synapse-tab:')) {
        return JSON.parse(rawText.slice('synapse-tab:'.length));
      }
    } catch {}
    return null;
  }

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state("files");
  let vaultExplorerRef = $state<any>(null);
  let isPaletteOpen = $state(false);
  let isQuickOpenOpen = $state(false);
  let isFullTextSearchOpen = $state(false);
  let pendingScrollTerms = $state<Record<string, string[]>>({});
  let isEditing = $state(false);
  let isVimMode = $state(false);
  let markdownViewMode = $state<MarkdownViewMode>("reading");
  let isConnectedToRust = $state(false);
  let syncState = $state<"synced" | "saving" | "error">("synced");
  let isDetachedWindow = $state(false);
  let currentWindowLabel = $state('main');

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
              isLoaded: true,
            };
          }
        } catch (e) {
          console.error("Error al recargar archivo restaurado:", e);
        }
      }
    }
    await refreshGitStatus();
  }

  async function handleReloadItems(targets: string[], reloadedFiles?: string[]) {
    // 1. Recargar metadatos generales de notas de la bóveda
    try {
      const notes = await vaultRepository.getNotes();
      if (notes && Array.isArray(notes)) {
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

        // Sincronizar abs_path en pestañas que se abrieron antes de cargar vaultItems
        for (const item of vaultItems) {
          if (openedNotes[item.relative_path] && !openedNotes[item.relative_path].abs_path && item.abs_path) {
            openedNotes[item.relative_path].abs_path = item.abs_path;
          }
        }
      }
    } catch (e) {
      console.error("Error al refrescar lista de notas:", e);
    }

    // 2. Refrescar archivos abiertos en memoria si coinciden con los objetivos
    const isTargetMatch = (openPath: string): boolean => {
      if (targets.includes('') || targets.includes('.')) return true;
      if (reloadedFiles && reloadedFiles.includes(openPath)) return true;
      for (const t of targets) {
        if (!t) return true;
        const cleanT = t.replace(/^\.\//, '').replace(/\\/g, '/');
        const cleanOpen = openPath.replace(/^\.\//, '').replace(/\\/g, '/');
        if (cleanOpen === cleanT) return true;
        if (cleanOpen.startsWith(`${cleanT}/`)) return true;
      }
      return false;
    };

    for (const openPath of openTabPaths) {
      if (isSyntheticTab(openPath)) continue;
      if (isTargetMatch(openPath)) {
        try {
          const noteData = await vaultRepository.readNote(openPath);
          if (noteData) {
            const currentMode = openedNotes[openPath]?.viewMode || "reading";
            openedNotes[openPath] = {
              ...openedNotes[openPath],
              title: noteData.title || openPath,
              content: noteData.content ?? "",
              savedContent: noteData.content ?? "",
              encoding: noteData.encoding && noteData.encoding.trim() !== "" ? noteData.encoding : "---",
              abs_path: noteData.abs_path || openedNotes[openPath]?.abs_path,
              isLoading: false,
              isLoaded: true,
              viewMode: currentMode,
              lastReloaded: Date.now(),
            };
          }
        } catch (e) {
          console.error(`Error al recargar archivo en memoria ${openPath}:`, e);
        }
      }
    }

    // 3. Refrescar estado de Git
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

  function getInitialSplitRatio(): number {
    try {
      const saved = localStorage.getItem('synapse_split_ratio');
      if (saved) {
        const parsed = parseFloat(saved);
        if (!isNaN(parsed) && parsed >= 20 && parsed <= 80) return parsed;
      }
    } catch {}
    return 50;
  }

  // Lista de metadatos de elementos de la bóveda (VaultItem[])
  let vaultItems = $state<VaultItem[]>([]);
  let openTabPaths = $state<string[]>([]);
  let activeTabPath = $state<string | null>(null);

  // Estado para panel dividido (Split View vertical)
  let splitTabPaths = $state<string[]>([]);
  let activeSplitTabPath = $state<string | null>(null);
  let splitRatio = $state<number>(getInitialSplitRatio());
  let isResizingSplit = $state(false);
  let isDragOverSplitRight = $state(false);
  let activeFocusedPane = $state<'left' | 'right'>('left');
  let splitMarkdownViewMode = $state<MarkdownViewMode>('live');
  let isSplitEditing = $state(true);
  let isSplit = $derived(splitTabPaths.length > 0 && activeSplitTabPath !== null);
  let splitContainerRef = $state<HTMLDivElement | null>(null);
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
    if (!isConnectedToRust || isDetachedWindow) return;
    if (saveTabsTimeout) clearTimeout(saveTabsTimeout);
    saveTabsTimeout = setTimeout(() => {
      const tabs = openTabPaths
        .filter((p) => p && !isSyntheticTab(p))
        .map((p) => ({
          path: p,
          view_mode: openedNotes[p]?.viewMode || "reading",
        }));
      const active = activeTabPath && !isSyntheticTab(activeTabPath) ? activeTabPath : null;
      vaultRepository.saveOpenTabsState(tabs, active);
    }, 250);
  }

  async function restoreOpenTabsState(): Promise<boolean> {
    if (isDetachedWindow) return false;
    try {
      const state = await vaultRepository.getOpenTabsState();
      if (state && state.open_tabs && state.open_tabs.length > 0) {
        openTabPaths = [];
        for (const tab of state.open_tabs) {
          if (!tab.path) continue;
          openTabPaths.push(tab.path);
          const defaultMode: MarkdownViewMode = isDrawingFile(tab.path) ? "live" : "reading";
          const mode = (tab.view_mode as MarkdownViewMode) || defaultMode;
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
              isLoaded: false,
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
          const defaultActiveMode: MarkdownViewMode = isDrawingFile(targetActive) ? "live" : "reading";
          const activeMode = openedNotes[targetActive]?.viewMode || defaultActiveMode;
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
    if (activeTabPath && !isSyntheticTab(activeTabPath)) {
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

  function toggleViewMode() {
    if (!activeTabPath || isSyntheticTab(activeTabPath)) return;

    if (isMarkdownFile(activeTabPath)) {
      if (!isEditing) {
        handleChangeMarkdownView(markdownViewMode === "reading" ? "live" : markdownViewMode);
      } else {
        handleChangeMarkdownView("reading");
      }
    } else {
      const nextEditing = !isEditing;
      isEditing = nextEditing;
      const nextMode: MarkdownViewMode = nextEditing ? "live" : "reading";
      markdownViewMode = nextMode;
      if (openedNotes[activeTabPath]) {
        openedNotes[activeTabPath].viewMode = nextMode;
      }
      persistTabsState();
    }
  }

  async function ensureContentLoaded(path: string) {
    if (!path || isSyntheticTab(path)) return;
    if (openedNotes[path] && openedNotes[path].isLoaded && !openedNotes[path].isLoading) return;

    const vaultItem = vaultItems.find((v) => v.relative_path === path);
    let initialAbsPath = vaultItem?.abs_path;

    // Para imágenes y PDFs no se requiere leer contenido como texto; resolveAssetUrl / readAssetDataUrl se encarga
    if ((isImageFile(path) && !isDrawingFile(path)) || isPdfFile(path)) {
      if (!initialAbsPath) {
        try {
          initialAbsPath = (await vaultRepository.resolveAssetFilePath(path)) ?? undefined;
        } catch (e) {
          console.warn(`Error resolviendo ruta absoluta para archivo binario ${path}:`, e);
        }
      }

      const currentMode = openedNotes[path]?.viewMode || "reading";
      openedNotes[path] = {
        relative_path: path,
        abs_path: initialAbsPath,
        title: vaultItem?.title || path,
        content: "",
        savedContent: "",
        encoding: "binary",
        isLoading: false,
        isLoaded: true,
        viewMode: currentMode,
      };
      return;
    }

    if (!openedNotes[path]) {
      const defaultMode: MarkdownViewMode = isDrawingFile(path) ? "live" : "reading";
      openedNotes[path] = {
        relative_path: path,
        abs_path: initialAbsPath,
        title: vaultItem?.title || path,
        content: "",
        savedContent: "",
        encoding: "---",
        isLoading: true,
        isLoaded: false,
        viewMode: defaultMode,
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
        const currentMode = openedNotes[path]?.viewMode || (isDrawingFile(path) ? "live" : "reading");

        openedNotes[path] = {
          relative_path: path,
          abs_path: fetchedAbsPath,
          title: noteData.title || vaultItem?.title || path,
          content: fetchedContent,
          savedContent: fetchedContent,
          encoding: fetchedEncoding,
          isLoading: false,
          isLoaded: true,
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
      isLoaded: true,
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
    if (!isSyntheticTab(path)) {
      const defaultMode: MarkdownViewMode = isDrawingFile(path) ? "live" : "reading";
      const mode = openedNotes[path]?.viewMode || defaultMode;
      markdownViewMode = mode;
      isEditing = mode !== "reading";
      ensureContentLoaded(path);
      recentFiles = [path, ...recentFiles.filter((p) => p !== path)].slice(0, 15);
      vaultRepository.recordNoteOpened(path);
    }
    recordTabVisit(path);
    persistTabsState();
  }

  function handleNavigateLink(targetPath: string) {
    if (!targetPath) return;
    const [filePath, anchor] = targetPath.split('#');
    const cleanPath = filePath.trim();
    if (cleanPath) {
      selectTab(cleanPath);
      if (anchor) {
        pendingScrollTerms[cleanPath] = [anchor.trim()];
      }
    } else if (anchor && activeTabPath) {
      pendingScrollTerms[activeTabPath] = [anchor.trim()];
    }
  }

  function closeTab(path: string) {
    const idx = openTabPaths.indexOf(path);
    if (idx !== -1) {
      openTabPaths.splice(idx, 1);
      // Guardar la posición previa en la barra de pestañas para reabrirla exactamente donde estaba
      if (!isSyntheticTab(path)) {
        lastClosedTabIndex[path] = idx;
      }

      if (activeTabPath === path) {
        if (openTabPaths.length > 0) {
          const nextIdx = Math.min(idx, openTabPaths.length - 1);
          activeTabPath = openTabPaths[nextIdx];
          if (activeTabPath && !isSyntheticTab(activeTabPath)) {
            const defaultNextMode: MarkdownViewMode = isDrawingFile(activeTabPath) ? "live" : "reading";
            const nextMode = openedNotes[activeTabPath]?.viewMode || defaultNextMode;
            markdownViewMode = nextMode;
            isEditing = nextMode !== "reading";
            ensureContentLoaded(activeTabPath);
          }
        } else {
          activeTabPath = null;
        }

        // Si la pestaña cerrada era la activa, sincronizar tabHistoryIndex con la nueva pestaña activa
        if (activeTabPath && !isSyntheticTab(path)) {
          const foundIdx = tabHistory.slice(0, tabHistoryIndex).lastIndexOf(activeTabPath);
          if (foundIdx !== -1) {
            tabHistoryIndex = foundIdx;
          }
        }
      }
    }
    // Liberar memoria consolidada del contenido de la nota para mantener bajo consumo de RAM
    delete openedNotes[path];
    delete excalidrawViewers[path];
    delete pdfViewers[path];
    delete recentSaveTimestamps[path];
    // Se conserva tabSelections[path] para preservar la posición del cursor si se reabre

    // Solo las pestañas temporales sintéticas se retiran del historial
    if (isSyntheticTab(path)) {
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
    excalidrawViewers = {};
    pdfViewers = {};
    recentSaveTimestamps = {};
    tabSelections = {};
    tabHistory = [];
    tabHistoryIndex = -1;
    handleNewEmptyTab();
    persistTabsState();
  }

  function closeOtherTabs(targetPath: string) {
    const tabsToClose = openTabPaths.filter((p) => p !== targetPath);
    for (const p of tabsToClose) {
      closeTab(p);
    }
  }

  function closeTabsToRight(targetPath: string) {
    const idx = openTabPaths.indexOf(targetPath);
    if (idx !== -1) {
      const tabsToClose = openTabPaths.slice(idx + 1);
      for (const p of tabsToClose) {
        closeTab(p);
      }
    }
  }

  async function handleOpenInNewWindow(path: string, moveTab = false) {
    if (!path || isSyntheticTab(path)) return;
    const vItem = vaultItems.find((v) => v.relative_path === path);
    const title = vItem?.title || path.split('/').pop() || path;
    await vaultRepository.openTabInNewWindow(path, title);
    if (moveTab) {
      closeTab(path);
    }
  }

  function reorderTab(path: string, targetIndex: number) {
    const currentIndex = openTabPaths.indexOf(path);
    if (currentIndex === -1) return;
    const newPaths = [...openTabPaths];
    newPaths.splice(currentIndex, 1);
    const adjustedIndex = targetIndex > currentIndex ? targetIndex - 1 : targetIndex;
    const finalIndex = Math.max(0, Math.min(adjustedIndex, newPaths.length));
    newPaths.splice(finalIndex, 0, path);
    openTabPaths = newPaths;
    persistTabsState();
  }

  function receiveTabFromOtherWindow(path: string, targetIndex?: number, sourceWindow?: string) {
    if (!openTabPaths.includes(path)) {
      const newPaths = [...openTabPaths];
      const insertAt = targetIndex !== undefined ? Math.min(targetIndex, newPaths.length) : newPaths.length;
      newPaths.splice(insertAt, 0, path);
      openTabPaths = newPaths;
    }
    selectTab(path);
    if (isTauriEnvironment() && sourceWindow) {
      emit("synapse:tab-moved-away", {
        path,
        sourceWindow,
        targetWindow: currentWindowLabel,
      });
    }
  }

  function handleNewSplitEmptyTab() {
    emptyTabCounter += 1;
    const emptyTabPath = `empty://${emptyTabCounter}-${Date.now()}`;
    splitTabPaths = [...splitTabPaths, emptyTabPath];
    activeSplitTabPath = emptyTabPath;
    openedNotes[emptyTabPath] = {
      relative_path: emptyTabPath,
      abs_path: undefined,
      title: "Nueva pestaña",
      content: "",
      savedContent: "",
      encoding: "UTF-8",
      isLoading: false,
      isLoaded: true,
      viewMode: "reading",
      lastReloaded: Date.now(),
    };
    activeFocusedPane = 'right';
  }

  function selectSplitTab(path: string) {
    if (!path) return;
    if (!splitTabPaths.includes(path)) {
      splitTabPaths = [...splitTabPaths, path];
    }
    activeSplitTabPath = path;
    if (!isSyntheticTab(path)) {
      ensureContentLoaded(path);
      const defaultMode: MarkdownViewMode = isDrawingFile(path) ? "live" : "reading";
      const mode = openedNotes[path]?.viewMode || defaultMode;
      splitMarkdownViewMode = mode;
      isSplitEditing = mode !== "reading";
      recentFiles = [path, ...recentFiles.filter((p) => p !== path)].slice(0, 15);
      vaultRepository.recordNoteOpened(path);
    }
    activeFocusedPane = 'right';
  }

  function closeSplitTab(path: string) {
    const idx = splitTabPaths.indexOf(path);
    if (idx !== -1) {
      splitTabPaths = splitTabPaths.filter((p) => p !== path);
      if (activeSplitTabPath === path) {
        if (splitTabPaths.length > 0) {
          const nextIdx = Math.min(idx, splitTabPaths.length - 1);
          activeSplitTabPath = splitTabPaths[nextIdx];
          if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)) {
            ensureContentLoaded(activeSplitTabPath);
          }
        } else {
          activeSplitTabPath = null;
          activeFocusedPane = 'left';
        }
      }
    }
    if (splitTabPaths.length === 0) {
      activeSplitTabPath = null;
      activeFocusedPane = 'left';
    }
  }

  function closeAllSplitTabs() {
    splitTabPaths = [];
    activeSplitTabPath = null;
    activeFocusedPane = 'left';
  }

  function closeOtherSplitTabs(targetPath: string) {
    const toClose = splitTabPaths.filter((p) => p !== targetPath);
    for (const p of toClose) {
      closeSplitTab(p);
    }
  }

  function closeSplitTabsToRight(targetPath: string) {
    const idx = splitTabPaths.indexOf(targetPath);
    if (idx !== -1) {
      const toClose = splitTabPaths.slice(idx + 1);
      for (const p of toClose) {
        closeSplitTab(p);
      }
    }
  }

  function reorderSplitTab(path: string, targetIndex: number) {
    const currentIndex = splitTabPaths.indexOf(path);
    if (currentIndex === -1) return;
    const newPaths = [...splitTabPaths];
    newPaths.splice(currentIndex, 1);
    const adjustedIndex = targetIndex > currentIndex ? targetIndex - 1 : targetIndex;
    const finalIndex = Math.max(0, Math.min(adjustedIndex, newPaths.length));
    newPaths.splice(finalIndex, 0, path);
    splitTabPaths = newPaths;
  }

  function splitTabToRight(path: string, sourceWindow?: string) {
    if (!path) return;
    if (sourceWindow && sourceWindow !== currentWindowLabel) {
      if (isTauriEnvironment()) {
        emit("synapse:tab-moved-away", {
          path,
          sourceWindow,
          targetWindow: currentWindowLabel,
        });
      }
    }

    if (openTabPaths.includes(path)) {
      openTabPaths = openTabPaths.filter((p) => p !== path);
      if (openTabPaths.length === 0) {
        handleNewEmptyTab();
      } else if (activeTabPath === path) {
        activeTabPath = openTabPaths[0];
      }
    }

    if (!splitTabPaths.includes(path)) {
      splitTabPaths = [...splitTabPaths, path];
    }
    activeSplitTabPath = path;

    if (!isSyntheticTab(path)) {
      ensureContentLoaded(path);
      const defaultMode: MarkdownViewMode = isDrawingFile(path) ? "live" : "reading";
      const mode = openedNotes[path]?.viewMode || defaultMode;
      splitMarkdownViewMode = mode;
      isSplitEditing = mode !== "reading";
    }

    activeFocusedPane = 'right';
  }

  function handleSplitView(targetPath?: string) {
    if (isSplit) {
      // Si ya está dividido, fusionar pestañas del panel derecho al izquierdo y cerrar división
      for (const p of splitTabPaths) {
        if (!openTabPaths.includes(p)) {
          openTabPaths = [...openTabPaths, p];
        }
      }
      splitTabPaths = [];
      activeSplitTabPath = null;
      activeFocusedPane = 'left';
      return;
    }

    const pathToSplit = targetPath || activeTabPath;
    if (!pathToSplit) return;

    if (openTabPaths.length > 1) {
      splitTabToRight(pathToSplit);
    } else {
      splitTabPaths = [pathToSplit];
      activeSplitTabPath = pathToSplit;
      activeFocusedPane = 'right';
      if (!isSyntheticTab(pathToSplit)) {
        ensureContentLoaded(pathToSplit);
      }
    }
  }

  function receiveTabFromSplitOrWindow(path: string, isTargetRight: boolean, targetIndex?: number, sourceWindow?: string) {
    if (isTargetRight) {
      if (openTabPaths.includes(path)) {
        openTabPaths = openTabPaths.filter((p) => p !== path);
        if (openTabPaths.length === 0) {
          handleNewEmptyTab();
        } else if (activeTabPath === path) {
          activeTabPath = openTabPaths[0];
        }
      }
      if (!splitTabPaths.includes(path)) {
        const newPaths = [...splitTabPaths];
        const insertAt = targetIndex !== undefined ? Math.min(targetIndex, newPaths.length) : newPaths.length;
        newPaths.splice(insertAt, 0, path);
        splitTabPaths = newPaths;
      }
      selectSplitTab(path);
    } else {
      if (splitTabPaths.includes(path)) {
        splitTabPaths = splitTabPaths.filter((p) => p !== path);
        if (splitTabPaths.length === 0) {
          activeSplitTabPath = null;
        } else if (activeSplitTabPath === path) {
          activeSplitTabPath = splitTabPaths[0];
        }
      }
      if (!openTabPaths.includes(path)) {
        const newPaths = [...openTabPaths];
        const insertAt = targetIndex !== undefined ? Math.min(targetIndex, newPaths.length) : newPaths.length;
        newPaths.splice(insertAt, 0, path);
        openTabPaths = newPaths;
      }
      selectTab(path);
    }

    if (isTauriEnvironment() && sourceWindow && sourceWindow !== currentWindowLabel) {
      emit("synapse:tab-moved-away", {
        path,
        sourceWindow,
        targetWindow: currentWindowLabel,
      });
    }
  }

  function handleSelectTabFromExplorer(path: string) {
    if (activeFocusedPane === 'right' && isSplit) {
      selectSplitTab(path);
    } else {
      selectTab(path);
    }
  }

  function handleChangeSplitMarkdownView(newMode: MarkdownViewMode) {
    splitMarkdownViewMode = newMode;
    isSplitEditing = newMode !== "reading";
    if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)) {
      if (openedNotes[activeSplitTabPath]) {
        openedNotes[activeSplitTabPath].viewMode = newMode;
      }
    }
  }

  function toggleSplitViewMode() {
    const current = !isSplitEditing ? "reading" : splitMarkdownViewMode;
    let nextMode: MarkdownViewMode;
    let nextEditing: boolean;
    if (current === "reading") {
      nextMode = "live";
      nextEditing = true;
    } else if (current === "live") {
      nextMode = "source";
      nextEditing = true;
    } else {
      nextMode = "reading";
      nextEditing = false;
    }
    splitMarkdownViewMode = nextMode;
    isSplitEditing = nextEditing;
    if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)) {
      if (openedNotes[activeSplitTabPath]) {
        openedNotes[activeSplitTabPath].viewMode = nextMode;
      }
    }
  }

  function handleSplitResizeStart(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    isResizingSplit = true;
    document.body.classList.add('is-resizing-col');

    const handlePointerMove = (ev: MouseEvent | PointerEvent) => {
      if (!splitContainerRef) return;
      const rect = splitContainerRef.getBoundingClientRect();
      const relativeX = ev.clientX - rect.left;
      const newRatio = (relativeX / rect.width) * 100;
      splitRatio = Math.max(20, Math.min(80, newRatio));
    };

    const handlePointerUp = () => {
      isResizingSplit = false;
      document.body.classList.remove('is-resizing-col');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
      window.removeEventListener('mousemove', handlePointerMove);
      window.removeEventListener('mouseup', handlePointerUp);
      try {
        localStorage.setItem('synapse_split_ratio', String(splitRatio));
      } catch {}
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
    window.addEventListener('mousemove', handlePointerMove);
    window.addEventListener('mouseup', handlePointerUp);
  }

  function handleEditorDragOver(e: DragEvent) {
    const isSynapseTab =
      e.dataTransfer?.types.includes('application/x-synapse-tab') ||
      e.dataTransfer?.types.includes('text/plain');
    if (!isSynapseTab) return;

    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }

    if (!editorContainerRef) return;
    const rect = editorContainerRef.getBoundingClientRect();
    const isRightZone = e.clientX > (rect.left + rect.width * 0.5);
    isDragOverSplitRight = isRightZone;
  }

  function handleEditorDragLeave(e: DragEvent) {
    if (e.currentTarget === e.target || !editorContainerRef?.contains(e.relatedTarget as Node)) {
      isDragOverSplitRight = false;
    }
  }

  function handleEditorDrop(e: DragEvent) {
    const wasOverSplit = isDragOverSplitRight;
    isDragOverSplitRight = false;

    const data = parseTabDragData(e);
    if (!data?.path) return;

    e.preventDefault();
    e.stopPropagation();

    if (wasOverSplit && !isSplit) {
      splitTabToRight(data.path, data.windowLabel);
    } else if (data.windowLabel !== currentWindowLabel) {
      receiveTabFromOtherWindow(data.path, openTabPaths.length, data.windowLabel);
    }
  }

  function handlePaneDragOver(e: DragEvent, _pane: 'left' | 'right') {
    const isSynapseTab =
      e.dataTransfer?.types.includes('application/x-synapse-tab') ||
      e.dataTransfer?.types.includes('text/plain');
    if (!isSynapseTab) return;

    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }
  }

  function handlePaneDrop(e: DragEvent, targetPane: 'left' | 'right') {
    const data = parseTabDragData(e);
    if (!data?.path) return;

    e.preventDefault();
    e.stopPropagation();

    receiveTabFromSplitOrWindow(data.path, targetPane === 'right', undefined, data.windowLabel);
  }

  let currentFocusedTabPath = $derived(
    activeFocusedPane === 'right' && isSplit && activeSplitTabPath
      ? activeSplitTabPath
      : activeTabPath
  );

  let activeNote = $derived(
    activeTabPath ? openedNotes[activeTabPath] : undefined
  );

  let focusedNote = $derived(
    currentFocusedTabPath ? openedNotes[currentFocusedTabPath] : undefined
  );

  let excalidrawViewers = $state<Record<string, any>>({});
  let pdfViewers = $state<Record<string, any>>({});
  let recentSaveTimestamps: Record<string, number> = {};

  let currentVaultItem = $derived(
    activeTabPath && !isSyntheticTab(activeTabPath)
      ? vaultItems.find((vaultItem) => vaultItem.relative_path === activeTabPath) || {
          id: "0",
          title: activeNote?.title || activeTabPath.split("/").pop() || activeTabPath,
          relative_path: activeTabPath,
          abs_path: activeNote?.abs_path,
        }
      : { id: "0", title: "", relative_path: "", abs_path: undefined }
  );

  let currentSplitVaultItem = $derived(
    activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)
      ? vaultItems.find((vaultItem) => vaultItem.relative_path === activeSplitTabPath) || {
          id: "0",
          title: openedNotes[activeSplitTabPath]?.title || activeSplitTabPath.split("/").pop() || activeSplitTabPath,
          relative_path: activeSplitTabPath,
          abs_path: openedNotes[activeSplitTabPath]?.abs_path,
        }
      : { id: "0", title: "", relative_path: "", abs_path: undefined }
  );

  let currentFocusedVaultItem = $derived(
    activeFocusedPane === 'right' && isSplit ? currentSplitVaultItem : currentVaultItem
  );

  let activeContent = $derived(
    activeNote ? activeNote.content : ""
  );

  let focusedContent = $derived(
    focusedNote ? focusedNote.content : ""
  );

  let currentEncoding = $derived(
    focusedNote ? focusedNote.encoding : "---"
  );

  let isMarkdownTab = $derived(
    Boolean(
      currentFocusedTabPath &&
      !isSyntheticTab(currentFocusedTabPath) &&
      (
        isMarkdownFile(currentFocusedTabPath) ||
        (currentFocusedVaultItem.relative_path && isMarkdownFile(currentFocusedVaultItem.relative_path))
      )
    )
  );

  let isMarkdownSplitTab = $derived(
    Boolean(
      activeSplitTabPath &&
      !isSyntheticTab(activeSplitTabPath) &&
      (
        isMarkdownFile(activeSplitTabPath) ||
        (currentSplitVaultItem.relative_path && isMarkdownFile(currentSplitVaultItem.relative_path))
      )
    )
  );

  let hasActiveContent = $derived(
    Boolean(
      activeTabPath &&
      !isSyntheticTab(activeTabPath) &&
      openTabPaths.includes(activeTabPath) &&
      !isImageFile(activeTabPath) &&
      !isPdfFile(activeTabPath) &&
      activeNote &&
      !activeNote.isLoading &&
      typeof activeNote.content === "string" &&
      activeNote.content.trim().length > 0
    )
  );

  let hasActiveSplitContent = $derived(
    Boolean(
      activeSplitTabPath &&
      !isSyntheticTab(activeSplitTabPath) &&
      splitTabPaths.includes(activeSplitTabPath) &&
      !isImageFile(activeSplitTabPath) &&
      !isPdfFile(activeSplitTabPath) &&
      openedNotes[activeSplitTabPath] &&
      !openedNotes[activeSplitTabPath]?.isLoading &&
      typeof openedNotes[activeSplitTabPath]?.content === "string" &&
      openedNotes[activeSplitTabPath]?.content.trim().length > 0
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
      if (path.startsWith("diff-staged:")) {
        const filePath = path.slice("diff-staged:".length);
        const fileName = filePath.split("/").pop() || filePath;
        return {
          path,
          title: `Diff (staged): ${fileName}`,
          abs_path: undefined,
          isDirty: false,
        };
      }
      if (path.startsWith("diff:")) {
        const filePath = path.slice("diff:".length);
        const fileName = filePath.split("/").pop() || filePath;
        return {
          path,
          title: `Diff: ${fileName}`,
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

  let rightTabsInfo = $derived(
    splitTabPaths.map((path) => {
      if (path.startsWith("empty:")) {
        return {
          path,
          title: "Nueva pestaña",
          abs_path: undefined,
          isDirty: false,
        };
      }
      if (path.startsWith("diff-staged:")) {
        const filePath = path.slice("diff-staged:".length);
        const fileName = filePath.split("/").pop() || filePath;
        return {
          path,
          title: `Diff (staged): ${fileName}`,
          abs_path: undefined,
          isDirty: false,
        };
      }
      if (path.startsWith("diff:")) {
        const filePath = path.slice("diff:".length);
        const fileName = filePath.split("/").pop() || filePath;
        return {
          path,
          title: `Diff: ${fileName}`,
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
    (currentFocusedTabPath && tabSelections[currentFocusedTabPath])
      ? tabSelections[currentFocusedTabPath]
      : defaultSelection
  );

  // Contadores calculados reactivamente sobre todo el documento
  let docWordCount = $derived(
    focusedContent && focusedContent.trim()
      ? focusedContent.trim().split(/\s+/).length
      : 0
  );
  let docCharCount = $derived(focusedContent ? focusedContent.length : 0);

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

  // Asegurar que activeTabPath sea siempre una pestaña válida si hay pestañas abiertas
  $effect(() => {
    if (openTabPaths.length > 0 && (!activeTabPath || !openTabPaths.includes(activeTabPath))) {
      selectTab(openTabPaths[0]);
    }
  });

  $effect(() => {
    if (splitTabPaths.length > 0 && (!activeSplitTabPath || !splitTabPaths.includes(activeSplitTabPath))) {
      selectSplitTab(splitTabPaths[0]);
    }
  });

  // Volver al inicio del documento y asegurar carga al cambiar de pestaña
  $effect(() => {
    const path = activeTabPath;
    if (path && !isSyntheticTab(path)) {
      ensureContentLoaded(path);
    }
    tick().then(() => {
      if (editorContainerRef) editorContainerRef.scrollTop = 0;
    });
  });

  async function fetchNotesFromBackend() {
    try {
      const activePath = await vaultRepository.getActiveVaultPath();
      if (!activePath) {
        vaultItems = [];
        isConnectedToRust = vaultRepository.isConnected();
        if (openTabPaths.length === 0) {
          handleNewEmptyTab();
        }
        return;
      }

      appSettings.setLastOpenedFolder(activePath);
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

  async function handleRenameItem(relativePath: string, newName: string): Promise<string> {
    const renamedPath = await vaultRepository.renameItem(relativePath, newName);

    // Conservar abierta la pestaña y su contenido bajo la nueva ruta.
    if (openedNotes[relativePath]) {
      openedNotes[renamedPath] = openedNotes[relativePath];
      delete openedNotes[relativePath];
    }
    openTabPaths = openTabPaths.map((path) => path === relativePath ? renamedPath : path);
    if (activeTabPath === relativePath) activeTabPath = renamedPath;
    if (tabSelections[relativePath]) {
      tabSelections[renamedPath] = tabSelections[relativePath];
      delete tabSelections[relativePath];
    }
    tabHistory = tabHistory.map((path) => path === relativePath ? renamedPath : path);
    recentFiles = recentFiles.map((path) => path === relativePath ? renamedPath : path);
    if (lastClosedTabIndex[relativePath] !== undefined) {
      lastClosedTabIndex[renamedPath] = lastClosedTabIndex[relativePath];
      delete lastClosedTabIndex[relativePath];
    }

    await fetchNotesFromBackend();
    persistTabsState();
    return renamedPath;
  }

  // Carga únicamente de metadatos desde la bóveda (repositorio) al montar
  onMount(() => {
    window.scrollTo(0, 0);

    // Cargar la configuración de tipos soportados a través de la entidad file-type
    loadSupportedFileTypes();

    vaultRepository.getDetachedWindowTab().then(async (detachedTab) => {
      if (detachedTab) {
        isDetachedWindow = true;
        openTabPaths = [detachedTab];
        activeTabPath = detachedTab;
        await fetchNotesFromBackend();
        ensureContentLoaded(detachedTab);
        return;
      }

      // Si al arrancar no hay ningún tab seleccionado/abierto, abrir uno vacío
      if (openTabPaths.length === 0) {
        handleNewEmptyTab();
      }

      const activePath = await vaultRepository.getActiveVaultPath();
      if (!activePath) {
        await handleOpenVaultFolder();
      } else {
        await fetchNotesFromBackend();
      }
    });

    let unlistenFsChange: (() => void) | undefined;
    let unlistenTabMoved: (() => void) | undefined;
    if (isTauriEnvironment()) {
      try {
        currentWindowLabel = getCurrentWebviewWindow().label;
      } catch {}

      listen<{ path: string; sourceWindow: string; targetWindow: string }>(
        "synapse:tab-moved-away",
        (event) => {
          if (event.payload.sourceWindow === currentWindowLabel) {
            closeTab(event.payload.path);
          }
        }
      ).then((unsub) => {
        unlistenTabMoved = unsub;
      });

      listen<VaultFsChangeEvent>("vault:files-changed", async (event) => {
        const { paths, deleted } = event.payload;

        // 1. Recargar pestañas abiertas que hayan sido modificadas externamente si están limpias
        for (const changedPath of paths) {
          const cleanChanged = changedPath.replace(/^\.\//, '').replace(/\\/g, '/');
          const matchedOpenPath = openTabPaths.find((p) => p.replace(/^\.\//, '').replace(/\\/g, '/') === cleanChanged);

          if (matchedOpenPath) {
            // Ignorar eventos generados por nuestro propio guardado reciente (evita bucles reactivos y re-renders)
            const lastSave = recentSaveTimestamps[matchedOpenPath] || recentSaveTimestamps[changedPath] || 0;
            if (Date.now() - lastSave < 2500) {
              continue;
            }

            // Si es un dibujo abierto y el usuario está editando activamente, no sobreescribir con datos de disco
            if (isDrawingFile(matchedOpenPath)) {
              const isCurrentlyEditing = (matchedOpenPath === activeTabPath && isEditing) || (matchedOpenPath === activeSplitTabPath && isSplitEditing);
              if (isCurrentlyEditing) {
                continue;
              }
            }

            const note = openedNotes[matchedOpenPath];
            const isClean = !note || note.content === note.savedContent;
            if (isClean) {
              try {
                const noteData = await vaultRepository.readNote(matchedOpenPath);
                if (noteData && (noteData.content ?? "") !== (openedNotes[matchedOpenPath]?.savedContent ?? null)) {
                  const currentMode = openedNotes[matchedOpenPath]?.viewMode || (isDrawingFile(matchedOpenPath) ? "live" : "reading");
                  openedNotes[matchedOpenPath] = {
                    relative_path: matchedOpenPath,
                    abs_path: noteData.abs_path,
                    title: noteData.title || matchedOpenPath,
                    content: noteData.content ?? "",
                    savedContent: noteData.content ?? "",
                    encoding: noteData.encoding || "---",
                    isLoading: false,
                    isLoaded: true,
                    viewMode: currentMode,
                  };
                }
              } catch (e) {
                console.warn(`Error al recargar archivo modificado externamente: ${matchedOpenPath}`, e);
              }
            }
          }
        }

        // 2. Si se eliminaron archivos que están abiertos
        for (const delPath of deleted) {
          const affectedTabs = openTabPaths.filter((p) => p === delPath || p.startsWith(`${delPath}/`));
          for (const tabPath of affectedTabs) {
            const note = openedNotes[tabPath];
            if (!note || note.content === note.savedContent) {
              closeTab(tabPath);
            }
          }
        }

        // 3. Refrescar notas, estado de Git y árbol del explorador
        await fetchNotesFromBackend();
        await refreshGitStatus();
        if (vaultExplorerRef?.refreshTree) {
          await vaultExplorerRef.refreshTree();
        }
      }).then((unsub) => {
        unlistenFsChange = unsub;
      }).catch((e) => {
        console.warn('Error escuchando cambios de sistema de archivos:', e);
      });
    }

    return () => {
      if (unlistenFsChange) {
        unlistenFsChange();
      }
      if (unlistenTabMoved) {
        unlistenTabMoved();
      }
    };
  });

  // Estado de Auto-Guardado
  let autoSave = $state(true);
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  function debouncedPersistVaultItemToRust(vaultItem: VaultItem, delay?: number) {
    if (!autoSave) return;
    const computedDelay = delay !== undefined ? delay : (isDrawingFile(vaultItem.relative_path) ? 1200 : 600);
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      persistVaultItemToRust(vaultItem);
    }, computedDelay);
  }

  async function persistVaultItemToRust(vaultItem: VaultItem, targetEncoding?: string) {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }
    const path = vaultItem.relative_path;
    const title = vaultItem.title || path?.split('/').pop() || path;
    if ((!isConnectedToRust && !vaultRepository.isConnected()) || !path) return;

    // Proteger archivos binarios o imágenes de sobreescrituras accidentales (excepto dibujos)
    if (isImageFile(path) && !isDrawingFile(path)) return;

    // Si es un archivo PDF, delegar persistencia al visor PDF (con anotaciones incrustadas)
    if (isPdfFile(path)) {
      if (pdfViewers[path]?.saveDocument) {
        try {
          syncState = "saving";
          recentSaveTimestamps[path] = Date.now();
          await pdfViewers[path].saveDocument();
          syncState = "synced";
        } catch (err) {
          console.error("Error al persistir PDF con anotaciones:", err);
          syncState = "error";
        }
      }
      return;
    }

    // Si es un dibujo, sincronizar inmediatamente la escena más fresca del visor Excalidraw
    if (isDrawingFile(path) && excalidrawViewers[path]?.getCurrentSerializedScene) {
      try {
        const freshContent = excalidrawViewers[path].getCurrentSerializedScene();
        if (freshContent && openedNotes[path]) {
          openedNotes[path].content = freshContent;
        }
      } catch (err) {
        console.warn("Error al extraer escena actual de Excalidraw:", err);
      }
    }

    const note = openedNotes[path];
    const contentToSave = note ? note.content : "";
    const currentTabEnc = note ? note.encoding : "---";
    const enc = targetEncoding || (currentTabEnc && currentTabEnc !== "---" ? currentTabEnc : "UTF-8");

    // Registrar timestamp antes de iniciar el guardado para evitar condiciones de carrera con notify / fs-events
    recentSaveTimestamps[path] = Date.now();
    syncState = "saving";
    try {
      await vaultRepository.saveNote({
        relativePath: path,
        title: title,
        content: contentToSave,
        encoding: enc,
      });
      recentSaveTimestamps[path] = Date.now();
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
    if (!activeTabPath || isSyntheticTab(activeTabPath)) {
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
      isLoaded: true,
    };

    const targetEmptyPath = emptyTabPathToReplace || (
      activeFocusedPane === 'right' && isSplit && activeSplitTabPath && activeSplitTabPath.startsWith("empty:")
        ? activeSplitTabPath
        : (activeTabPath && activeTabPath.startsWith("empty:") ? activeTabPath : null)
    );
    if (targetEmptyPath && splitTabPaths.includes(targetEmptyPath)) {
      const idx = splitTabPaths.indexOf(targetEmptyPath);
      splitTabPaths[idx] = newRelPath;
      delete openedNotes[targetEmptyPath];
      delete tabSelections[targetEmptyPath];
      activeSplitTabPath = newRelPath;
      activeFocusedPane = 'right';
    } else if (targetEmptyPath && openTabPaths.includes(targetEmptyPath)) {
      const idx = openTabPaths.indexOf(targetEmptyPath);
      openTabPaths[idx] = newRelPath;
      delete openedNotes[targetEmptyPath];
      delete tabSelections[targetEmptyPath];
      tabHistory = tabHistory.map((p) => (p === targetEmptyPath ? newRelPath : p));
      activeTabPath = newRelPath;
      activeFocusedPane = 'left';
    } else if (activeFocusedPane === 'right' && isSplit) {
      splitTabPaths.push(newRelPath);
      activeSplitTabPath = newRelPath;
    } else {
      openTabPaths.push(newRelPath);
      activeTabPath = newRelPath;
    }
    recentFiles = [newRelPath, ...recentFiles.filter((p) => p !== newRelPath)].slice(0, 15);
    recordTabVisit(newRelPath);
    await persistVaultItemToRust(newVaultItem);
    refreshGitStatus();
  }

  // Registrar comandos por defecto al iniciar
  onMount(() => {
    commandRegistry.registerMany([
      {
        id: "cmd-split-view",
        name: "Dividir panel verticalmente (Split View)",
        category: "Vista",
        shortcut: "Ctrl+\\",
        action: () => handleSplitView(),
      },
      {
        id: "cmd-new-tab",
        name: "Nueva pestaña vacía",
        category: "Pestañas",
        shortcut: "Ctrl+T",
        action: handleNewEmptyTab,
      },
      {
        id: "cmd-open-in-new-window",
        name: "Abrir pestaña activa en nueva ventana",
        category: "Pestañas",
        action: () => {
          if (activeTabPath && !isSyntheticTab(activeTabPath)) {
            handleOpenInNewWindow(activeTabPath, false);
          }
        },
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
        action: () => {
          info(`activeTabPath: ${activeTabPath}`);
          createNewVaultItem();
        },
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
          if (!activeTabPath || isSyntheticTab(activeTabPath)) return;
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
          if (!activeTabPath || isSyntheticTab(activeTabPath)) return;
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
          if (!activeTabPath || isSyntheticTab(activeTabPath)) return;
          try {
            await vaultRepository.gitRestore([activeTabPath]);
            await handleGitRestoreFiles([activeTabPath]);
          } catch (e) {
            console.error("Error en git restore:", e);
          }
        },
      },
      {
        id: "cmd-reload-active-file",
        name: "Archivo: Recargar archivo actual desde el disco",
        category: "Archivo",
        shortcut: "Ctrl+R",
        action: async () => {
          if (!activeTabPath || isSyntheticTab(activeTabPath)) return;
          await handleReloadItems([activeTabPath]);
        },
      },
      {
        id: "cmd-reload-vault",
        name: "Bóveda: Recargar toda la bóveda y refrescar metadatos",
        category: "Bóveda",
        shortcut: "Ctrl+Shift+R",
        action: async () => {
          await handleReloadItems(['']);
        },
      },
      {
        id: "cmd-search-fulltext",
        name: "Buscar en el contenido de las notas (FTS)",
        category: "Búsqueda",
        shortcut: "Ctrl+Shift+F",
        action: () => {
          isFullTextSearchOpen = true;
        },
      },
      {
        id: "cmd-rebuild-fts-index",
        name: "Reconstruir índice de búsqueda full-text",
        category: "Búsqueda",
        action: async () => {
          await searchRepository.rebuildIndex();
        },
      },
      {
        id: "cmd-locate-active-file",
        name: "Explorador: Ubicar archivo actual en el árbol",
        category: "Navegación",
        shortcut: "Ctrl+Alt+1, Alt+L",
        action: async () => {
          if (activeRibbonTab !== 'files') {
            if (sidebarWidth < 140) {
              sidebarWidth = 240;
            }
            activeRibbonTab = 'files';
          }
          await tick();
          vaultExplorerRef?.locateActiveFile();
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
      if ((e.ctrlKey || e.metaKey) && e.altKey && !e.shiftKey) {
        if (e.key === '1' || e.code === 'Digit1' || e.code === 'Numpad1') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          if (activeRibbonTab !== 'files') {
            if (sidebarWidth < 140) {
              sidebarWidth = 240;
            }
            activeRibbonTab = 'files';
          }
          tick().then(() => {
            vaultExplorerRef?.locateActiveFile();
          });
          return;
        }
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
        } else if (e.key.toLowerCase() === 'l') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          if (activeRibbonTab !== 'files') {
            if (sidebarWidth < 140) {
              sidebarWidth = 240;
            }
            activeRibbonTab = 'files';
          }
          tick().then(() => {
            vaultExplorerRef?.locateActiveFile();
          });
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
          } else if (key === 'f') {
            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();
            isFullTextSearchOpen = true;
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
          info(`activeTabPath: ${activeTabPath}`);
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
        } else if (key === '\\' || key === '|') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          handleSplitView();
          return;
        } else if (key === 'w' || key === 'f4') {
          e.preventDefault();
          e.stopPropagation();
          e.stopImmediatePropagation();
          if (activeFocusedPane === 'right' && isSplit && activeSplitTabPath) {
            closeSplitTab(activeSplitTabPath);
          } else if (activeTabPath) {
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
    } else if (actionId === "quick-open") {
      isQuickOpenOpen = true;
    } else if (actionId === "search") {
      isFullTextSearchOpen = true;
    }
  }

  function handleSelectFullTextHit(path: string, matchedTerms: string[]) {
    if (matchedTerms && matchedTerms.length > 0) {
      pendingScrollTerms[path] = [...matchedTerms];
    }
    selectTab(path);
  }

  function handleSelectQuickOpenFile(path: string, matchedTerms?: string[]) {
    if (matchedTerms && matchedTerms.length > 0) {
      pendingScrollTerms[path] = [...matchedTerms];
    }
    selectTab(path);
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
  {#if !isDetachedWindow}
    <!-- 1. BARRA RIBBON IZQUIERDA -->
    <Ribbon bind:activeTab={activeRibbonTab} onAction={handleRibbonAction}/>

    <!-- 2. PANEL LATERAL (EXPLORADOR DE ARCHIVOS DE LA BÓVEDA) -->
    <VaultExplorer
      bind:this={vaultExplorerRef}
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
      onSelectTab={handleSelectTabFromExplorer}
      onOpenVaultFolder={handleOpenVaultFolder}
      onDeleteItem={handleDeleteItem}
      onDeleteItems={handleDeleteItems}
      onRenameItem={handleRenameItem}
      onResizeStart={handleSidebarResizeStart}
      onCollapse={toggleSidebar}
      onRefreshGit={refreshGitStatus}
      onGitRestore={handleGitRestoreFiles}
      onReloadItems={handleReloadItems}
      onOpenDiff={(path, staged) => selectTab(staged ? `diff-staged:${path}` : `diff:${path}`)}
      onOpenInNewWindow={handleOpenInNewWindow}
    />
  {/if}

  {#snippet renderPaneContent(paneTabs: string[], activePath: string | null, isRight: boolean)}
    {#if paneTabs.length === 0 || !activePath || !paneTabs.includes(activePath)}
      <EmptyWorkspace
        hasVaultItems={vaultItems.length > 0}
        onCreateNew={() => (isRight ? handleNewSplitEmptyTab() : createNewVaultItem())}
      />
    {:else}
      {#each paneTabs as tabPath (tabPath)}
        {#if tabPath.startsWith("empty:")}
          <div
            class="tab-pane"
            class:hidden={tabPath !== activePath}
          >
            <EmptyWorkspace
              hasVaultItems={vaultItems.length > 0}
              onCreateNew={() => createNewVaultItem(tabPath)}
            />
          </div>
        {:else if tabPath.startsWith("diff:") || tabPath.startsWith("diff-staged:")}
          <div
            class="tab-pane full-pane"
            class:hidden={tabPath !== activePath}
          >
            <DiffViewer
              filePath={tabPath.startsWith("diff-staged:") ? tabPath.slice("diff-staged:".length) : tabPath.slice("diff:".length)}
              isStaged={tabPath.startsWith("diff-staged:")}
              onClose={() => (isRight ? closeSplitTab(tabPath) : closeTab(tabPath))}
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
          {@const paneEditing = isRight ? isSplitEditing : isEditing}
          {@const paneViewMode = isRight ? (!isSplitEditing ? 'reading' : splitMarkdownViewMode) : (!isEditing ? 'reading' : markdownViewMode)}
          <div
            class="tab-pane"
            class:hidden={tabPath !== activePath}
            class:full-pane={isDiagramFile(tabPath) ||
              isImageFile(tabPath) ||
              isDrawingFile(tabPath) ||
              isPdfFile(tabPath)}
          >
            <BreadCrumb
              path={tabPath}
              onNavigateFolder={(folderPath) => vaultExplorerRef?.locateActiveFile(folderPath)}
              onRenameFile={(newName) => handleRenameItem(tabPath, newName)}
            />
            {#if isLoading}
              <div class="content-loading">
                <span class="spinner"></span>
                <span>Cargando contenido desde disco...</span>
              </div>
            {:else}
              {#key note?.lastReloaded}
                {#if isDiagramFile(tabPath)}
                  {#if appSettings.mermaidRenderer === 'mermaidjs'}
                    <MermaidViewer
                      {content}
                      readOnly={!paneEditing}
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
                      readOnly={!paneEditing}
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
                    bind:this={excalidrawViewers[tabPath]}
                    {content}
                    readOnly={!paneEditing}
                    onChange={(updatedContent) => {
                      if (openedNotes[tabPath]) {
                        openedNotes[tabPath].content = updatedContent;
                      }
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    onSave={(savedContent) => {
                      if (savedContent && openedNotes[tabPath]) {
                        openedNotes[tabPath].content = savedContent;
                      }
                      persistVaultItemToRust(vaultItem);
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
                      readOnly={!paneEditing}
                      vimMode={isVimMode}
                      viewMode={paneViewMode}
                      scrollToTerms={pendingScrollTerms[tabPath]}
                      onNavigate={handleNavigateLink}
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
                    src={note?.abs_path || tabPath}
                    alt={vaultItem.title || tabPath}
                    {content}
                  />
                {:else if isPdfFile(tabPath)}
                  <PdfViewer
                    bind:this={pdfViewers[tabPath]}
                    src={note?.abs_path || tabPath}
                    onSaved={() => {
                      syncState = "synced";
                    }}
                  />
                {:else}
                  <div class="editor-main-content">
                    <pre
                      style="padding: 24px; font-family: var(--code-font, monospace); white-space: pre-wrap; overflow-y: auto; height: 100%;">{content}</pre>
                  </div>
                {/if}
              {/key}
            {/if}
          </div>
        {/if}
      {/each}
    {/if}
  {/snippet}

  <!-- 3. ÁREA DE TRABAJO PRINCIPAL -->
  <main class="main-workspace" class:is-resizing={isResizingSidebar || isResizingSplit}>
    {#if !isSplit}
      <!-- MODO NORMAL: UN SOLO PANEL -->
      <EditorHeader
        bind:isEditing
        tabs={tabsInfo}
        {activeTabPath}
        {canGoBack}
        {canGoForward}
        onNavigateBack={navigateBack}
        onNavigateForward={navigateForward}
        isMarkdownFile={isMarkdownTab}
        showViewToggle={hasActiveContent && !isSyntheticTab(activeTabPath)}
        markdownViewMode={!isEditing ? "reading" : markdownViewMode}
        onChangeMarkdownView={handleChangeMarkdownView}
        title={
          activeTabPath?.startsWith("empty:")
            ? "Nueva pestaña"
            : activeTabPath?.startsWith("diff-staged:")
            ? `Diff (staged): ${activeTabPath.slice("diff-staged:".length).split("/").pop()}`
            : activeTabPath?.startsWith("diff:")
            ? `Diff: ${activeTabPath.slice("diff:".length).split("/").pop()}`
            : (currentVaultItem.relative_path || currentVaultItem.title)
        }
        showSaveButton={isEditing &&
          !!activeTabPath &&
          !isSyntheticTab(activeTabPath) &&
          (!currentVaultItem.relative_path ||
            isMarkdownFile(currentVaultItem.relative_path) ||
            isDrawingFile(currentVaultItem.relative_path) ||
            isPdfFile(currentVaultItem.relative_path))}
        onSelectTab={(path) => selectTab(path)}
        onCloseTab={(path) => closeTab(path)}
        onCloseAllTabs={closeAllTabs}
        onCloseOtherTabs={closeOtherTabs}
        onCloseTabsToRight={closeTabsToRight}
        onOpenInNewWindow={handleOpenInNewWindow}
        onReorderTab={reorderTab}
        onReceiveTabFromOtherWindow={receiveTabFromOtherWindow}
        onNewTab={handleNewEmptyTab}
        onNewFile={() => createNewVaultItem()}
        onOpenQuickOpen={() => (isQuickOpenOpen = true)}
        onSplitView={handleSplitView}
        onAction={(actionId) => {
          if (actionId === 'nav-back') navigateBack();
          else if (actionId === 'nav-forward') navigateForward();
          else if (actionId === 'new-file') createNewVaultItem();
          else if (actionId === 'quick-open') isQuickOpenOpen = true;
        }}
        onToggleView={() => {
          toggleViewMode();
        }}
        onSave={() => {
          if (activeTabPath && currentVaultItem.relative_path) {
            persistVaultItemToRust(currentVaultItem);
          }
        }}
        onOpenCommandPalette={() => (isPaletteOpen = true)}
        showWindowControls={true}
        isSplitActive={false}
      />

      <!-- CONTENEDOR DEL EDITOR CON MULTI-TAB Y CARGA BAJO DEMANDA -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="editor-container"
        bind:this={editorContainerRef}
        ondragover={handleEditorDragOver}
        ondragleave={handleEditorDragLeave}
        ondrop={handleEditorDrop}
        onpointerdown={() => { activeFocusedPane = 'left'; }}
      >
        {@render renderPaneContent(openTabPaths, activeTabPath, false)}

        {#if isDragOverSplitRight}
          <div class="split-drop-zone-overlay">
            <div class="split-drop-zone-content">
              <Columns2 size={36} />
              <span>Dividir panel a la derecha</span>
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <!-- MODO DIVIDIDO: DOS PANELES LADO A LADO -->
      <div class="split-workspace-container" bind:this={splitContainerRef}>
        <!-- PANEL IZQUIERDO -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="split-pane-wrapper left-pane"
          style="width: {splitRatio}%;"
          onpointerdown={() => { activeFocusedPane = 'left'; }}
        >
          <EditorHeader
            bind:isEditing
            tabs={tabsInfo}
            {activeTabPath}
            {canGoBack}
            {canGoForward}
            onNavigateBack={navigateBack}
            onNavigateForward={navigateForward}
            isMarkdownFile={isMarkdownTab}
            showViewToggle={hasActiveContent && !isSyntheticTab(activeTabPath)}
            markdownViewMode={!isEditing ? "reading" : markdownViewMode}
            onChangeMarkdownView={handleChangeMarkdownView}
            title={
              activeTabPath?.startsWith("empty:")
                ? "Nueva pestaña"
                : activeTabPath?.startsWith("diff-staged:")
                ? `Diff (staged): ${activeTabPath.slice("diff-staged:".length).split("/").pop()}`
                : activeTabPath?.startsWith("diff:")
                ? `Diff: ${activeTabPath.slice("diff:".length).split("/").pop()}`
                : (currentVaultItem.relative_path || currentVaultItem.title)
            }
            showSaveButton={isEditing &&
              !!activeTabPath &&
              !isSyntheticTab(activeTabPath) &&
              (!currentVaultItem.relative_path ||
                isMarkdownFile(currentVaultItem.relative_path) ||
                isDrawingFile(currentVaultItem.relative_path) ||
                isPdfFile(currentVaultItem.relative_path))}
            onSelectTab={(path) => selectTab(path)}
            onCloseTab={(path) => closeTab(path)}
            onCloseAllTabs={closeAllTabs}
            onCloseOtherTabs={closeOtherTabs}
            onCloseTabsToRight={closeTabsToRight}
            onOpenInNewWindow={handleOpenInNewWindow}
            onReorderTab={reorderTab}
            onReceiveTabFromOtherWindow={(p, idx, src) => receiveTabFromSplitOrWindow(p, false, idx, src)}
            onNewTab={handleNewEmptyTab}
            onNewFile={() => createNewVaultItem()}
            onOpenQuickOpen={() => (isQuickOpenOpen = true)}
            onSplitView={handleSplitView}
            onAction={(actionId) => {
              if (actionId === 'nav-back') navigateBack();
              else if (actionId === 'nav-forward') navigateForward();
              else if (actionId === 'new-file') createNewVaultItem();
              else if (actionId === 'quick-open') isQuickOpenOpen = true;
            }}
            onToggleView={() => {
              toggleViewMode();
            }}
            onSave={() => {
              if (activeTabPath && currentVaultItem.relative_path) {
                persistVaultItemToRust(currentVaultItem);
              }
            }}
            onOpenCommandPalette={() => (isPaletteOpen = true)}
            showWindowControls={false}
            isSplitActive={true}
          />
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="editor-container"
            ondragover={(e) => handlePaneDragOver(e, 'left')}
            ondrop={(e) => handlePaneDrop(e, 'left')}
          >
            {@render renderPaneContent(openTabPaths, activeTabPath, false)}
          </div>
        </div>

        <!-- DIVISOR REDIMENSIONABLE -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="split-resizer"
          class:is-resizing={isResizingSplit}
          onpointerdown={handleSplitResizeStart}
          title="Arrastrar para redimensionar paneles"
        >
          <div class="split-resizer-line"></div>
        </div>

        <!-- PANEL DERECHO -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="split-pane-wrapper right-pane"
          style="width: {100 - splitRatio}%;"
          onpointerdown={() => { activeFocusedPane = 'right'; }}
        >
          <EditorHeader
            bind:isEditing={isSplitEditing}
            tabs={rightTabsInfo}
            activeTabPath={activeSplitTabPath}
            canGoBack={false}
            canGoForward={false}
            isMarkdownFile={isMarkdownSplitTab}
            showViewToggle={hasActiveSplitContent && !isSyntheticTab(activeSplitTabPath)}
            markdownViewMode={!isSplitEditing ? "reading" : splitMarkdownViewMode}
            onChangeMarkdownView={handleChangeSplitMarkdownView}
            title={
              activeSplitTabPath?.startsWith("empty:")
                ? "Nueva pestaña"
                : activeSplitTabPath?.startsWith("diff-staged:")
                ? `Diff (staged): ${activeSplitTabPath.slice("diff-staged:".length).split("/").pop()}`
                : activeSplitTabPath?.startsWith("diff:")
                ? `Diff: ${activeSplitTabPath.slice("diff:".length).split("/").pop()}`
                : (currentSplitVaultItem.relative_path || currentSplitVaultItem.title)
            }
            showSaveButton={isSplitEditing &&
              !!activeSplitTabPath &&
              !isSyntheticTab(activeSplitTabPath) &&
              (!currentSplitVaultItem.relative_path ||
                isMarkdownFile(currentSplitVaultItem.relative_path) ||
                isDrawingFile(currentSplitVaultItem.relative_path) ||
                isPdfFile(currentSplitVaultItem.relative_path))}
            onSelectTab={(path) => selectSplitTab(path)}
            onCloseTab={(path) => closeSplitTab(path)}
            onCloseAllTabs={closeAllSplitTabs}
            onCloseOtherTabs={closeOtherSplitTabs}
            onCloseTabsToRight={closeSplitTabsToRight}
            onOpenInNewWindow={handleOpenInNewWindow}
            onReorderTab={reorderSplitTab}
            onReceiveTabFromOtherWindow={(p, idx, src) => receiveTabFromSplitOrWindow(p, true, idx, src)}
            onNewTab={handleNewSplitEmptyTab}
            onNewFile={() => createNewVaultItem()}
            onOpenQuickOpen={() => (isQuickOpenOpen = true)}
            onSplitView={handleSplitView}
            onAction={(actionId) => {
              if (actionId === 'new-file') createNewVaultItem();
              else if (actionId === 'quick-open') isQuickOpenOpen = true;
            }}
            onToggleView={() => {
              toggleSplitViewMode();
            }}
            onSave={() => {
              if (activeSplitTabPath && currentSplitVaultItem.relative_path) {
                persistVaultItemToRust(currentSplitVaultItem);
              }
            }}
            onOpenCommandPalette={() => (isPaletteOpen = true)}
            showWindowControls={true}
            isSplitActive={true}
          />
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="editor-container"
            ondragover={(e) => handlePaneDragOver(e, 'right')}
            ondrop={(e) => handlePaneDrop(e, 'right')}
          >
            {@render renderPaneContent(splitTabPaths, activeSplitTabPath, true)}
          </div>
        </div>
      </div>
    {/if}

    <!-- BARRA DE ESTADO INFERIOR -->
    <StatusBar
      wordCount={vaultItems.length > 0 && currentFocusedTabPath ? displayWordCount : 0}
      charCount={vaultItems.length > 0 && currentFocusedTabPath ? displayCharCount : 0}
      line={vaultItems.length > 0 && currentFocusedTabPath ? displayLine : 0}
      col={vaultItems.length > 0 && currentFocusedTabPath ? displayCol : 0}
      hasSelection={vaultItems.length > 0 && !!currentFocusedTabPath && currentSelection.hasSelection}
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
    onSelectFile={handleSelectQuickOpenFile}
  />

  <!-- 6. BÚSQUEDA EN EL CONTENIDO DE NOTAS (FULL-TEXT SEARCH CTRL+SHIFT+F) -->
  <FullTextSearch
    bind:isOpen={isFullTextSearchOpen}
    onSelectHit={handleSelectFullTextHit}
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

  .split-workspace-container {
    flex: 1;
    display: flex;
    flex-direction: row;
    width: 100%;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .split-pane-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    position: relative;
  }

  .split-resizer {
    width: 6px;
    height: 100%;
    cursor: col-resize;
    position: relative;
    flex-shrink: 0;
    user-select: none;
    z-index: 20;
    background-color: var(--border-primary, #d0d7de);
    transition: background-color 0.15s ease;
  }

  .split-resizer:hover,
  .split-resizer.is-resizing {
    background-color: var(--accent, #0969da);
  }

  .split-resizer-line {
    width: 100%;
    height: 100%;
  }

  .split-drop-zone-overlay {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 50%;
    background-color: rgba(9, 105, 218, 0.12);
    border: 2px dashed var(--accent, #0969da);
    border-radius: 8px;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    animation: fadeIn 0.15s ease-out;
  }

  .split-drop-zone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: var(--accent, #0969da);
    font-weight: 600;
    font-size: 14px;
    padding: 16px 24px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--accent, #0969da);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
