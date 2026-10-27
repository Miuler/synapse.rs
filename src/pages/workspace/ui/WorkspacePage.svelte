<script lang="ts">
  import {debug, info, warn, error} from '@tauri-apps/plugin-log';
  import {onMount, tick} from "svelte";
  import {Ribbon} from "@widgets/ribbon";
  import WorkspacePane from "./WorkspacePane.svelte";
  import {StatusBar} from "@widgets/status-bar";
  import {CommandPalette} from "@widgets/command-palette";
  import {QuickOpen} from "@widgets/quick-open";
  import {FullTextSearch} from "@widgets/full-text-search";
  import {EmptyWorkspace} from "@widgets/empty-workspace";
  import {WelcomeScreen} from "@widgets/welcome-screen";
  import {VaultExplorer} from "@features/vault-explorer";
  import {MarkdownViewer} from "@features/markdown-editor";
  import {MermanViewer} from "@features/merman-editor";
  import {MermaidViewer} from "@features/mermaid-editor";
  import {D2Viewer} from "@features/d2-editor";
  import {ExcalidrawViewer, ObsidianExcalidrawViewer} from "@features/excalidraw-editor";
  import {ImageViewer} from "@features/image-viewer";
  import {PdfViewer} from "@features/pdf-viewer";
  import {DiffViewer} from "@features/diff-viewer";
  import {RevisionViewer} from "@features/revision-viewer";
  import {CommunityPluginsModal} from "@features/community-plugins";
  import {GraphViewer} from "@features/graph-viewer";
  import {TagsExplorer} from "@features/tags-explorer";
  import {RelationsInspector} from "@features/relations-inspector";
  import {GitManagementDialogs} from "@features/git-management";
  import {appSettings, shortcutsConfig, matchesShortcut, type ExcalidrawViewerType} from "@entities/settings";
  import { t } from "@shared/lib/i18n";
  import {
    isImageFile,
    isMarkdownFile,
    isDiagramFile,
    isD2File,
    isDrawingFile,
    isPdfFile,
    isOfficeFile,
    loadSupportedFileTypes,
    DEFAULT_APP_CONFIG,
  } from "@entities/file-type";
  import {vaultApi, type VaultItem, type OpenedNote, type MarkdownViewMode} from "@entities/vault-item";
  import {gitApi} from "@entities/git";
  import {commandRegistry} from "@entities/command";
  import {vaultRepository, searchRepository, toggleDevtools, isTauriEnvironment, type GitFileStatusKind} from "@shared/repositories";
  import {systemApi, type McpRuntimeStatus} from "@shared/api";
  import {createSynapseApp, type SynapseApp, isExcalidrawMarkdown, resolvePluginViewType, createCanonicalExcalidrawContent} from "@shared/obsidian-api";
  import {listen, emit} from "@tauri-apps/api/event";
  import {getCurrentWebviewWindow, getAllWebviewWindows} from "@tauri-apps/api/webviewWindow";
  import {BreadCrumb} from "@widgets/breadcrumb";
  import {UnsavedChangesDialog} from "@shared/ui/unsaved-changes-dialog";
  import {
    isSyntheticTab,
    isDrawingTab,
    cleanTabTitle,
    getTabDisplayTitle,
    parseTabDragData,
    getInitialSidebarWidth,
    getInitialSplitRatio,
    TabHistoryState,
    WorkspaceLayoutState,
    UnsavedChangesState,
  } from "../model";

  interface VaultFsChangeEvent {
    paths: string[];
    deleted: string[];
  }

  // Estados reactivos con Runas de Svelte 5
  let activeRibbonTab = $state("files");
  let vaultExplorerRef = $state<any>(null);
  let isPaletteOpen = $state(false);
  let isQuickOpenOpen = $state(false);
  let isFullTextSearchOpen = $state(false);
  let quickOpenSearchQuery = $state("");
  let ftsSearchQuery = $state("");
  let isPluginsModalOpen = $state(false);

  function openQuickOpenModal() {
    if (isFullTextSearchOpen) {
      const carriedQuery = ftsSearchQuery;
      isFullTextSearchOpen = false;
      ftsSearchQuery = "";
      quickOpenSearchQuery = carriedQuery;
    }
    if (isPaletteOpen) {
      isPaletteOpen = false;
    }
    isQuickOpenOpen = true;
  }

  function openFullTextSearchModal() {
    if (isQuickOpenOpen) {
      const carriedQuery = quickOpenSearchQuery.startsWith("?")
        ? quickOpenSearchQuery.slice(1).trim()
        : quickOpenSearchQuery;
      isQuickOpenOpen = false;
      quickOpenSearchQuery = "";
      ftsSearchQuery = carriedQuery;
    }
    if (isPaletteOpen) {
      isPaletteOpen = false;
    }
    isFullTextSearchOpen = true;
  }
  let pendingScrollTerms = $state<Record<string, string[]>>({});
  let isEditing = $state(false);
  let isVimMode = $derived(appSettings.vimMode);
  let markdownViewMode = $state<MarkdownViewMode>("reading");
  let isConnectedToRust = $state(false);
  let syncState = $state<"synced" | "saving" | "error">("synced");
  let isDetachedWindow = $state(false);
  let currentWindowLabel = $state('main');
  let showWelcomeScreen = $state(true);
  let hasActiveWorkspaceLoaded = $state(false);
  let isStandaloneMode = $state(false);

  // Estado de control de versiones Git
  let isGitRepo = $state(false);
  let gitBranch = $state<string | null>(null);
  let gitStatuses = $state<Record<string, GitFileStatusKind>>({});

  async function refreshGitStatus() {
    if (!vaultRepository.isConnected() || isStandaloneMode) return;
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
          if (isPdfFile(p)) {
            openedNotes[p] = {
              ...openedNotes[p],
              lastReloaded: Date.now(),
            };
            if (pdfViewers[p]?.reload) {
              pdfViewers[p].reload();
            }
          } else if (isImageFile(p) && !isDrawingFile(p)) {
            openedNotes[p] = {
              ...openedNotes[p],
              lastReloaded: Date.now(),
            };
          } else {
            const noteData = await vaultRepository.readNote(p);
            if (noteData) {
              openedNotes[p] = {
                ...openedNotes[p],
                content: noteData.content ?? "",
                savedContent: noteData.content ?? "",
                isLoading: false,
                isLoaded: true,
                lastReloaded: Date.now(),
              };
            }
          }
        } catch (e) {
          console.error("Error al recargar archivo restaurado:", e);
        }
      } else {
        // Si no estaba en openedNotes pero está abierto como pestaña PDF
        if (isPdfFile(p) && pdfViewers[p]?.reload) {
          pdfViewers[p].reload();
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
            view_mode: n.view_mode,
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
          if (isPdfFile(openPath)) {
            openedNotes[openPath] = {
              ...openedNotes[openPath],
              lastReloaded: Date.now(),
            };
            if (pdfViewers[openPath]?.reload) {
              pdfViewers[openPath].reload();
            }
          } else if (isImageFile(openPath) && !isDrawingFile(openPath)) {
            openedNotes[openPath] = {
              ...openedNotes[openPath],
              lastReloaded: Date.now(),
            };
          } else {
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
          }
        } catch (e) {
          console.error(`Error al recargar archivo en memoria ${openPath}:`, e);
        }
      }
    }

    // 3. Refrescar estado de Git
    await refreshGitStatus();
  }

  async function restoreSidebarWidth() {
    try {
      const uiState = await vaultRepository.getVaultUiState();
      if (uiState?.sidebar_width && uiState.sidebar_width >= 140 && uiState.sidebar_width <= 900) {
        layoutState.setSidebarWidth(uiState.sidebar_width);
      }
      if (uiState?.locked_files) {
        unsavedChanges.setLockedFiles(uiState.locked_files);
      }
    } catch (e) {
      console.error("Error al restaurar estado de la UI de la bóveda:", e);
    }
  }

  // Controladores de estado composables con Runes de Svelte 5
  const layoutState = new WorkspaceLayoutState();
  const unsavedChanges = new UnsavedChangesState();

  // Conjunto reactivo de rutas relativas de archivos bloqueados
  let lockedFiles = $derived(unsavedChanges.lockedFiles);

  // Diálogo para advertir sobre cambios sin persistir en archivos bloqueados
  let unsavedDialogOpen = $state(false);
  let unsavedDialogFiles = $state<string[]>([]);
  let unsavedDialogIsWindowClose = $state(false);

  // Estado reactivo del Servidor MCP en proceso
  let mcpStatus = $state<McpRuntimeStatus | null>(null);

  async function refreshMcpStatus() {
    try {
      const status = await systemApi.getMcpServerStatus();
      if (status) {
        mcpStatus = status;
      }
    } catch (e) {
      console.warn('Error al consultar estado MCP:', e);
    }
  }

  async function handleToggleMcp() {
    try {
      const updated = await systemApi.toggleMcpServer();
      if (updated) {
        mcpStatus = updated;
        appSettings.setMcpServerEnabled(updated.running);
      }
    } catch (e) {
      console.error('Error al alternar servidor MCP:', e);
    }
  }

  async function copyMcpUrlToClipboard() {
    if (mcpStatus?.endpoint) {
      try {
        await navigator.clipboard.writeText(mcpStatus.endpoint);
      } catch (e) {
        console.warn('Error al copiar endpoint MCP al portapapeles:', e);
      }
    }
  }
  let isForceSavingUnsaved = $state(false);
  let pendingCloseAction = $state<{
    type: 'single-tab' | 'single-split-tab' | 'multiple-tabs' | 'window-close' | 'manual-save';
    paths: string[];
  } | null>(null);

  async function handleToggleLockFile(path: string) {
    await unsavedChanges.toggleLock(path, isStandaloneMode);
  }

  // Lista de metadatos de elementos de la bóveda (VaultItem[])
  let vaultItems = $state<VaultItem[]>([]);
  let openTabPaths = $state<string[]>([]);
  let activeTabPath = $state<string | null>(null);
  let lastActiveDocumentPath = $state<string | null>(null);

  // Estado para panel dividido (Split View vertical)
  let splitTabPaths = $state<string[]>([]);
  let activeSplitTabPath = $state<string | null>(null);
  let splitRatio = $derived(layoutState.splitRatio);
  let isResizingSplit = $derived(layoutState.isResizingSplit);
  let isDragOverSplitRight = $state(false);
  let activeFocusedPane = $state<'left' | 'right'>('left');
  let splitMarkdownViewMode = $state<MarkdownViewMode>('live');
  let isSplitEditing = $state(true);
  let isSplit = $derived(splitTabPaths.length > 0 && activeSplitTabPath !== null);
  let splitContainerRef = $state<HTMLDivElement | null>(null);
  let sidebarWidth = $derived(layoutState.sidebarWidth);
  let isResizingSidebar = $derived(layoutState.isResizingSidebar);

  $effect(() => {
    layoutState.splitContainerRef = splitContainerRef;
  });

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

  // Historial de navegación de pestañas (Back / Forward)
  const tabHistory = new TabHistoryState();

  // Instancia de la fachada Obsidian API para soporte de plugins
  let synapseApp = $state<SynapseApp | null>(null);

  function ensureSynapseAppLoaded() {
    if (!synapseApp && vaultRepository.isConnected()) {
      try {
        synapseApp = createSynapseApp(vaultRepository);
        vaultRepository.getEnabledPlugins().then(async (enabledList) => {
          for (const pluginId of enabledList) {
            try {
              const entry = await synapseApp?.plugins.loadPlugin(pluginId);
              if (entry) {
                console.log(`[Workspace] Plugin de Obsidian '${entry.manifest.name}' (${entry.manifest.id}) cargado y activo.`);
              }
            } catch (e: any) {
              console.log(`[Workspace] Plugin '${pluginId}' no cargado automáticamente:`, e.message);
            }
          }
        }).catch((e) => {
          console.warn('[Workspace] Error al cargar plugins habilitados:', e);
        });
      } catch (e) {
        console.warn('[Workspace] Error al instanciar SynapseApp:', e);
      }
    }
  }

  // Registro de la posición física (índice en la barra de pestañas) de los archivos cerrados
  let lastClosedTabIndex = $state<Record<string, number>>({});

  let canGoBack = $derived(tabHistory.canGoBack);
  let canGoForward = $derived(tabHistory.canGoForward);

  function recordTabVisit(path: string) {
    tabHistory.recordVisit(path);
  }

  function safeNavigateBack() {
    tabHistory.navigateBack(selectTab);
  }

  function safeNavigateForward() {
    tabHistory.navigateForward(selectTab);
  }

  const navigateBack = safeNavigateBack;
  const navigateForward = safeNavigateForward;

  function removeTabFromHistory(path: string) {
    tabHistory.removeTab(path, activeTabPath);
  }

  let saveTabsTimeout: ReturnType<typeof setTimeout> | null = null;
  function persistTabsState() {
    if (!isConnectedToRust || isDetachedWindow || isStandaloneMode) return;
    if (saveTabsTimeout) clearTimeout(saveTabsTimeout);
    saveTabsTimeout = setTimeout(() => {
      const tabs = openTabPaths
        .filter((p) => p && !isSyntheticTab(p))
        .map((p) => ({
          path: p,
          view_mode: isDrawingTab(p, openedNotes[p]?.content)
            ? (openedNotes[p]?.excalidrawViewer || appSettings.defaultExcalidrawViewer)
            : (openedNotes[p]?.viewMode || "reading"),
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
          const vItem = vaultItems.find((v) => v.relative_path === tab.path);
          const isDrawing = isDrawingTab(tab.path);
          const isTabLocked = unsavedChanges.isLocked(tab.path);
          const memoryMode = (tab.view_mode || vItem?.view_mode) as MarkdownViewMode | undefined;
          const mode: MarkdownViewMode = (!isDrawing && !isTabLocked && memoryMode && ['live', 'source', 'reading'].includes(memoryMode))
            ? memoryMode
            : "reading";
          const excalViewer: ExcalidrawViewerType | undefined = (tab.view_mode === 'obsidian' || tab.view_mode === 'native')
            ? tab.view_mode
            : undefined;

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
              excalidrawViewer: excalViewer,
            };
          } else {
            openedNotes[tab.path].viewMode = mode;
            if (excalViewer) openedNotes[tab.path].excalidrawViewer = excalViewer;
          }
        }

        if (openTabPaths.length > 0) {
          const targetActive = state.active_tab && openTabPaths.includes(state.active_tab)
            ? state.active_tab
            : openTabPaths[0];

          activeTabPath = targetActive;
          const isTargetDrawing = isDrawingTab(targetActive);
          const isTargetLocked = unsavedChanges.isLocked(targetActive);
          const activeMode = (isTargetDrawing || isTargetLocked) ? "reading" : (openedNotes[targetActive]?.viewMode || "reading");
          markdownViewMode = activeMode;
          isEditing = activeMode !== "reading";
          ensureContentLoaded(targetActive);

          // Precargar en segundo plano las demás pestañas para evitar desfasajes o lienzos vacíos
          for (const p of openTabPaths) {
            if (p !== targetActive && !isSyntheticTab(p)) {
              ensureContentLoaded(p);
            }
          }

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
    isEditing = newMode !== "reading";
    if (activeTabPath && !isSyntheticTab(activeTabPath)) {
      if (openedNotes[activeTabPath]) {
        openedNotes[activeTabPath].viewMode = newMode;
      }
      const vItem = vaultItems.find((v) => v.relative_path === activeTabPath);
      if (vItem) vItem.view_mode = newMode;
      vaultRepository.setNoteViewMode(activeTabPath, newMode);
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

    if (isMarkdownFile(activeTabPath) && !isDrawingTab(activeTabPath, activeContent)) {
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
      const vItem = vaultItems.find((v) => v.relative_path === activeTabPath);
      if (vItem) vItem.view_mode = nextMode;
      vaultRepository.setNoteViewMode(activeTabPath, nextMode);
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
      const isDrawing = isDrawingTab(path);
      const memoryMode = (vaultItem?.view_mode) as MarkdownViewMode | undefined;
      const defaultMode: MarkdownViewMode = (lockedFiles.has(path) || isDrawing) ? "reading" : (memoryMode || "reading");
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
        const isDrawing = isDrawingTab(path, fetchedContent);
        // Si es un dibujo o está bloqueado, por defecto SIEMPRE debe abrir en modo lectura ('reading')
        const isAlreadyLoaded = openedNotes[path]?.isLoaded === true;
        const previousMode = openedNotes[path]?.viewMode;
        const hasExplicitEditSession = isAlreadyLoaded && (previousMode === 'live' || previousMode === 'source');
        const currentMode: MarkdownViewMode = (lockedFiles.has(path) || isDrawing)
          ? (hasExplicitEditSession ? previousMode : "reading")
          : (previousMode || ((noteData.view_mode as MarkdownViewMode) || (vaultItem?.view_mode as MarkdownViewMode) || "reading"));

        if (isDrawing && !hasExplicitEditSession) {
          if (path === activeTabPath) {
            markdownViewMode = "reading";
            isEditing = false;
          }
          if (path === activeSplitTabPath) {
            splitMarkdownViewMode = "reading";
            isSplitEditing = false;
          }
        }

        const prevExcalViewer = openedNotes[path]?.excalidrawViewer;
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
          excalidrawViewer: prevExcalViewer,
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

  function flushPendingSaveForPath(targetPath: string | null | undefined) {
    if (!targetPath || isSyntheticTab(targetPath)) return;
    if (lockedFiles.has(targetPath)) return;
    if (isDrawingTab(targetPath, openedNotes[targetPath]?.content) && excalidrawViewers[targetPath]?.getCurrentSerializedScene) {
      try {
        const fresh = excalidrawViewers[targetPath].getCurrentSerializedScene();
        if (fresh && openedNotes[targetPath]) {
          openedNotes[targetPath].content = fresh;
        }
      } catch (e) {
        console.warn(`Error al extraer escena antes de cambiar de pestaña en ${targetPath}:`, e);
      }
    }

    const note = openedNotes[targetPath];
    if (note && note.content !== note.savedContent) {
      const vItem = vaultItems.find((v) => v.relative_path === targetPath) || {
        id: "0",
        title: note.title || targetPath,
        relative_path: targetPath,
        abs_path: note.abs_path,
      };
      persistVaultItemToRust(vItem);
    }
  }

  function handleWindowBlur() {
    if (activeTabPath && !isSyntheticTab(activeTabPath)) {
      flushPendingSaveForPath(activeTabPath);
    }
    if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)) {
      flushPendingSaveForPath(activeSplitTabPath);
    }
  }

  $effect(() => {
    if (activeTabPath && !isSyntheticTab(activeTabPath) && openedNotes[activeTabPath]) {
      const targetMode: MarkdownViewMode = isEditing
        ? (markdownViewMode === 'reading' ? 'live' : markdownViewMode)
        : 'reading';
      if (openedNotes[activeTabPath].viewMode !== targetMode) {
        openedNotes[activeTabPath].viewMode = targetMode;
        const vItem = vaultItems.find((v) => v.relative_path === activeTabPath);
        if (vItem) vItem.view_mode = targetMode;
        vaultRepository.setNoteViewMode(activeTabPath, targetMode);
        persistTabsState();
      }
    }
  });

  $effect(() => {
    if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath) && openedNotes[activeSplitTabPath]) {
      const targetMode: MarkdownViewMode = isSplitEditing
        ? (splitMarkdownViewMode === 'reading' ? 'live' : splitMarkdownViewMode)
        : 'reading';
      if (openedNotes[activeSplitTabPath].viewMode !== targetMode) {
        openedNotes[activeSplitTabPath].viewMode = targetMode;
        const vItem = vaultItems.find((v) => v.relative_path === activeSplitTabPath);
        if (vItem) vItem.view_mode = targetMode;
        vaultRepository.setNoteViewMode(activeSplitTabPath, targetMode);
        persistTabsState();
      }
    }
  });

  function selectTab(path: string, viewerPreference?: ExcalidrawViewerType) {
    if (!path) return;
    if (isOfficeFile(path)) {
      systemApi.openFileWithDefaultApp(path).catch((err) => {
        console.warn('Error al abrir archivo con la aplicación por defecto del sistema:', err);
      });
      return;
    }
    const previousActivePath = activeTabPath;

    if (previousActivePath && previousActivePath !== path) {
      flushPendingSaveForPath(previousActivePath);
    }

    // Si la pestaña actual es una pestaña vacía y seleccionamos un archivo nuevo, lo sustituye en esa pestaña
    if (activeTabPath && activeTabPath.startsWith("empty:") && !openTabPaths.includes(path)) {
      const idx = openTabPaths.indexOf(activeTabPath);
      if (idx !== -1) {
        openTabPaths[idx] = path;
        delete openedNotes[activeTabPath];
        delete tabSelections[activeTabPath];
        if (previousActivePath) {
          tabHistory.replaceTab(previousActivePath, path);
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
      lastActiveDocumentPath = path;
      const vItem = vaultItems.find((v) => v.relative_path === path);
      const isLocked = lockedFiles.has(path);
      const isDrawing = isDrawingTab(path, openedNotes[path]?.content);
      const existingMode = openedNotes[path]?.viewMode;
      const isAlreadyLoaded = openedNotes[path]?.isLoaded === true;
      const hasExplicitEditSession = isAlreadyLoaded && (existingMode === 'live' || existingMode === 'source');
      const validMemoryMode = vItem?.view_mode && ['live', 'source', 'reading'].includes(vItem.view_mode) ? (vItem.view_mode as MarkdownViewMode) : undefined;
      const mode: MarkdownViewMode = (isLocked || isDrawing)
        ? (hasExplicitEditSession ? (existingMode as MarkdownViewMode) : "reading")
        : (existingMode || validMemoryMode || "reading");
      markdownViewMode = mode;
      isEditing = mode !== "reading";
      ensureContentLoaded(path);
      if (viewerPreference) {
        if (!openedNotes[path]) {
          openedNotes[path] = {
            relative_path: path,
            abs_path: vItem?.abs_path,
            title: vItem?.title || path.split("/").pop() || path,
            content: "",
            savedContent: "",
            encoding: "---",
            isLoading: true,
            isLoaded: false,
            viewMode: mode,
            excalidrawViewer: viewerPreference,
          };
        } else {
          openedNotes[path].excalidrawViewer = viewerPreference;
        }
      }
      recentFiles = [path, ...recentFiles.filter((p) => p !== path)].slice(0, 15);
      vaultRepository.recordNoteOpened(path);
    }
    recordTabVisit(path);
    persistTabsState();
  }

  function getExcalidrawViewerForTab(tabPath: string): ExcalidrawViewerType {
    return openedNotes[tabPath]?.excalidrawViewer || appSettings.defaultExcalidrawViewer;
  }

  function switchExcalidrawViewerForTab(tabPath: string, viewer: ExcalidrawViewerType) {
    if (openedNotes[tabPath]) {
      openedNotes[tabPath].excalidrawViewer = viewer;
    } else {
      openedNotes[tabPath] = {
        relative_path: tabPath,
        title: tabPath.split("/").pop() || tabPath,
        content: "",
        savedContent: "",
        encoding: "---",
        isLoading: false,
        excalidrawViewer: viewer,
      };
    }
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
            const vItem = vaultItems.find((v) => v.relative_path === activeTabPath);
            const isLocked = lockedFiles.has(activeTabPath);
            const isDrawing = isDrawingTab(activeTabPath, openedNotes[activeTabPath]?.content);
            const existingMode = openedNotes[activeTabPath]?.viewMode;
            const hasExplicitEditSession = existingMode === 'live' || existingMode === 'source';
            const validMemoryMode = vItem?.view_mode && ['live', 'source', 'reading'].includes(vItem.view_mode) ? (vItem.view_mode as MarkdownViewMode) : undefined;
            const nextMode: MarkdownViewMode = (isLocked || isDrawing)
              ? (hasExplicitEditSession ? (existingMode as MarkdownViewMode) : "reading")
              : (existingMode || validMemoryMode || "reading");
            markdownViewMode = nextMode;
            isEditing = nextMode !== "reading";
            ensureContentLoaded(activeTabPath);
          }
        } else {
          activeTabPath = null;
        }

        // Si la pestaña cerrada era la activa, sincronizar tabHistoryIndex con la nueva pestaña activa
        if (activeTabPath && !isSyntheticTab(path)) {
          tabHistory.syncActiveTab(activeTabPath);
        }
      }
    }
    // Si es un archivo PDF con visor activo y no está bloqueado, forzar guardado síncrono/inmediato antes de destruir
    if (isPdfFile(path) && !unsavedChanges.isLocked(path) && pdfViewers[path]?.saveDocument) {
      pdfViewers[path].saveDocument().catch((err: any) => {
        console.error("Error al persistir PDF al cerrar pestaña:", err);
      });
    }

    // Liberar memoria consolidada del contenido de la nota para mantener bajo consumo de RAM
    delete openedNotes[path];
    delete excalidrawViewers[path];
    delete pdfViewers[path];
    delete markdownViewers[path];
    delete recentSaveTimestamps[path];
    // Se conserva tabSelections[path] para preservar la posición del cursor si se reabre

    // Solo las pestañas temporales sintéticas se retiran del historial
    if (isSyntheticTab(path)) {
      removeTabFromHistory(path);
      delete lastClosedTabIndex[path];
      delete tabSelections[path];
    }

    // Si no queda ningún tab abierto, crear automáticamente una pestaña vacía o volver a bienvenida si es modo puntual
    if (openTabPaths.length === 0) {
      if (isStandaloneMode) {
        isStandaloneMode = false;
        showWelcomeScreen = true;
        hasActiveWorkspaceLoaded = false;
        vaultItems = [];
      } else {
        handleNewEmptyTab();
      }
    }
    persistTabsState();
  }

  function closeAllTabs() {
    openTabPaths = [];
    activeTabPath = null;
    openedNotes = {};
    excalidrawViewers = {};
    pdfViewers = {};
    markdownViewers = {};
    recentSaveTimestamps = {};
    tabSelections = {};
    tabHistory.clear();
    if (isStandaloneMode) {
      isStandaloneMode = false;
      showWelcomeScreen = true;
      hasActiveWorkspaceLoaded = false;
      vaultItems = [];
    } else {
      handleNewEmptyTab();
    }
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

  function isTabDirty(path: string): boolean {
    if (!path || isSyntheticTab(path)) return false;
    const note = openedNotes[path];
    if (!note) return false;
    if (isDrawingTab(path, note.content) && excalidrawViewers[path]?.getCurrentSerializedScene) {
      try {
        const fresh = excalidrawViewers[path].getCurrentSerializedScene();
        if (fresh && fresh !== note.savedContent) return true;
      } catch {}
    }
    return note.isLoaded ? note.content !== note.savedContent : false;
  }

  function requestCloseTab(path: string) {
    if (lockedFiles.has(path) && isTabDirty(path)) {
      pendingCloseAction = { type: 'single-tab', paths: [path] };
      unsavedDialogFiles = [path];
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }
    closeTab(path);
  }

  function requestCloseSplitTab(path: string) {
    if (lockedFiles.has(path) && isTabDirty(path)) {
      pendingCloseAction = { type: 'single-split-tab', paths: [path] };
      unsavedDialogFiles = [path];
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }
    closeSplitTab(path);
  }

  function requestCloseOtherTabs(targetPath: string) {
    const tabsToClose = openTabPaths.filter((p) => p !== targetPath);
    const dirtyLocked = tabsToClose.filter((p) => lockedFiles.has(p) && isTabDirty(p));
    if (dirtyLocked.length > 0) {
      pendingCloseAction = { type: 'multiple-tabs', paths: tabsToClose };
      unsavedDialogFiles = dirtyLocked;
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }
    closeOtherTabs(targetPath);
  }

  function requestCloseTabsToRight(targetPath: string) {
    const idx = openTabPaths.indexOf(targetPath);
    if (idx === -1) return;
    const tabsToClose = openTabPaths.slice(idx + 1);
    const dirtyLocked = tabsToClose.filter((p) => lockedFiles.has(p) && isTabDirty(p));
    if (dirtyLocked.length > 0) {
      pendingCloseAction = { type: 'multiple-tabs', paths: tabsToClose };
      unsavedDialogFiles = dirtyLocked;
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }
    closeTabsToRight(targetPath);
  }

  function requestCloseAllTabs() {
    const tabsToClose = [...openTabPaths];
    const dirtyLocked = tabsToClose.filter((p) => lockedFiles.has(p) && isTabDirty(p));
    if (dirtyLocked.length > 0) {
      pendingCloseAction = { type: 'multiple-tabs', paths: tabsToClose };
      unsavedDialogFiles = dirtyLocked;
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }
    closeAllTabs();
  }

  function handleUnsavedDialogAbort() {
    unsavedDialogOpen = false;
    pendingCloseAction = null;
  }

  async function handleUnsavedDialogDiscard() {
    unsavedDialogOpen = false;
    if (!pendingCloseAction) return;
    const action = pendingCloseAction;
    pendingCloseAction = null;

    // Revertir cambios en memoria al estado guardado en disco
    for (const p of unsavedDialogFiles) {
      if (openedNotes[p]) {
        openedNotes[p] = {
          ...openedNotes[p],
          content: openedNotes[p].savedContent,
          lastReloaded: Date.now(),
        };
      }
    }

    if (action.type === 'manual-save') {
      return;
    } else if (action.type === 'single-tab') {
      closeTab(action.paths[0]);
    } else if (action.type === 'single-split-tab') {
      closeSplitTab(action.paths[0]);
    } else if (action.type === 'multiple-tabs') {
      for (const p of action.paths) {
        closeTab(p);
      }
    } else if (action.type === 'window-close') {
      if (isTauriEnvironment()) {
        try {
          const appWindow = getCurrentWebviewWindow();
          if (appWindow.label === 'main') {
            const allWindows = await getAllWebviewWindows();
            for (const win of allWindows) {
              if (win.label !== 'main') {
                try {
                  await win.destroy();
                } catch {}
              }
            }
          }
          await appWindow.destroy();
        } catch {}
      }
    }
  }

  async function handleUnsavedDialogForceSave() {
    if (!pendingCloseAction || isForceSavingUnsaved) return;
    isForceSavingUnsaved = true;
    const action = pendingCloseAction;

    try {
      for (const p of unsavedDialogFiles) {
        if (isDrawingTab(p, openedNotes[p]?.content) && excalidrawViewers[p]?.getCurrentSerializedScene) {
          try {
            const fresh = excalidrawViewers[p].getCurrentSerializedScene();
            if (fresh && openedNotes[p]) {
              openedNotes[p].content = fresh;
            }
          } catch {}
        }
        const note = openedNotes[p];
        const vItem = vaultItems.find((v) => v.relative_path === p) || {
          id: "0",
          title: note?.title || p,
          relative_path: p,
          abs_path: note?.abs_path,
        };
        await persistVaultItemToRust(vItem, undefined, true);
      }

      unsavedDialogOpen = false;
      pendingCloseAction = null;

      if (action.type === 'manual-save') {
        return;
      } else if (action.type === 'single-tab') {
        closeTab(action.paths[0]);
      } else if (action.type === 'single-split-tab') {
        closeSplitTab(action.paths[0]);
      } else if (action.type === 'multiple-tabs') {
        for (const p of action.paths) {
          closeTab(p);
        }
      } else if (action.type === 'window-close') {
        await flushAllModifiedTabs();
        if (isTauriEnvironment()) {
          try {
            const appWindow = getCurrentWebviewWindow();
            if (appWindow.label === 'main') {
              const allWindows = await getAllWebviewWindows();
              for (const win of allWindows) {
                if (win.label !== 'main') {
                  try {
                    await win.destroy();
                  } catch {}
                }
              }
            }
            await appWindow.destroy();
          } catch {}
        }
      }
    } finally {
      isForceSavingUnsaved = false;
    }
  }

  async function handleOpenInNewWindow(path: string, moveTab = false) {
    if (!path || isSyntheticTab(path)) return;
    const vItem = vaultItems.find((v) => v.relative_path === path);
    const title = vItem?.title || path.split('/').pop() || path;
    await systemApi.openTabInNewWindow(path, title);
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

  function switchToNextTab() {
    if (activeFocusedPane === 'right' && isSplit && splitTabPaths.length > 0) {
      if (splitTabPaths.length < 2) return;
      const currentIndex = activeSplitTabPath ? splitTabPaths.indexOf(activeSplitTabPath) : -1;
      const nextIndex = (currentIndex + 1) % splitTabPaths.length;
      selectSplitTab(splitTabPaths[nextIndex]);
    } else if (openTabPaths.length > 0) {
      if (openTabPaths.length < 2) return;
      const currentIndex = activeTabPath ? openTabPaths.indexOf(activeTabPath) : -1;
      const nextIndex = (currentIndex + 1) % openTabPaths.length;
      selectTab(openTabPaths[nextIndex]);
    }
  }

  function switchToPrevTab() {
    if (activeFocusedPane === 'right' && isSplit && splitTabPaths.length > 0) {
      if (splitTabPaths.length < 2) return;
      const currentIndex = activeSplitTabPath ? splitTabPaths.indexOf(activeSplitTabPath) : 0;
      const prevIndex = (currentIndex - 1 + splitTabPaths.length) % splitTabPaths.length;
      selectSplitTab(splitTabPaths[prevIndex]);
    } else if (openTabPaths.length > 0) {
      if (openTabPaths.length < 2) return;
      const currentIndex = activeTabPath ? openTabPaths.indexOf(activeTabPath) : 0;
      const prevIndex = (currentIndex - 1 + openTabPaths.length) % openTabPaths.length;
      selectTab(openTabPaths[prevIndex]);
    }
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

  function selectSplitTab(path: string, viewerPreference?: ExcalidrawViewerType) {
    if (!path) return;
    if (isOfficeFile(path)) {
      systemApi.openFileWithDefaultApp(path).catch((err) => {
        console.warn('Error al abrir archivo con la aplicación por defecto del sistema:', err);
      });
      return;
    }
    const previousSplitPath = activeSplitTabPath;
    if (previousSplitPath && previousSplitPath !== path) {
      flushPendingSaveForPath(previousSplitPath);
    }
    if (!splitTabPaths.includes(path)) {
      splitTabPaths = [...splitTabPaths, path];
    }
    activeSplitTabPath = path;
    if (!isSyntheticTab(path)) {
      ensureContentLoaded(path);
      const vItem = vaultItems.find((v) => v.relative_path === path);
      const isLocked = lockedFiles.has(path);
      const isDrawing = isDrawingTab(path, openedNotes[path]?.content);
      const existingMode = openedNotes[path]?.viewMode;
      const isAlreadyLoaded = openedNotes[path]?.isLoaded === true;
      const hasExplicitEditSession = isAlreadyLoaded && (existingMode === 'live' || existingMode === 'source');
      const validMemoryMode = vItem?.view_mode && ['live', 'source', 'reading'].includes(vItem.view_mode) ? (vItem.view_mode as MarkdownViewMode) : undefined;
      const mode: MarkdownViewMode = (isLocked || isDrawing)
        ? (hasExplicitEditSession ? (existingMode as MarkdownViewMode) : "reading")
        : (existingMode || validMemoryMode || "reading");
      splitMarkdownViewMode = mode;
      isSplitEditing = mode !== "reading";
      if (viewerPreference) {
        if (!openedNotes[path]) {
          openedNotes[path] = {
            relative_path: path,
            abs_path: vItem?.abs_path,
            title: vItem?.title || path.split("/").pop() || path,
            content: "",
            savedContent: "",
            encoding: "---",
            isLoading: true,
            isLoaded: false,
            viewMode: mode,
            excalidrawViewer: viewerPreference,
          };
        } else {
          openedNotes[path].excalidrawViewer = viewerPreference;
        }
      }
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
            const vItem = vaultItems.find((v) => v.relative_path === activeSplitTabPath);
            const isLocked = lockedFiles.has(activeSplitTabPath);
            const isDrawing = isDrawingTab(activeSplitTabPath, openedNotes[activeSplitTabPath]?.content);
            const nextMode = ((isLocked || isDrawing) ? "reading" : (openedNotes[activeSplitTabPath]?.viewMode || vItem?.view_mode || "reading")) as MarkdownViewMode;
            splitMarkdownViewMode = nextMode;
            isSplitEditing = nextMode !== "reading";
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
      const vItem = vaultItems.find((v) => v.relative_path === path);
      const isLocked = lockedFiles.has(path);
      const isDrawing = isDrawingTab(path, openedNotes[path]?.content);
      const existingMode = openedNotes[path]?.viewMode;
      const isAlreadyLoaded = openedNotes[path]?.isLoaded === true;
      const hasExplicitEditSession = isAlreadyLoaded && (existingMode === 'live' || existingMode === 'source');
      const validMemoryMode = vItem?.view_mode && ['live', 'source', 'reading'].includes(vItem.view_mode) ? (vItem.view_mode as MarkdownViewMode) : undefined;
      const mode: MarkdownViewMode = (isLocked || isDrawing)
        ? (hasExplicitEditSession ? (existingMode as MarkdownViewMode) : "reading")
        : (existingMode || validMemoryMode || "reading");
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

  function handleSelectTabFromExplorer(path: string, viewerPreference?: ExcalidrawViewerType) {
    if (activeFocusedPane === 'right' && isSplit) {
      selectSplitTab(path, viewerPreference);
    } else {
      selectTab(path, viewerPreference);
    }
  }

  function handleChangeSplitMarkdownView(newMode: MarkdownViewMode) {
    splitMarkdownViewMode = newMode;
    isSplitEditing = newMode !== "reading";
    if (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)) {
      if (openedNotes[activeSplitTabPath]) {
        openedNotes[activeSplitTabPath].viewMode = newMode;
      }
      const vItem = vaultItems.find((v) => v.relative_path === activeSplitTabPath);
      if (vItem) vItem.view_mode = newMode;
      vaultRepository.setNoteViewMode(activeSplitTabPath, newMode);
      persistTabsState();
    }
  }

  function toggleSplitViewMode() {
    if (!activeSplitTabPath || isSyntheticTab(activeSplitTabPath)) return;

    if (isMarkdownFile(activeSplitTabPath) && !isDrawingTab(activeSplitTabPath, openedNotes[activeSplitTabPath]?.content)) {
      if (!isSplitEditing) {
        handleChangeSplitMarkdownView(splitMarkdownViewMode === "reading" ? "live" : splitMarkdownViewMode);
      } else {
        handleChangeSplitMarkdownView("reading");
      }
    } else {
      const nextEditing = !isSplitEditing;
      isSplitEditing = nextEditing;
      const nextMode: MarkdownViewMode = nextEditing ? "live" : "reading";
      splitMarkdownViewMode = nextMode;
      if (openedNotes[activeSplitTabPath]) {
        openedNotes[activeSplitTabPath].viewMode = nextMode;
      }
      const vItem = vaultItems.find((v) => v.relative_path === activeSplitTabPath);
      if (vItem) vItem.view_mode = nextMode;
      vaultRepository.setNoteViewMode(activeSplitTabPath, nextMode);
      persistTabsState();
    }
  }

  function handleSplitResizeStart(e: PointerEvent) {
    layoutState.handleSplitResizeStart(e);
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
  let markdownViewers = $state<Record<string, any>>({});
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

  let currentActiveNonGraphPath = $derived(
    activeTabPath && !isSyntheticTab(activeTabPath)
      ? activeTabPath
      : (activeSplitTabPath && !isSyntheticTab(activeSplitTabPath)
          ? activeSplitTabPath
          : lastActiveDocumentPath)
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
      !isDrawingTab(currentFocusedTabPath, focusedContent) &&
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
      !isDrawingTab(activeSplitTabPath, openedNotes[activeSplitTabPath]?.content) &&
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
      !activeNote.isLoading
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
      !openedNotes[activeSplitTabPath]?.isLoading
    )
  );

  let tabsInfo = $derived(
    openTabPaths.map((path) => {
      if (isSyntheticTab(path)) {
        return {
          path,
          title: getTabDisplayTitle(path, "Pestaña"),
          abs_path: undefined,
          isDirty: false,
        };
      }
      const vaultItem = vaultItems.find((item) => item.relative_path === path);
      const isDirty = isTabDirty(path);
      const isLocked = lockedFiles.has(path);
      const note = openedNotes[path];
      const isDrawing = isDrawingTab(path, note?.content);
      const rawTitle = note?.title || vaultItem?.title || path;
      const displayTitle = cleanTabTitle(rawTitle, path);
      return {
        path,
        abs_path: note?.abs_path || vaultItem?.abs_path,
        title: displayTitle,
        isDirty,
        isLocked,
        kind: isDrawing ? ('excalidraw' as const) : undefined,
      };
    })
  );

  let rightTabsInfo = $derived(
    splitTabPaths.map((path) => {
      if (isSyntheticTab(path)) {
        return {
          path,
          title: getTabDisplayTitle(path, "Pestaña"),
          abs_path: undefined,
          isDirty: false,
        };
      }
      const vaultItem = vaultItems.find((item) => item.relative_path === path);
      const isDirty = isTabDirty(path);
      const isLocked = lockedFiles.has(path);
      const note = openedNotes[path];
      const isDrawing = isDrawingTab(path, note?.content);
      const rawTitle = note?.title || vaultItem?.title || path;
      const displayTitle = cleanTabTitle(rawTitle, path);
      return {
        path,
        abs_path: note?.abs_path || vaultItem?.abs_path,
        title: displayTitle,
        isDirty,
        isLocked,
        kind: isDrawing ? ('excalidraw' as const) : undefined,
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
      const activePath = await vaultApi.getActiveVaultPath();
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
      ensureSynapseAppLoaded();
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
            view_mode: n.view_mode,
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

      if (lockedFiles.has(relativePath)) {
        const next = new Set(lockedFiles);
        next.delete(relativePath);
        unsavedChanges.setLockedFiles(Array.from(next));
        if (!isStandaloneMode) {
          vaultRepository.saveVaultUiState(undefined, undefined, undefined, undefined, Array.from(next));
        }
      }

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

      let changedLocks = false;
      const nextLocks = new Set(lockedFiles);
      for (const item of items) {
        if (nextLocks.has(item.relativePath)) {
          nextLocks.delete(item.relativePath);
          changedLocks = true;
        }
      }
      if (changedLocks) {
        unsavedChanges.setLockedFiles(Array.from(nextLocks));
        if (!isStandaloneMode) {
          vaultRepository.saveVaultUiState(undefined, undefined, undefined, undefined, Array.from(nextLocks));
        }
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
    tabHistory.replaceTab(relativePath, renamedPath);
    recentFiles = recentFiles.map((path) => path === relativePath ? renamedPath : path);
    if (lastClosedTabIndex[relativePath] !== undefined) {
      lastClosedTabIndex[renamedPath] = lastClosedTabIndex[relativePath];
      delete lastClosedTabIndex[relativePath];
    }

    if (lockedFiles.has(relativePath)) {
      const next = new Set(lockedFiles);
      next.delete(relativePath);
      next.add(renamedPath);
      unsavedChanges.setLockedFiles(Array.from(next));
      if (!isStandaloneMode) {
        vaultRepository.saveVaultUiState(undefined, undefined, undefined, undefined, Array.from(next));
      }
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

    systemApi.getStartupTarget().then(async (startupTarget) => {
      if (startupTarget?.type === 'standalone') {
        await openStandaloneFile(startupTarget.path);
        return;
      } else if (startupTarget?.type === 'vault') {
        showWelcomeScreen = false;
        hasActiveWorkspaceLoaded = true;
        await vaultApi.setActiveVaultPath(startupTarget.path);
        appSettings.setLastOpenedFolder(startupTarget.path);
        await fetchNotesFromBackend();
        return;
      }

      systemApi.getDetachedWindowTab().then(async (detachedTab) => {
        if (detachedTab) {
          isDetachedWindow = true;
          showWelcomeScreen = false;
          hasActiveWorkspaceLoaded = true;
          openTabPaths = [detachedTab];
          activeTabPath = detachedTab;
          await restoreSidebarWidth();
          await fetchNotesFromBackend();
          ensureContentLoaded(detachedTab);
          return;
        }

        // Verificar si se abrió una ventana nueva con parámetro ?vault=...
        const urlParams = new URLSearchParams(window.location.search);
        const vaultQueryParam = urlParams.get('vault');
        if (vaultQueryParam) {
          showWelcomeScreen = false;
          hasActiveWorkspaceLoaded = true;
          await vaultApi.setActiveVaultPath(vaultQueryParam);
          appSettings.setLastOpenedFolder(vaultQueryParam);
          await fetchNotesFromBackend();
          return;
        }

        const activePath = await vaultApi.getActiveVaultPath();
        if (!activePath) {
          if (openTabPaths.length === 0) {
            handleNewEmptyTab();
          }
          showWelcomeScreen = true;
          hasActiveWorkspaceLoaded = false;
        } else {
          showWelcomeScreen = false;
          hasActiveWorkspaceLoaded = true;
          await fetchNotesFromBackend();
        }
      });
    });

    let unlistenFsChange: (() => void) | undefined;
    let unlistenTabMoved: (() => void) | undefined;
    let unlistenCloseRequested: (() => void) | undefined;
    if (isTauriEnvironment()) {
      try {
        const appWindow = getCurrentWebviewWindow();
        currentWindowLabel = appWindow.label;

        // Interceptar el cierre de ventana para persistir cambios pendientes o advertir si hay archivos bloqueados sucios
        appWindow.onCloseRequested(async (event) => {
          const allTabs = Array.from(new Set([...openTabPaths, ...splitTabPaths]));
          const dirtyLocked = allTabs.filter((p) => lockedFiles.has(p) && isTabDirty(p));
          if (dirtyLocked.length > 0) {
            event.preventDefault();
            pendingCloseAction = { type: 'window-close', paths: allTabs };
            unsavedDialogFiles = dirtyLocked;
            unsavedDialogIsWindowClose = true;
            unsavedDialogOpen = true;
            return;
          }
          await flushAllModifiedTabs();

          // Si la ventana que se está cerrando es la ventana principal, cerrar también todas las ventanas secundarias desacopladas
          if (appWindow.label === 'main') {
            try {
              const allWindows = await getAllWebviewWindows();
              for (const win of allWindows) {
                if (win.label !== 'main') {
                  try {
                    await win.destroy();
                  } catch {}
                }
              }
            } catch (err) {
              console.warn('Error cerrando ventanas secundarias:', err);
            }
          }
        }).then((unsub) => {
          unlistenCloseRequested = unsub;
        }).catch((e) => {
          console.warn('Error al registrar onCloseRequested:', e);
        });
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
          const allOpen = Array.from(new Set([...openTabPaths, ...splitTabPaths]));
          const matchedOpenPath = allOpen.find((p) => p.replace(/^\.\//, '').replace(/\\/g, '/') === cleanChanged);

          if (matchedOpenPath) {
            // Ignorar eventos generados por nuestro propio guardado reciente (evita bucles reactivos y re-renders)
            const lastSave = recentSaveTimestamps[matchedOpenPath] || recentSaveTimestamps[changedPath] || 0;
            if (Date.now() - lastSave < 2500) {
              continue;
            }

            // Si es un dibujo abierto y el usuario está editando activamente, no sobreescribir con datos de disco
            if (isDrawingTab(matchedOpenPath, openedNotes[matchedOpenPath]?.content)) {
              const isCurrentlyEditing = (matchedOpenPath === activeTabPath && isEditing) || (matchedOpenPath === activeSplitTabPath && isSplitEditing);
              if (isCurrentlyEditing) {
                continue;
              }
            }

            const note = openedNotes[matchedOpenPath];
            const isClean = !note || note.content === note.savedContent;
            if (isClean) {
              try {
                if (isPdfFile(matchedOpenPath)) {
                  openedNotes[matchedOpenPath] = {
                    ...openedNotes[matchedOpenPath],
                    lastReloaded: Date.now(),
                  };
                  if (pdfViewers[matchedOpenPath]?.reload) {
                    pdfViewers[matchedOpenPath].reload();
                  }
                } else if (isImageFile(matchedOpenPath) && !isDrawingFile(matchedOpenPath)) {
                  openedNotes[matchedOpenPath] = {
                    ...openedNotes[matchedOpenPath],
                    lastReloaded: Date.now(),
                  };
                } else {
                  const noteData = await vaultRepository.readNote(matchedOpenPath);
                  if (noteData && (noteData.content ?? "") !== (openedNotes[matchedOpenPath]?.savedContent ?? null)) {
                    const currentMode = openedNotes[matchedOpenPath]?.viewMode || "reading";
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
                      lastReloaded: Date.now(),
                    };
                  }
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

    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      const allTabs = Array.from(new Set([...openTabPaths, ...splitTabPaths]));
      const dirtyLocked = allTabs.filter((p) => lockedFiles.has(p) && isTabDirty(p));
      if (dirtyLocked.length > 0) {
        e.preventDefault();
        e.returnValue = '';
        return '';
      }
      // Intentar guardar cualquier pestaña pendiente si se descarga la página
      flushAllModifiedTabs();
    };
    window.addEventListener('beforeunload', handleBeforeUnload);

    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
      if (unlistenCloseRequested) {
        unlistenCloseRequested();
      }
      if (unlistenFsChange) {
        unlistenFsChange();
      }
      if (unlistenTabMoved) {
        unlistenTabMoved();
      }
    };
  });

  // Persiste inmediatamente todas las notas y lienzos con modificaciones pendientes
  async function flushAllModifiedTabs() {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    const allTabs = Array.from(new Set([...openTabPaths, ...splitTabPaths]));
    for (const p of allTabs) {
      if (!p || isSyntheticTab(p) || lockedFiles.has(p)) continue;

      if (isDrawingTab(p, openedNotes[p]?.content) && excalidrawViewers[p]?.getCurrentSerializedScene) {
        try {
          const fresh = excalidrawViewers[p].getCurrentSerializedScene();
          if (fresh && openedNotes[p]) {
            openedNotes[p].content = fresh;
          }
        } catch (err) {
          console.warn(`Error al extraer escena Excalidraw durante flush en ${p}:`, err);
        }
      }

      const note = openedNotes[p];
      if (note && note.isLoaded && note.content !== note.savedContent) {
        const vItem = vaultItems.find((v) => v.relative_path === p) || {
          id: "0",
          title: note.title || p,
          relative_path: p,
          abs_path: note.abs_path,
        };
        await persistVaultItemToRust(vItem);
      }
    }
  }

  // Estado de Auto-Guardado (inicializado desde config.jsonc)
  let autoSave = $state(DEFAULT_APP_CONFIG.editor?.auto_save ?? true);
  let autoSaveDelayMs = DEFAULT_APP_CONFIG.editor?.auto_save_delay_ms ?? 600;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  function debouncedPersistVaultItemToRust(vaultItem: VaultItem, delay?: number) {
    if (!autoSave) return;
    if (unsavedChanges.isLocked(vaultItem.relative_path)) return;
    const isDrawing = isDrawingTab(vaultItem.relative_path, openedNotes[vaultItem.relative_path]?.content);
    const computedDelay = delay !== undefined ? delay : (isDrawing ? Math.max(autoSaveDelayMs, 1000) : autoSaveDelayMs);
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      persistVaultItemToRust(vaultItem);
    }, computedDelay);
  }

  function requestManualSave(vaultItem: VaultItem) {
    const path = vaultItem.relative_path;
    if (!path || isSyntheticTab(path)) return;

    if (isDrawingTab(path, openedNotes[path]?.content) && excalidrawViewers[path]?.getCurrentSerializedScene) {
      try {
        const freshContent = excalidrawViewers[path].getCurrentSerializedScene();
        if (freshContent && openedNotes[path]) {
          openedNotes[path].content = freshContent;
        }
      } catch (err) {
        console.warn("Error al extraer escena actual de Excalidraw:", err);
      }
    }

    if (unsavedChanges.isLocked(path)) {
      pendingCloseAction = { type: 'manual-save', paths: [path] };
      unsavedDialogFiles = [path];
      unsavedDialogIsWindowClose = false;
      unsavedDialogOpen = true;
      return;
    }

    persistVaultItemToRust(vaultItem, undefined, true);
  }

  async function persistVaultItemToRust(vaultItem: VaultItem, targetEncoding?: string, force = false) {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }
    const path = vaultItem.relative_path;
    const title = vaultItem.title || path?.split('/').pop() || path;
    if ((!isConnectedToRust && !vaultRepository.isConnected()) || !path) return;

    // Si el archivo está bloqueado (locked), abortar guardado salvo que se fuerce explícitamente (ej: diálogo de confirmación)
    if (!force && unsavedChanges.isLocked(path)) {
      return;
    }

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
    if (isDrawingTab(path, openedNotes[path]?.content) && excalidrawViewers[path]?.getCurrentSerializedScene) {
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

    // Protección estricta contra sobreescritura accidental: si el archivo aún no ha terminado de cargarse y su contenido está vacío, abortar guardado
    if (note && !note.isLoaded && (note.content === "" || !note.content)) {
      console.warn(`[Protección] Se abortó guardado de ${path} porque aún no ha sido cargado desde disco.`);
      return;
    }

    const contentToSave = note ? note.content : "";
    const currentTabEnc = note ? note.encoding : "---";
    const enc = targetEncoding || (currentTabEnc && currentTabEnc !== "---" ? currentTabEnc : "UTF-8");

    // Registrar timestamp antes de iniciar el guardado para evitar condiciones de carrera con notify / fs-events
    const cleanP = path.replace(/^\.\//, '').replace(/\\/g, '/');
    const now = Date.now();
    recentSaveTimestamps[path] = now;
    recentSaveTimestamps[cleanP] = now;
    syncState = "saving";
    try {
      await vaultRepository.saveNote({
        relativePath: path,
        title: title,
        content: contentToSave,
        encoding: enc,
      });
      const postNow = Date.now();
      recentSaveTimestamps[path] = postNow;
      recentSaveTimestamps[cleanP] = postNow;
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

  async function createNewVaultItem(
    emptyTabPathToReplace?: string,
    fileType: 'markdown' | 'mermaid' | 'excalidraw' | 'obsidian-excalidraw' = 'markdown'
  ) {
    let extension = 'md';
    let defaultBaseTitle = 'Nuevo Archivo';
    let initialContent = '';

    if (fileType === 'mermaid') {
      extension = 'mmd';
      defaultBaseTitle = 'Nuevo Diagrama';
      initialContent = 'graph TD\n  A[Inicio] --> B[Fin]\n';
    } else if (fileType === 'excalidraw') {
      extension = 'excalidraw';
      defaultBaseTitle = 'Nuevo Dibujo';
      initialContent = JSON.stringify(
        {
          type: 'excalidraw',
          version: 2,
          source: 'synapse',
          elements: [],
          appState: {},
          files: {},
        },
        null,
        2
      );
    } else if (fileType === 'obsidian-excalidraw') {
      extension = 'excalidraw.md';
      defaultBaseTitle = 'Nuevo Dibujo';

      // Consultar si el plugin obsidian-excalidraw-plugin tiene plantilla configurada
      const plugin = synapseApp?.plugins.loadedPlugins.get('obsidian-excalidraw-plugin')?.instance as any;
      let templateContent: string | undefined;

      if (plugin?.settings?.templateFilePath) {
        try {
          const tNote = await vaultRepository.readNote(plugin.settings.templateFilePath);
          if (tNote && typeof tNote.content === 'string' && tNote.content.trim().length > 0) {
            templateContent = tNote.content;
          }
        } catch {}
      }

      initialContent = createCanonicalExcalidrawContent(templateContent);
    }

    let counter = vaultItems.length + 1;
    let newTitle = `${defaultBaseTitle} ${counter}`;
    let newRelPath = `${newTitle}.${extension}`;

    while (vaultItems.some((v) => v.relative_path === newRelPath)) {
      counter++;
      newTitle = `${defaultBaseTitle} ${counter}`;
      newRelPath = `${newTitle}.${extension}`;
    }

    const newVaultItem: VaultItem = {
      id: String(counter),
      title: newTitle,
      relative_path: newRelPath,
      abs_path: undefined,
    };
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
      viewMode: "live",
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
      tabHistory.replaceTab(targetEmptyPath, newRelPath);
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

  function registerWorkspaceCommands() {
    commandRegistry.registerMany([
      {
        id: "cmd-split-view",
        name: "Dividir panel verticalmente (Split View)",
        category: "Vista",
        shortcut: shortcutsConfig.workspace.split_view,
        action: () => handleSplitView(),
      },
      {
        id: "cmd-new-tab",
        name: "Nueva pestaña vacía",
        category: "Pestañas",
        shortcut: shortcutsConfig.workspace.new_tab,
        action: handleNewEmptyTab,
      },
      {
        id: "cmd-next-tab",
        name: "Pestaña siguiente",
        category: "Pestañas",
        shortcut: shortcutsConfig.workspace.next_tab,
        action: switchToNextTab,
      },
      {
        id: "cmd-prev-tab",
        name: "Pestaña anterior",
        category: "Pestañas",
        shortcut: shortcutsConfig.workspace.prev_tab,
        action: switchToPrevTab,
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
        shortcut: shortcutsConfig.workspace.open_vault_folder,
        action: handleOpenVaultFolder,
      },
      {
        id: "cmd-welcome-screen",
        name: "Workspaces Recientes / Pantalla de Bienvenida",
        category: "Bóveda",
        action: () => {
          showWelcomeScreen = true;
        },
      },
      {
        id: "cmd-community-plugins",
        name: "Plugins de la comunidad: Explorar e instalar plugins de Obsidian",
        category: "Plugins",
        action: () => {
          isPluginsModalOpen = true;
        },
      },
      {
        id: "cmd-new-file",
        name: "Crear nuevo archivo Markdown",
        category: "Archivo",
        shortcut: shortcutsConfig.workspace.new_file,
        action: () => {
          createNewVaultItem();
        },
      },
      {
        id: "cmd-new-mermaid",
        name: "Crear nuevo diagrama Mermaid (.mmd)",
        category: "Archivo",
        action: () => {
          createNewVaultItem(undefined, 'mermaid');
        },
      },
      {
        id: "cmd-new-excalidraw",
        name: "Crear nuevo dibujo Excalidraw (.excalidraw)",
        category: "Archivo",
        action: () => {
          createNewVaultItem(undefined, 'excalidraw');
        },
      },
      {
        id: "cmd-new-obsidian-excalidraw",
        name: "Excalidraw: Crear nuevo dibujo (.excalidraw.md)",
        category: "Excalidraw",
        action: () => {
          createNewVaultItem(undefined, 'obsidian-excalidraw');
        },
      },
      {
        id: "cmd-close-tab",
        name: "Cerrar pestaña actual",
        category: "Pestañas",
        shortcut: shortcutsConfig.workspace.close_tab,
        action: () => {
          if (activeTabPath) requestCloseTab(activeTabPath);
        },
      },
      {
        id: "cmd-close-all-tabs",
        name: "Cerrar todas las pestañas",
        category: "Pestañas",
        action: requestCloseAllTabs,
      },
      {
        id: "cmd-toggle-vim",
        name: "Alternar modo Vim",
        category: "Editor",
        action: () => {
          appSettings.toggleVimMode();
        },
      },
      {
        id: "cmd-toggle-markdown-view",
        name: "Alternar vista Markdown (En vivo / Fuente / Lectura)",
        category: "Vista",
        action: toggleMarkdownViewMode,
      },
      {
        id: "cmd-toggle-edit-mode",
        name: "Alternar modo Edición / Lectura",
        category: "Vista",
        shortcut: shortcutsConfig.workspace.toggle_edit_mode,
        action: () => {
          if (activeFocusedPane === 'right' && isSplit && activeSplitTabPath) {
            toggleSplitViewMode();
          } else {
            toggleViewMode();
          }
        },
      },
      {
        id: "cmd-toggle-sidebar",
        name: "Mostrar u ocultar explorador de archivos",
        category: "Vista",
        shortcut: shortcutsConfig.workspace.toggle_sidebar,
        action: toggleExplorer,
      },
      {
        id: "cmd-switch-language",
        name: t('commands.switchLanguage'),
        category: t('commands.categories.settings'),
        action: () => {
          appSettings.toggleLocale();
        },
      },
      {
        id: "cmd-toggle-mermaid-engine",
        name: t('commands.toggleMermaidEngine'),
        category: t('commands.categories.settings'),
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
        id: "cmd-toggle-default-excalidraw-viewer",
        name: "Alternar visor por defecto de Excalidraw (Nativo / Obsidian)",
        category: "Configuración",
        action: () => {
          appSettings.toggleDefaultExcalidrawViewer();
        },
      },
      {
        id: "cmd-set-default-excalidraw-native",
        name: "Configurar visor por defecto de Excalidraw: Usar Excalidraw nativo",
        category: "Configuración",
        action: () => {
          appSettings.setDefaultExcalidrawViewer("native");
        },
      },
      {
        id: "cmd-set-default-excalidraw-obsidian",
        name: "Configurar visor por defecto de Excalidraw: Usar plugin de Excalidraw para Obsidian",
        category: "Configuración",
        action: () => {
          appSettings.setDefaultExcalidrawViewer("obsidian");
        },
      },
      {
        id: "cmd-switch-current-excalidraw-viewer",
        name: "Excalidraw: Alternar visor de la pestaña actual (Nativo / Obsidian)",
        category: "Excalidraw",
        action: () => {
          const targetPath = activeFocusedPane === 'right' && isSplit ? activeSplitTabPath : activeTabPath;
          if (targetPath && isDrawingTab(targetPath, openedNotes[targetPath]?.content)) {
            const current = getExcalidrawViewerForTab(targetPath);
            switchExcalidrawViewerForTab(targetPath, current === 'native' ? 'obsidian' : 'native');
          }
        },
      },
      {
        id: "cmd-save-file",
        name: "Guardar / Grabar archivo actual",
        category: "Archivo",
        shortcut: shortcutsConfig.workspace.save_file,
        action: () => {
          if (currentFocusedTabPath && currentFocusedVaultItem.relative_path) {
            requestManualSave(currentFocusedVaultItem);
          }
        },
      },
      {
        id: "cmd-toggle-devtools",
        name: "Alternar herramientas de desarrollo (DevTools)",
        category: "Desarrollo",
        shortcut: shortcutsConfig.workspace.toggle_devtools,
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
            await gitApi.add([activeTabPath]);
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
            await gitApi.restoreStaged([activeTabPath]);
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
            await gitApi.restore([activeTabPath]);
            await handleGitRestoreFiles([activeTabPath]);
            if (vaultExplorerRef?.refreshTree) {
              await vaultExplorerRef.refreshTree();
            }
          } catch (e) {
            console.error("Error en git restore:", e);
          }
        },
      },
      {
        id: "cmd-reload-active-file",
        name: "Archivo: Recargar archivo actual desde el disco",
        category: "Archivo",
        shortcut: shortcutsConfig.workspace.reload_active_file,
        action: async () => {
          if (!activeTabPath || isSyntheticTab(activeTabPath)) return;
          await handleReloadItems([activeTabPath]);
        },
      },
      {
        id: "cmd-reload-vault",
        name: "Bóveda: Recargar toda la bóveda y refrescar metadatos",
        category: "Bóveda",
        shortcut: shortcutsConfig.workspace.reload_vault,
        action: async () => {
          await handleReloadItems(['']);
        },
      },
      {
        id: "cmd-search-fulltext",
        name: "Buscar en el contenido de las notas (FTS)",
        category: "Búsqueda",
        shortcut: shortcutsConfig.workspace.search_fulltext,
        action: () => {
          openFullTextSearchModal();
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
        shortcut: shortcutsConfig.workspace.locate_active_file,
        action: async () => {
          if (activeRibbonTab !== 'files') {
            if (sidebarWidth < 140) {
              layoutState.setSidebarWidth(240);
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
        shortcut: shortcutsConfig.workspace.nav_back,
        action: navigateBack,
      },
      {
        id: "cmd-nav-forward",
        name: "Navegar adelante entre pestañas",
        category: "Navegación",
        shortcut: shortcutsConfig.workspace.nav_forward,
        action: navigateForward,
      },
      {
        id: "cmd-toggle-theme",
        name: "Alternar tema (Claro / Oscuro)",
        category: "Apariencia",
        shortcut: shortcutsConfig.workspace.toggle_theme,
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
      {
        id: "cmd-toggle-mcp-server",
        name: "MCP: Alternar servidor MCP local (Activar / Desactivar)",
        category: "MCP",
        action: handleToggleMcp,
      },
      {
        id: "cmd-copy-mcp-endpoint",
        name: "MCP: Copiar URL del servidor MCP al portapapeles",
        category: "MCP",
        action: copyMcpUrlToClipboard,
      },
    ]);
  }

  // Re-registrar comandos al cambiar idioma reactivamente
  $effect(() => {
    const _loc = appSettings.locale;
    registerWorkspaceCommands();
  });

  // Registrar comandos por defecto y cargar atajos JSONC al iniciar
  onMount(() => {
    registerWorkspaceCommands();
    shortcutsConfig.loadFromBackend().then(() => {
      registerWorkspaceCommands();
    });

    // Escuchar cambios de estado en el servidor MCP y restaurar preferencia
    refreshMcpStatus();
    if (appSettings.mcpServerEnabled) {
      systemApi.startMcpServer().then((s) => {
        if (s) mcpStatus = s;
      });
    }

    let unlistenMcp: (() => void) | undefined;
    if (isTauriEnvironment()) {
      listen<McpRuntimeStatus>('mcp:status-changed', (event) => {
        mcpStatus = event.payload;
      }).then((u) => {
        unlistenMcp = u;
      }).catch((e) => {
        console.warn('Error escuchando eventos de estado MCP:', e);
      });
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (showWelcomeScreen) {
        return;
      }
      const ws = shortcutsConfig.workspace;
      const stopEvent = () => {
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
      };

      if (matchesShortcut(e, ws.toggle_devtools)) {
        stopEvent();
        toggleDevtools();
        return;
      }
      if (matchesShortcut(e, ws.locate_active_file)) {
        stopEvent();
        if (activeRibbonTab !== 'files') {
          if (sidebarWidth < 140) {
            layoutState.setSidebarWidth(240);
          }
          activeRibbonTab = 'files';
        }
        tick().then(() => {
          vaultExplorerRef?.locateActiveFile();
        });
        return;
      }
      if (matchesShortcut(e, ws.toggle_sidebar)) {
        stopEvent();
        toggleExplorer();
        return;
      }
      if (matchesShortcut(e, ws.nav_back)) {
        stopEvent();
        navigateBack();
        return;
      }
      if (matchesShortcut(e, ws.nav_forward)) {
        stopEvent();
        navigateForward();
        return;
      }
      if (matchesShortcut(e, ws.open_vault_folder)) {
        stopEvent();
        handleOpenVaultFolder();
        return;
      }
      if (matchesShortcut(e, ws.toggle_theme)) {
        stopEvent();
        appSettings.toggleTheme();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
        const targetPath = activeFocusedPane === 'right' && isSplit ? activeSplitTabPath : activeTabPath;
        if (targetPath && markdownViewers[targetPath]) {
          stopEvent();
          markdownViewers[targetPath].triggerSearch();
          return;
        }
      }
      if (matchesShortcut(e, ws.search_fulltext)) {
        stopEvent();
        openFullTextSearchModal();
        return;
      }
      if (matchesShortcut(e, ws.reload_vault)) {
        stopEvent();
        handleReloadItems(['']);
        return;
      }
      if (matchesShortcut(e, ws.reload_active_file)) {
        stopEvent();
        if (activeTabPath && !isSyntheticTab(activeTabPath)) {
          handleReloadItems([activeTabPath]);
        }
        return;
      }
      if (matchesShortcut(e, ws.save_file)) {
        stopEvent();
        if (currentFocusedTabPath && currentFocusedVaultItem.relative_path) {
          requestManualSave(currentFocusedVaultItem);
        }
        return;
      }
      if (matchesShortcut(e, ws.new_file)) {
        info(`activeTabPath: ${activeTabPath}`);
        stopEvent();
        createNewVaultItem();
        return;
      }
      if (matchesShortcut(e, ws.new_tab)) {
        stopEvent();
        handleNewEmptyTab();
        return;
      }
      if (matchesShortcut(e, ws.next_tab)) {
        stopEvent();
        switchToNextTab();
        return;
      }
      if (matchesShortcut(e, ws.prev_tab)) {
        stopEvent();
        switchToPrevTab();
        return;
      }
      if (matchesShortcut(e, ws.split_view)) {
        stopEvent();
        handleSplitView();
        return;
      }
      if (matchesShortcut(e, ws.close_tab)) {
        stopEvent();
        if (activeFocusedPane === 'right' && isSplit && activeSplitTabPath) {
          requestCloseSplitTab(activeSplitTabPath);
        } else if (activeTabPath) {
          requestCloseTab(activeTabPath);
        }
        return;
      }
      if (matchesShortcut(e, ws.quick_open)) {
        stopEvent();
        openQuickOpenModal();
        return;
      }
      if (matchesShortcut(e, ws.command_palette)) {
        stopEvent();
        isQuickOpenOpen = false;
        isFullTextSearchOpen = false;
        isPaletteOpen = true;
        return;
      }
      if (matchesShortcut(e, ws.toggle_edit_mode)) {
        stopEvent();
        if (activeFocusedPane === 'right' && isSplit && activeSplitTabPath) {
          toggleSplitViewMode();
        } else {
          toggleViewMode();
        }
        return;
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
      if (unlistenMcp) unlistenMcp();
    };
  });

  function handleRibbonAction(actionId: string) {
    if (actionId === "command-palette") {
      isPaletteOpen = true;
    } else if (actionId === "plugins") {
      isPluginsModalOpen = true;
    } else if (actionId === "new-note") {
      createNewVaultItem();
    } else if (actionId === "new-excalidraw") {
      createNewVaultItem(undefined, 'obsidian-excalidraw');
    } else if (actionId === "quick-open") {
      openQuickOpenModal();
    } else if (actionId === "search") {
      openFullTextSearchModal();
    } else if (actionId === "graph") {
      const target = (activeTabPath && !isSyntheticTab(activeTabPath))
        ? activeTabPath
        : ((activeSplitTabPath && !isSyntheticTab(activeSplitTabPath))
            ? activeSplitTabPath
            : lastActiveDocumentPath);
      if (target) {
        selectTab(`synapse://graph:${target}`);
      } else {
        selectTab("synapse://graph");
      }
    } else if (actionId === "welcome") {
      showWelcomeScreen = true;
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

  async function handleWorkspaceSelectedFromWelcome(
    folderPath: string,
    initialNotes?: Array<{ relative_path: string; abs_path?: string; title: string }>
  ) {
    isStandaloneMode = false;
    showWelcomeScreen = false;
    hasActiveWorkspaceLoaded = true;
    appSettings.setLastOpenedFolder(folderPath);
    openTabPaths = [];
    activeTabPath = null;
    splitTabPaths = [];
    activeSplitTabPath = null;
    openedNotes = {};
    tabSelections = {};
    recentFiles = [];
    tabHistory.clear();

    if (initialNotes && initialNotes.length > 0) {
      vaultItems = initialNotes.map((n, index) => {
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
      isConnectedToRust = vaultRepository.isConnected();
      await refreshGitStatus();
      vaultRepository.getRecentNotes(15).then((recents) => {
        if (recents && recents.length > 0) {
          recentFiles = recents;
        }
      });
      await restoreSidebarWidth();
      const restored = await restoreOpenTabsState();
      if (!restored) {
        handleNewEmptyTab();
      }
    } else {
      await fetchNotesFromBackend();
    }
  }

  async function openStandaloneFile(filePath: string) {
    isStandaloneMode = true;
    showWelcomeScreen = false;
    hasActiveWorkspaceLoaded = true;
    openTabPaths = [filePath];
    activeTabPath = filePath;
    const fileName = filePath.split(/[\\/]/).pop() || filePath;
    vaultItems = [
      {
        id: filePath,
        title: fileName,
        relative_path: filePath,
        abs_path: filePath,
      },
    ];
    await ensureContentLoaded(filePath);
  }

  function handleStandaloneFileSelected(filePath: string) {
    openStandaloneFile(filePath);
  }

  async function handleOpenVaultFolder() {
    try {
      const result = await vaultApi.selectVaultFolder(appSettings.lastOpenedFolder);
      if (result) {
        isStandaloneMode = false;
        if (result.folder_path) {
          appSettings.setLastOpenedFolder(result.folder_path);
        }
        showWelcomeScreen = false;
        hasActiveWorkspaceLoaded = true;
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
            view_mode: n.view_mode,
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
        tabHistory.clear();
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
        layoutState.setSidebarWidth(240);
      }
      activeRibbonTab = 'files';
    }
  }

  async function ensureExplorerVisible() {
    if (activeRibbonTab !== 'files' || sidebarWidth < 140) {
      if (sidebarWidth < 140) {
        layoutState.setSidebarWidth(240);
      }
      activeRibbonTab = 'files';
      await tick();
    }
  }

  async function handleBreadcrumbNavigateFolder(folderPath: string) {
    if (isDetachedWindow) return;
    await ensureExplorerVisible();
    vaultExplorerRef?.locateActiveFile(folderPath);
  }

  function toggleSidebar() {
    toggleExplorer();
  }

  function handleSidebarResizeStart(e: MouseEvent | PointerEvent) {
    layoutState.handleSidebarResizeStart(
      e,
      () => { activeRibbonTab = ''; },
      () => { if (!activeRibbonTab) activeRibbonTab = 'files'; }
    );
  }
</script>

<svelte:window onblur={handleWindowBlur} />

<div class="workspace-layout">
  {#if showWelcomeScreen && !isDetachedWindow}
    <WelcomeScreen
      onWorkspaceOpened={handleWorkspaceSelectedFromWelcome}
      onStandaloneFileOpened={handleStandaloneFileSelected}
      canCloseWelcome={hasActiveWorkspaceLoaded}
      onCloseWelcome={() => {
        showWelcomeScreen = false;
      }}
    />
  {/if}
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
      vaultPath={isStandaloneMode ? undefined : appSettings.lastOpenedFolder}
      {gitStatuses}
      isGitRepo={isStandaloneMode ? false : isGitRepo}
      gitBranch={isStandaloneMode ? null : gitBranch}
      isStandalone={isStandaloneMode}
      {lockedFiles}
      onToggleLockFile={handleToggleLockFile}
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
      onOpenRevision={(path, revision) => selectTab(`rev:${revision}:${path}`)}
      onOpenDiffWithRevision={(path, revision) => selectTab(`diff-rev:${revision}:${path}`)}
      onOpenInNewWindow={handleOpenInNewWindow}
    />

    {#if activeRibbonTab === 'tags'}
      <TagsExplorer
        {sidebarWidth}
        {isResizingSidebar}
        onSelectNote={(notePath) => selectTab(notePath)}
        onResizeStart={handleSidebarResizeStart}
        onCollapse={toggleSidebar}
      />
    {:else if activeRibbonTab === 'relations'}
      <aside
        class="relations-sidebar-wrapper"
        style="width: {sidebarWidth}px;"
      >
        <RelationsInspector
          activeNotePath={currentActiveNonGraphPath}
          onSelectNote={(notePath) => selectTab(notePath)}
        />
        <div
          role="separator"
          tabindex="-1"
          aria-label="Redimensionar inspector de relaciones"
          class="sidebar-resizer"
          class:is-resizing={isResizingSidebar}
          onpointerdown={handleSidebarResizeStart}
        ></div>
      </aside>
    {/if}
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
        {:else if tabPath.startsWith("diff-rev:")}
          {@const rest = tabPath.slice("diff-rev:".length)}
          {@const firstColonIdx = rest.indexOf(":")}
          {@const rev = firstColonIdx !== -1 ? rest.slice(0, firstColonIdx) : ""}
          {@const fPath = firstColonIdx !== -1 ? rest.slice(firstColonIdx + 1) : rest}
          <div
            class="tab-pane full-pane"
            class:hidden={tabPath !== activePath}
          >
            <DiffViewer
              filePath={fPath}
              isStaged={false}
              baseRevision={rev}
              onClose={() => (isRight ? requestCloseSplitTab(tabPath) : requestCloseTab(tabPath))}
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
              onClose={() => (isRight ? requestCloseSplitTab(tabPath) : requestCloseTab(tabPath))}
            />
          </div>
        {:else if tabPath.startsWith("rev:")}
          {@const rest = tabPath.slice("rev:".length)}
          {@const firstColonIdx = rest.indexOf(":")}
          {@const rev = firstColonIdx !== -1 ? rest.slice(0, firstColonIdx) : ""}
          {@const fPath = firstColonIdx !== -1 ? rest.slice(firstColonIdx + 1) : rest}
          <div
            class="tab-pane full-pane"
            class:hidden={tabPath !== activePath}
          >
            <RevisionViewer
              filePath={fPath}
              revision={rev}
              onClose={() => (isRight ? requestCloseSplitTab(tabPath) : requestCloseTab(tabPath))}
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
          {@const isDrawing = isDrawingTab(tabPath, content)}
          <div
            class="tab-pane"
            class:hidden={tabPath !== activePath}
            class:full-pane={isDiagramFile(tabPath) ||
              isImageFile(tabPath) ||
              isDrawing ||
              isPdfFile(tabPath) ||
              tabPath.startsWith('synapse://')}
          >
            {#if !tabPath.startsWith('synapse://')}
              <BreadCrumb
                path={isStandaloneMode ? (tabPath.split(/[\\/]/).pop() || tabPath) : tabPath}
                readOnly={isStandaloneMode}
                onNavigateFolder={handleBreadcrumbNavigateFolder}
                onRenameFile={(newName) => handleRenameItem(tabPath, newName)}
              />
            {/if}
            {#if isLoading}
              <div class="content-loading">
                <span class="spinner"></span>
                <span>Cargando contenido desde disco...</span>
              </div>
            {:else}
              {#key note?.lastReloaded}
                {#if isDiagramFile(tabPath)}
                  {#if isD2File(tabPath)}
                    <D2Viewer
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
                  {:else if appSettings.mermaidRenderer === 'mermaidjs'}
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
                {:else if isDrawing}
                  {#if getExcalidrawViewerForTab(tabPath) === 'obsidian'}
                    <ObsidianExcalidrawViewer
                      bind:this={excalidrawViewers[tabPath]}
                      {content}
                      filePath={tabPath}
                      readOnly={!paneEditing}
                      {synapseApp}
                      onSwitchToNative={() => switchExcalidrawViewerForTab(tabPath, 'native')}
                      onNavigate={handleNavigateLink}
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
                  {:else}
                    <ExcalidrawViewer
                      bind:this={excalidrawViewers[tabPath]}
                      {content}
                      filePath={tabPath}
                      readOnly={!paneEditing}
                      onSwitchToObsidian={() => switchExcalidrawViewerForTab(tabPath, 'obsidian')}
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
                  {/if}
                {:else if isMarkdownFile(tabPath)}
                  <input
                    type="text"
                    class="editor-title-input"
                    readonly={!paneEditing}
                    bind:value={vaultItem.title}
                    oninput={() => {
                      if (openedNotes[tabPath]) {
                        openedNotes[tabPath].title = vaultItem.title;
                      }
                      debouncedPersistVaultItemToRust(vaultItem);
                    }}
                    placeholder="Título del archivo..."
                  />

                  <div class="editor-main-content">
                    <MarkdownViewer
                      bind:this={markdownViewers[tabPath]}
                      {content}
                      filePath={tabPath}
                      readOnly={!paneEditing}
                      vimMode={isVimMode}
                      viewMode={paneViewMode}
                      isActive={tabPath === activePath}
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
                {:else if tabPath.startsWith("synapse://graph")}
                  {@const targetFocus = tabPath.slice("synapse://graph".length).replace(/^:/, "") || currentActiveNonGraphPath}
                  <GraphViewer
                    activeNotePath={targetFocus}
                    onSelectNote={(notePath: string) => selectTab(notePath)}
                  />
                {:else if isImageFile(tabPath)}
                  <ImageViewer
                    src={note?.abs_path || tabPath}
                    alt={vaultItem.title || tabPath}
                    {content}
                  />
                {:else if isPdfFile(tabPath)}
                  <PdfViewer
                    bind:this={pdfViewers[tabPath]}
                    src={tabPath}
                    reloadKey={note?.lastReloaded || 0}
                    onDirtyChange={(dirty) => {
                      if (dirty) {
                        syncState = "saving";
                      }
                    }}
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
      <WorkspacePane
        paneId="single"
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
        title={getTabDisplayTitle(activeTabPath, currentVaultItem.relative_path || currentVaultItem.title)}
        showSaveButton={(!autoSave || (!!activeTabPath && lockedFiles.has(activeTabPath))) &&
          isEditing &&
          !!activeTabPath &&
          !isSyntheticTab(activeTabPath) &&
          (!currentVaultItem.relative_path ||
            isMarkdownFile(currentVaultItem.relative_path) ||
            isDrawingTab(currentVaultItem.relative_path, activeContent) ||
            isPdfFile(currentVaultItem.relative_path))}
        showWindowControls={true}
        isSplitActive={false}
        bind:containerRef={editorContainerRef}
        showSplitDropZoneOverlay={isDragOverSplitRight}
        onSelectTab={(path) => selectTab(path)}
        onCloseTab={(path) => requestCloseTab(path)}
        onCloseAllTabs={requestCloseAllTabs}
        onCloseOtherTabs={requestCloseOtherTabs}
        onCloseTabsToRight={requestCloseTabsToRight}
        onOpenInNewWindow={handleOpenInNewWindow}
        onReorderTab={reorderTab}
        onReceiveTab={receiveTabFromOtherWindow}
        onNewTab={handleNewEmptyTab}
        onNewFile={() => createNewVaultItem()}
        onOpenQuickOpen={openQuickOpenModal}
        onSplitView={handleSplitView}
        onToggleView={toggleViewMode}
        onSave={() => {
          if (activeTabPath && currentVaultItem.relative_path) {
            requestManualSave(currentVaultItem);
          }
        }}
        onOpenCommandPalette={() => (isPaletteOpen = true)}
        onPointerDown={() => { activeFocusedPane = 'left'; }}
        onDragOver={handleEditorDragOver}
        onDragLeave={handleEditorDragLeave}
        onDrop={handleEditorDrop}
      >
        {@render renderPaneContent(openTabPaths, activeTabPath, false)}
      </WorkspacePane>
    {:else}
      <!-- MODO DIVIDIDO: DOS PANELES LADO A LADO -->
      <div class="split-workspace-container" bind:this={splitContainerRef}>
        <!-- PANEL IZQUIERDO -->
        <WorkspacePane
          paneId="left"
          isSplit={true}
          style="width: {splitRatio}%;"
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
          title={getTabDisplayTitle(activeTabPath, currentVaultItem.relative_path || currentVaultItem.title)}
          showSaveButton={(!autoSave || (!!activeTabPath && lockedFiles.has(activeTabPath))) &&
            isEditing &&
            !!activeTabPath &&
            !isSyntheticTab(activeTabPath) &&
            (!currentVaultItem.relative_path ||
              isMarkdownFile(currentVaultItem.relative_path) ||
              isDrawingTab(currentVaultItem.relative_path, activeContent) ||
              isPdfFile(currentVaultItem.relative_path))}
          showWindowControls={false}
          isSplitActive={true}
          onSelectTab={(path) => selectTab(path)}
          onCloseTab={(path) => requestCloseTab(path)}
          onCloseAllTabs={requestCloseAllTabs}
          onCloseOtherTabs={requestCloseOtherTabs}
          onCloseTabsToRight={requestCloseTabsToRight}
          onOpenInNewWindow={handleOpenInNewWindow}
          onReorderTab={reorderTab}
          onReceiveTab={(p, idx, src) => receiveTabFromSplitOrWindow(p, false, idx, src)}
          onNewTab={handleNewEmptyTab}
          onNewFile={() => createNewVaultItem()}
          onOpenQuickOpen={openQuickOpenModal}
          onSplitView={handleSplitView}
          onToggleView={toggleViewMode}
          onSave={() => {
            if (activeTabPath && currentVaultItem.relative_path) {
              requestManualSave(currentVaultItem);
            }
          }}
          onOpenCommandPalette={() => (isPaletteOpen = true)}
          onPointerDown={() => { activeFocusedPane = 'left'; }}
          onDragOver={(e) => handlePaneDragOver(e, 'left')}
          onDrop={(e) => handlePaneDrop(e, 'left')}
        >
          {@render renderPaneContent(openTabPaths, activeTabPath, false)}
        </WorkspacePane>

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
        <WorkspacePane
          paneId="right"
          isSplit={true}
          style="width: {100 - splitRatio}%;"
          bind:isEditing={isSplitEditing}
          tabs={rightTabsInfo}
          activeTabPath={activeSplitTabPath}
          canGoBack={false}
          canGoForward={false}
          isMarkdownFile={isMarkdownSplitTab}
          showViewToggle={hasActiveSplitContent && !isSyntheticTab(activeSplitTabPath)}
          markdownViewMode={!isSplitEditing ? "reading" : splitMarkdownViewMode}
          onChangeMarkdownView={handleChangeSplitMarkdownView}
          title={getTabDisplayTitle(activeSplitTabPath, currentSplitVaultItem.relative_path || currentSplitVaultItem.title)}
          showSaveButton={(!autoSave || (!!activeSplitTabPath && lockedFiles.has(activeSplitTabPath))) &&
            isSplitEditing &&
            !!activeSplitTabPath &&
            !isSyntheticTab(activeSplitTabPath) &&
            (!currentSplitVaultItem.relative_path ||
              isMarkdownFile(currentSplitVaultItem.relative_path) ||
              isDrawingTab(currentSplitVaultItem.relative_path, openedNotes[activeSplitTabPath]?.content) ||
              isPdfFile(currentSplitVaultItem.relative_path))}
          showWindowControls={true}
          isSplitActive={true}
          onSelectTab={(path) => selectSplitTab(path)}
          onCloseTab={(path) => requestCloseSplitTab(path)}
          onCloseAllTabs={closeAllSplitTabs}
          onCloseOtherTabs={closeOtherSplitTabs}
          onCloseTabsToRight={closeSplitTabsToRight}
          onOpenInNewWindow={handleOpenInNewWindow}
          onReorderTab={reorderSplitTab}
          onReceiveTab={(p, idx, src) => receiveTabFromSplitOrWindow(p, true, idx, src)}
          onNewTab={handleNewSplitEmptyTab}
          onNewFile={() => createNewVaultItem()}
          onOpenQuickOpen={openQuickOpenModal}
          onSplitView={handleSplitView}
          onToggleView={toggleSplitViewMode}
          onSave={() => {
            if (activeSplitTabPath && currentSplitVaultItem.relative_path) {
              requestManualSave(currentSplitVaultItem);
            }
          }}
          onOpenCommandPalette={() => (isPaletteOpen = true)}
          onPointerDown={() => { activeFocusedPane = 'right'; }}
          onDragOver={(e) => handlePaneDragOver(e, 'right')}
          onDrop={(e) => handlePaneDrop(e, 'right')}
        >
          {@render renderPaneContent(splitTabPaths, activeSplitTabPath, true)}
        </WorkspacePane>
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
      mcpStatus={mcpStatus}
      onToggleMcp={handleToggleMcp}
      onToggleVim={() => appSettings.toggleVimMode()}
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
    bind:searchQuery={quickOpenSearchQuery}
    {vaultItems}
    {recentFiles}
    onSelectFile={handleSelectQuickOpenFile}
  />

  <!-- 6. BÚSQUEDA EN EL CONTENIDO DE NOTAS (FULL-TEXT SEARCH CTRL+SHIFT+F) -->
  <FullTextSearch
    bind:isOpen={isFullTextSearchOpen}
    bind:searchQuery={ftsSearchQuery}
    onSelectHit={handleSelectFullTextHit}
  />

  <!-- 7. GESTOR Y EXPLORADOR DE PLUGINS DE LA COMUNIDAD DE OBSIDIAN -->
  <CommunityPluginsModal
    bind:open={isPluginsModalOpen}
    app={synapseApp}
    repository={vaultRepository}
  />

  <!-- 8. DIÁLOGO DE ADVERTENCIA PARA ARCHIVOS BLOQUEADOS CON CAMBIOS SIN PERSISTIR -->
  <UnsavedChangesDialog
    bind:open={unsavedDialogOpen}
    files={unsavedDialogFiles}
    isWindowClose={unsavedDialogIsWindowClose}
    isManualSave={pendingCloseAction?.type === 'manual-save'}
    loading={isForceSavingUnsaved}
    onAbort={handleUnsavedDialogAbort}
    onDiscard={handleUnsavedDialogDiscard}
    onForceSave={handleUnsavedDialogForceSave}
  />

  <!-- 9. DIÁLOGOS DE GESTIÓN DE GIT (BRANCHES, COMMITS, HISTORIAL DE ARCHIVO) -->
  <GitManagementDialogs
    {gitBranch}
    {gitStatuses}
    onRefreshGit={refreshGitStatus}
    onViewRevision={(p, rev) => selectTab(`rev:${rev}:${p}`)}
    onDiffRevision={(p, rev) => selectTab(`diff-rev:${rev}:${p}`)}
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
</style>
