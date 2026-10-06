<script lang="ts">
  import { tick } from 'svelte';
  import type { VaultItem } from '@entities/vault-item';
  import { vaultRepository, type GitFileStatusKind } from '@shared/repositories';
  import { ContextMenu, Collapsible } from 'bits-ui';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import { FileIcon, FolderIcon } from '@shared/ui/icons';
  import { GitBranch, Link, Copy, Trash2, Check, ChevronRight, PanelLeftClose, Files, RotateCcw, Undo2, Plus, RefreshCw, Crosshair } from 'lucide-svelte';

  interface Props {
    activeRibbonTab: string;
    sidebarWidth: number;
    isResizingSidebar: boolean;
    isConnectedToRust: boolean;
    vaultItems: VaultItem[];
    activeTabPath: string | null;
    vaultPath?: string;
    gitStatuses?: Record<string, GitFileStatusKind>;
    isGitRepo?: boolean;
    gitBranch?: string | null;
    onSelectTab: (path: string) => void;
    onOpenVaultFolder: () => void;
    onDeleteItem?: (relativePath: string, isFolder: boolean) => Promise<void> | void;
    onDeleteItems?: (items: Array<{ relativePath: string; isFolder: boolean }>) => Promise<void> | void;
    onResizeStart: (e: MouseEvent | PointerEvent) => void;
    onResizeMove?: (e: PointerEvent) => void;
    onResizeEnd?: (e: PointerEvent) => void;
    onCollapse?: () => void;
    onRefreshGit?: () => Promise<void> | void;
    onGitRestore?: (paths: string[]) => Promise<void> | void;
    onReloadItems?: (paths: string[], reloadedFiles?: string[]) => Promise<void> | void;
  }

  let {
    activeRibbonTab,
    sidebarWidth,
    isResizingSidebar,
    isConnectedToRust,
    vaultItems,
    activeTabPath,
    vaultPath,
    gitStatuses = {},
    isGitRepo = false,
    gitBranch = null,
    onSelectTab,
    onOpenVaultFolder,
    onDeleteItem,
    onDeleteItems,
    onResizeStart,
    onResizeMove,
    onResizeEnd,
    onCollapse,
    onRefreshGit,
    onGitRestore,
    onReloadItems,
  }: Props = $props();

  function getGitStatusInfo(raw: unknown): {
    index: string | null;
    worktree: string | null;
    is_stashed: boolean;
  } | null {
    if (!raw) return null;
    if (typeof raw === 'string') {
      if (raw === 'modified') return { index: null, worktree: 'M', is_stashed: false };
      if (raw === 'untracked') return { index: null, worktree: '?', is_stashed: false };
      if (raw === 'staged') return { index: 'M', worktree: null, is_stashed: false };
      if (raw === 'staged_modified' || raw === 'modified_staged') return { index: 'M', worktree: 'M', is_stashed: false };
      if (raw === 'stashed') return { index: null, worktree: null, is_stashed: true };
      if (raw === 'stashed_modified' || raw === 'modified_stashed') return { index: null, worktree: 'M', is_stashed: true };
      return null;
    }
    if (typeof raw === 'object') {
      const s = raw as Record<string, unknown>;
      const index = typeof s.index === 'string' ? s.index : null;
      const worktree = typeof s.worktree === 'string' ? s.worktree : null;
      const is_stashed = Boolean(s.is_stashed);
      if (!index && !worktree && !is_stashed) return null;
      return { index, worktree, is_stashed };
    }
    return null;
  }

  let vaultFolderName = $derived.by(() => {
    if (!vaultPath) return 'Bóveda de Archivos';
    const cleanPath = vaultPath.replace(/[\\/]+$/, '');
    const segments = cleanPath.split(/[\\/]/).filter(Boolean);
    return segments.pop() || vaultPath;
  });

  export interface VaultTreeNode {
    name: string;
    relativePath: string;
    isFolder: boolean;
    item?: VaultItem;
    children: VaultTreeNode[];
  }

  // Estado de carpetas expandidas (por defecto TODAS colapsadas)
  let expandedFolders = $state<Record<string, boolean>>({});

  // Caché de hijos de carpetas cargados bajo demanda desde Rust DashMap
  let folderChildren = $state<Record<string, VaultTreeNode[]>>({});

  export function getChildren(node: VaultTreeNode): VaultTreeNode[] {
    return folderChildren[node.relativePath] || node.children || [];
  }

  async function loadDirectory(parentPath: string = '') {
    if (!isConnectedToRust) return;
    try {
      const entries = await vaultRepository.getDirectoryChildren(parentPath);
      const nodes: VaultTreeNode[] = entries.map((entry) => ({
        name: entry.name,
        relativePath: entry.relative_path,
        isFolder: entry.is_folder,
        item: entry.is_folder
          ? undefined
          : {
              id: entry.relative_path,
              title: entry.title || entry.name.replace(/\.[^/.]+$/, ''),
              relative_path: entry.relative_path,
            },
        children: [],
      }));
      folderChildren[parentPath] = nodes;
    } catch (err) {
      console.error(`Error al cargar directorio '${parentPath}':`, err);
    }
  }

  let persistFoldersTimer: ReturnType<typeof setTimeout> | null = null;
  function persistExpandedFolders() {
    if (!isConnectedToRust) return;
    if (persistFoldersTimer) clearTimeout(persistFoldersTimer);
    persistFoldersTimer = setTimeout(async () => {
      const expandedList = Object.entries(expandedFolders)
        .filter(([_, isExpanded]) => isExpanded)
        .map(([path]) => path);
      await vaultRepository.saveVaultUiState(undefined, expandedList);
    }, 250);
  }

  async function restoreExpandedFolders() {
    if (!isConnectedToRust) return;
    try {
      const uiState = await vaultRepository.getVaultUiState();
      if (uiState?.expanded_folders && uiState.expanded_folders.length > 0) {
        for (const folder of uiState.expanded_folders) {
          expandedFolders[folder] = true;
          await loadDirectory(folder);
        }
      }
    } catch (err) {
      console.error('Error al restaurar carpetas expandidas:', err);
    }
  }

  // Carga inicial de la raíz y al cambiar de bóveda activa
  $effect(() => {
    const _path = vaultPath;
    const _connected = isConnectedToRust;
    if (_connected) {
      loadDirectory('');
      restoreExpandedFolders();
    }
  });

  async function toggleFolder(folderPath: string) {
    const nextState = !expandedFolders[folderPath];
    expandedFolders[folderPath] = nextState;
    if (nextState && !folderChildren[folderPath]) {
      await loadDirectory(folderPath);
    }
    persistExpandedFolders();
  }

  // Construir el árbol jerárquico a partir de la lista plana de vaultItems (fallback)
  let treeNodes = $derived.by(() => {
    const rootNodes: VaultTreeNode[] = [];
    const nodeMap: Record<string, VaultTreeNode> = {};

    vaultItems.forEach((vaultItem) => {
      const rawPath = vaultItem.relative_path || `${vaultItem.title}.md`;
      const parts = rawPath.split('/').filter(Boolean);
      let currentPath = '';

      parts.forEach((part, index) => {
        const isLast = index === parts.length - 1;
        const isFolder = !isLast;
        currentPath = currentPath ? `${currentPath}/${part}` : part;

        if (!nodeMap[currentPath]) {
          const node: VaultTreeNode = {
            name: part,
            relativePath: currentPath,
            isFolder,
            item: isLast ? vaultItem : undefined,
            children: [],
          };
          nodeMap[currentPath] = node;

          if (index === 0) {
            rootNodes.push(node);
          } else {
            const parentPath = currentPath.substring(0, currentPath.lastIndexOf('/'));
            if (nodeMap[parentPath]) {
              nodeMap[parentPath].children.push(node);
            }
          }
        }
      });
    });

    // Ordenar carpetas primero y luego archivos en orden alfabético
    function sortNodes(nodes: VaultTreeNode[]) {
      nodes.sort((a, b) => {
        if (a.isFolder && !b.isFolder) return -1;
        if (!a.isFolder && b.isFolder) return 1;
        return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
      });
      nodes.forEach((node) => {
        if (node.isFolder) sortNodes(node.children);
      });
    }

    sortNodes(rootNodes);
    return rootNodes;
  });

  // Árbol activo: usa el cargado perezosamente desde Rust si está disponible, o el fallback
  let displayTreeNodes = $derived(
    folderChildren[''] && folderChildren[''].length > 0
      ? folderChildren['']
      : treeNodes
  );

  // Expandir carpetas padres automáticamente cuando un archivo se selecciona como pestaña activa
  $effect(() => {
    const path = activeTabPath;
    if (path && path.includes('/')) {
      const parts = path.split('/').filter(Boolean);
      let currentPath = '';
      let changed = false;
      for (let i = 0; i < parts.length - 1; i++) {
        currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];
        if (!expandedFolders[currentPath]) {
          expandedFolders[currentPath] = true;
          changed = true;
        }
        if (!folderChildren[currentPath]) {
          loadDirectory(currentPath);
        }
      }
      if (changed) {
        persistExpandedFolders();
      }
    }
  });

  // Estado de selección múltiple y navegación
  let selectedPaths = $state<string[]>([]);
  let lastFocusedPath = $state<string | null>(null);
  let anchorPath = $state<string | null>(null);
  let sidebarPanelEl = $state<HTMLElement | null>(null);
  let sidebarContentEl = $state<HTMLElement | null>(null);

  function isSelected(path: string): boolean {
    return selectedPaths.includes(path);
  }

  // Sincronizar selección inicial con la pestaña activa únicamente cuando cambie la pestaña
  let prevActiveTab = $state<string | null>(null);
  $effect(() => {
    const current = activeTabPath;
    if (current && current !== prevActiveTab) {
      prevActiveTab = current;
      selectedPaths = [current];
      lastFocusedPath = current;
      anchorPath = current;
    }
  });

  // Obtener la lista aplanada de nodos visibles actualmente en el árbol
  function getVisibleNodes(nodes: VaultTreeNode[]): VaultTreeNode[] {
    const visible: VaultTreeNode[] = [];
    for (const node of nodes) {
      visible.push(node);
      if (node.isFolder && expandedFolders[node.relativePath]) {
        visible.push(...getVisibleNodes(getChildren(node)));
      }
    }
    return visible;
  }

  function scrollPathIntoView(path: string) {
    tick().then(() => {
      if (!sidebarContentEl) return;
      const selector = `[data-path="${CSS.escape(path)}"]`;
      const el = sidebarContentEl.querySelector<HTMLElement>(selector);
      el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    });
  }

  export async function locateActiveFile(targetPath?: string) {
    const pathToLocate = targetPath || activeTabPath;
    if (!pathToLocate || pathToLocate.startsWith('empty:')) {
      showToast('No hay ningún archivo activo para ubicar');
      return;
    }

    if (!folderChildren[''] || folderChildren[''].length === 0) {
      await loadDirectory('');
    }

    const parts = pathToLocate.split('/').filter(Boolean);
    let currentPath = '';
    let changed = false;

    for (let i = 0; i < parts.length - 1; i++) {
      currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];
      if (!folderChildren[currentPath]) {
        await loadDirectory(currentPath);
      }
      if (!expandedFolders[currentPath]) {
        expandedFolders[currentPath] = true;
        changed = true;
      }
    }

    if (changed) {
      persistExpandedFolders();
    }

    selectedPaths = [pathToLocate];
    lastFocusedPath = pathToLocate;
    anchorPath = pathToLocate;

    await tick();

    setTimeout(() => {
      if (!sidebarContentEl) return;
      const selector = `[data-path="${CSS.escape(pathToLocate)}"]`;
      const el = sidebarContentEl.querySelector<HTMLElement>(selector);
      if (el) {
        el.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'smooth' });
        el.classList.remove('locate-highlight');
        void el.offsetWidth;
        el.classList.add('locate-highlight');
        setTimeout(() => {
          el?.classList.remove('locate-highlight');
        }, 1800);
      }
      sidebarPanelEl?.focus();
    }, 60);
  }

  function handleActivateNode(node: VaultTreeNode) {
    if (node.isFolder) {
      toggleFolder(node.relativePath);
    } else {
      onSelectTab(node.relativePath);
    }
  }

  // Alternar selección individual con Ctrl/Cmd
  function toggleItemSelection(path: string) {
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter((p) => p !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
    lastFocusedPath = path;
    anchorPath = path;
  }

  // Selección de rango con Shift
  function selectRange(startPath: string, endPath: string) {
    const visible = getVisibleNodes(displayTreeNodes);
    const startIdx = visible.findIndex((n) => n.relativePath === startPath);
    const endIdx = visible.findIndex((n) => n.relativePath === endPath);

    if (startIdx === -1 || endIdx === -1) {
      selectedPaths = [endPath];
      lastFocusedPath = endPath;
      anchorPath = endPath;
      return;
    }

    const min = Math.min(startIdx, endIdx);
    const max = Math.max(startIdx, endIdx);
    const range = visible.slice(min, max + 1).map((n) => n.relativePath);
    selectedPaths = range;
    lastFocusedPath = endPath;
  }

  function navigateTree(direction: 'up' | 'down', e: KeyboardEvent) {
    const visible = getVisibleNodes(displayTreeNodes);
    if (visible.length === 0) return;

    let currentIndex = -1;
    if (lastFocusedPath) {
      currentIndex = visible.findIndex((n) => n.relativePath === lastFocusedPath);
    }
    if (currentIndex === -1 && selectedPaths.length > 0) {
      currentIndex = visible.findIndex((n) => selectedPaths.includes(n.relativePath));
    }
    if (currentIndex === -1 && activeTabPath) {
      currentIndex = visible.findIndex((n) => n.relativePath === activeTabPath);
    }

    let newIndex: number;
    if (direction === 'down') {
      if (currentIndex === -1) {
        newIndex = 0;
      } else {
        newIndex = Math.min(currentIndex + 1, visible.length - 1);
      }
    } else {
      if (currentIndex === -1) {
        newIndex = visible.length - 1;
      } else {
        newIndex = Math.max(currentIndex - 1, 0);
      }
    }

    const targetNode = visible[newIndex];
    if (!targetNode) return;

    e.preventDefault();

    if (e.shiftKey) {
      const anchor = anchorPath || lastFocusedPath || targetNode.relativePath;
      anchorPath = anchor;
      selectRange(anchor, targetNode.relativePath);
    } else {
      selectedPaths = [targetNode.relativePath];
      lastFocusedPath = targetNode.relativePath;
      anchorPath = targetNode.relativePath;
    }

    scrollPathIntoView(targetNode.relativePath);
  }

  // Nodos correspondientes a la selección activa
  let selectedNodes = $derived.by(() => {
    if (selectedPaths.length === 0) {
      return contextMenuNode ? [contextMenuNode] : [];
    }
    const map: Record<string, VaultTreeNode> = {};
    function collect(nodes: VaultTreeNode[]) {
      for (const n of nodes) {
        map[n.relativePath] = n;
        if (n.isFolder) collect(getChildren(n));
      }
    }
    collect(displayTreeNodes);
    return selectedPaths.map((p) => map[p]).filter(Boolean);
  });

  function handleFolderClick(e: MouseEvent, node: VaultTreeNode, defaultTriggerClick?: (e: MouseEvent) => void) {
    sidebarPanelEl?.focus();
    if (e.metaKey || e.ctrlKey) {
      e.preventDefault();
      e.stopPropagation();
      toggleItemSelection(node.relativePath);
      return;
    }
    if (e.shiftKey && lastFocusedPath) {
      e.preventDefault();
      e.stopPropagation();
      selectRange(anchorPath || lastFocusedPath, node.relativePath);
      return;
    }
    selectedPaths = [node.relativePath];
    lastFocusedPath = node.relativePath;
    anchorPath = node.relativePath;
    defaultTriggerClick?.(e);
  }

  function handleFileClick(e: MouseEvent, node: VaultTreeNode) {
    sidebarPanelEl?.focus();
    if (e.metaKey || e.ctrlKey) {
      e.stopPropagation();
      toggleItemSelection(node.relativePath);
      return;
    }
    if (e.shiftKey && lastFocusedPath) {
      e.stopPropagation();
      selectRange(anchorPath || lastFocusedPath, node.relativePath);
      return;
    }
    selectedPaths = [node.relativePath];
    lastFocusedPath = node.relativePath;
    anchorPath = node.relativePath;
    onSelectTab(node.relativePath);
  }

  function handleItemContextMenu(e: MouseEvent, node: VaultTreeNode) {
    sidebarPanelEl?.focus();
    if (!selectedPaths.includes(node.relativePath)) {
      selectedPaths = [node.relativePath];
      lastFocusedPath = node.relativePath;
      anchorPath = node.relativePath;
    }
    contextMenuNode = node;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (itemsToDelete.length > 0 || isContextMenuOpen) return;

    const target = e.target as HTMLElement;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
      return;
    }

    if (e.key === 'Delete' || (e.key === 'Backspace' && (e.metaKey || e.ctrlKey))) {
      if ((selectedNodes.length > 0 || contextMenuNode) && itemsToDelete.length === 0) {
        e.preventDefault();
        handlePromptDelete();
      }
    } else if (e.key === 'Escape') {
      selectedPaths = [];
      anchorPath = null;
    } else if (e.key === 'ArrowDown') {
      navigateTree('down', e);
    } else if (e.key === 'ArrowUp') {
      navigateTree('up', e);
    } else if (e.key === 'ArrowRight') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(displayTreeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node && node.isFolder) {
          e.preventDefault();
          if (!expandedFolders[node.relativePath]) {
            toggleFolder(node.relativePath);
          } else {
            const children = getChildren(node);
            if (children.length > 0) {
              const firstChild = children[0];
              selectedPaths = [firstChild.relativePath];
              lastFocusedPath = firstChild.relativePath;
              anchorPath = firstChild.relativePath;
              scrollPathIntoView(firstChild.relativePath);
            }
          }
        }
      }
    } else if (e.key === 'ArrowLeft') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(displayTreeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node && node.isFolder && expandedFolders[node.relativePath]) {
          e.preventDefault();
          expandedFolders[node.relativePath] = false;
          persistExpandedFolders();
        } else if (node) {
          const slashIdx = node.relativePath.lastIndexOf('/');
          if (slashIdx !== -1) {
            const parentRelPath = node.relativePath.slice(0, slashIdx);
            const parentNode = visible.find((n) => n.relativePath === parentRelPath);
            if (parentNode) {
              e.preventDefault();
              selectedPaths = [parentNode.relativePath];
              lastFocusedPath = parentNode.relativePath;
              anchorPath = parentNode.relativePath;
              scrollPathIntoView(parentNode.relativePath);
            }
          }
        }
      }
    } else if (e.key === 'Enter') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(displayTreeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node) {
          e.preventDefault();
          handleActivateNode(node);
        }
      }
    } else if (e.key === ' ') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(displayTreeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node) {
          e.preventDefault();
          handleActivateNode(node);
        }
      }
    } else if (e.key === 'Home') {
      const visible = getVisibleNodes(displayTreeNodes);
      if (visible.length > 0) {
        e.preventDefault();
        const first = visible[0];
        selectedPaths = [first.relativePath];
        lastFocusedPath = first.relativePath;
        anchorPath = first.relativePath;
        scrollPathIntoView(first.relativePath);
      }
    } else if (e.key === 'End') {
      const visible = getVisibleNodes(displayTreeNodes);
      if (visible.length > 0) {
        e.preventDefault();
        const last = visible[visible.length - 1];
        selectedPaths = [last.relativePath];
        lastFocusedPath = last.relativePath;
        anchorPath = last.relativePath;
        scrollPathIntoView(last.relativePath);
      }
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'a' || e.key === 'A')) {
      e.preventDefault();
      const visible = getVisibleNodes(displayTreeNodes);
      selectedPaths = visible.map((n) => n.relativePath);
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'c' || e.key === 'C')) {
      e.preventDefault();
      handleCopy();
    } else if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && (e.key === 'r' || e.key === 'R'))) {
      e.preventDefault();
      handleReload();
    } else if (
      (e.altKey && (e.key === 'l' || e.key === 'L')) ||
      ((e.ctrlKey || e.metaKey) && e.altKey && (e.key === '1' || e.code === 'Digit1' || e.code === 'Numpad1'))
    ) {
      e.preventDefault();
      locateActiveFile();
    }
  }

  // Estado del menú contextual con bits-ui
  let isContextMenuOpen = $state(false);
  let contextMenuNode = $state<VaultTreeNode | null>(null);

  let toastMessage = $state<string | null>(null);
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string) {
    if (toastTimeout) clearTimeout(toastTimeout);
    toastMessage = msg;
    toastTimeout = setTimeout(() => {
      toastMessage = null;
      toastTimeout = null;
    }, 2000);
  }

  function getFullAbsolutePath(relativePath: string, itemAbsPath?: string): string {
    if (itemAbsPath) return itemAbsPath;
    if (!vaultPath) return relativePath;
    const cleanVault = vaultPath.replace(/[\\/]+$/, '');
    if (!relativePath || relativePath === '.' || relativePath === './') {
      return cleanVault;
    }
    const isWindows = cleanVault.includes('\\') || /^[a-zA-Z]:/.test(cleanVault);
    const sep = isWindows ? '\\' : '/';
    const cleanRel = isWindows ? relativePath.replace(/\//g, '\\') : relativePath.replace(/\\/g, '/');
    return `${cleanVault}${sep}${cleanRel}`;
  }

  async function copyTextToClipboard(text: string): Promise<boolean> {
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textArea = document.createElement('textarea');
        textArea.value = text;
        textArea.style.position = 'fixed';
        textArea.style.opacity = '0';
        document.body.appendChild(textArea);
        textArea.focus();
        textArea.select();
        document.execCommand('copy');
        document.body.removeChild(textArea);
      }
      return true;
    } catch (err) {
      console.error('Error al copiar al portapapeles:', err);
      return false;
    }
  }

  function getSelectedRelativePaths(): string[] {
    if (contextMenuNode && (!selectedPaths.includes(contextMenuNode.relativePath) || selectedPaths.length <= 1)) {
      return [contextMenuNode.relativePath];
    }
    if (selectedPaths.length > 0) {
      return selectedPaths;
    }
    if (contextMenuNode) {
      return [contextMenuNode.relativePath];
    }
    return [];
  }

  async function handleCopy() {
    const paths = getSelectedRelativePaths();
    if (paths.length === 0) return;
    const text = paths.join('\n');
    const ok = await copyTextToClipboard(text);
    if (ok) {
      showToast(
        paths.length > 1
          ? `${paths.length} rutas copiadas al portapapeles`
          : 'Ruta copiada al portapapeles'
      );
    }
  }

  async function handleCopyRelativePath() {
    await handleCopy();
  }

  async function handleCopyFullPath() {
    const paths = getSelectedRelativePaths();
    if (paths.length === 0) return;
    const fullPaths = paths.map((relPath) => {
      const node =
        selectedNodes.find((n) => n.relativePath === relPath) ||
        (contextMenuNode?.relativePath === relPath ? contextMenuNode : undefined);
      return getFullAbsolutePath(relPath, node?.item?.abs_path);
    });
    const text = fullPaths.join('\n');
    const ok = await copyTextToClipboard(text);
    if (ok) {
      showToast(
        fullPaths.length > 1
          ? `${fullPaths.length} rutas completas copiadas al portapapeles`
          : 'Ruta completa copiada al portapapeles'
      );
    }
  }

  let isReloading = $state(false);

  let reloadMenuLabel = $derived.by(() => {
    const paths = getSelectedRelativePaths();
    if (paths.length > 1) {
      return `Recargar (${paths.length} elementos)`;
    }
    const isFolder =
      (contextMenuNode && contextMenuNode.isFolder) ||
      (paths.length === 1 && selectedNodes.some((n) => n.relativePath === paths[0] && n.isFolder));
    if (isFolder) {
      return 'Recargar carpeta';
    }
    if (contextMenuNode || paths.length === 1) {
      return 'Recargar archivo';
    }
    return 'Recargar bóveda';
  });

  async function reloadDirectoryAndSubdirectories(dirPath: string) {
    await loadDirectory(dirPath);
    const prefix = dirPath ? `${dirPath}/` : '';
    for (const folder of Object.keys(expandedFolders)) {
      if (expandedFolders[folder]) {
        if (!dirPath || folder === dirPath || folder.startsWith(prefix)) {
          await loadDirectory(folder);
        }
      }
    }
  }

  async function handleReload() {
    if (isReloading) return;
    const paths = getSelectedRelativePaths();
    const targets = paths.length > 0 ? paths : [''];
    isReloading = true;
    try {
      const reloadedFiles = await vaultRepository.reloadVaultItems(targets);

      const refreshedParents = new Set<string>();
      for (const t of targets) {
        if (!t || t === '.') {
          await reloadDirectoryAndSubdirectories('');
          refreshedParents.add('');
          break;
        }

        const isFolder =
          selectedNodes.some((n) => n.relativePath === t && n.isFolder) ||
          (contextMenuNode?.relativePath === t && contextMenuNode.isFolder);

        if (isFolder) {
          await reloadDirectoryAndSubdirectories(t);
        } else {
          const slash = t.lastIndexOf('/');
          const parentDir = slash !== -1 ? t.slice(0, slash) : '';
          if (!refreshedParents.has(parentDir)) {
            refreshedParents.add(parentDir);
            await loadDirectory(parentDir);
          }
        }
      }

      const isSingleFolder =
        targets.length === 1 &&
        (contextMenuNode?.isFolder || selectedNodes.some((n) => n.relativePath === targets[0] && n.isFolder));

      showToast(
        targets.length > 1
          ? `${targets.length} elementos recargados`
          : targets[0] === ''
            ? 'Bóveda recargada'
            : isSingleFolder
              ? 'Carpeta recargada'
              : 'Archivo recargado'
      );

      if (onReloadItems) {
        await onReloadItems(targets, reloadedFiles);
      }
    } catch (err: unknown) {
      console.error('Error al recargar elemento(s):', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error al recargar: ${msg}`);
    } finally {
      isReloading = false;
    }
  }

  // Estado del diálogo de confirmación de borrado múltiple
  interface ItemToDelete {
    name: string;
    relativePath: string;
    isFolder: boolean;
  }

  let itemsToDelete = $state<ItemToDelete[]>([]);
  let isDeleting = $state(false);
  let isDeleteDialogOpen = $state(false);

  function handlePromptDelete() {
    if (selectedNodes.length > 0) {
      itemsToDelete = selectedNodes.map((n) => ({
        name: n.name,
        relativePath: n.relativePath,
        isFolder: n.isFolder,
      }));
      isDeleteDialogOpen = true;
    } else if (contextMenuNode) {
      itemsToDelete = [
        {
          name: contextMenuNode.name,
          relativePath: contextMenuNode.relativePath,
          isFolder: contextMenuNode.isFolder,
        },
      ];
      isDeleteDialogOpen = true;
    }
  }

  function cancelDelete() {
    if (isDeleting) return;
    isDeleteDialogOpen = false;
    itemsToDelete = [];
  }

  async function confirmDelete() {
    if (itemsToDelete.length === 0 || isDeleting) return;
    const targets = [...itemsToDelete];
    isDeleting = true;
    try {
      if (onDeleteItems) {
        await onDeleteItems(
          targets.map((t) => ({ relativePath: t.relativePath, isFolder: t.isFolder }))
        );
      } else {
        for (const target of targets) {
          if (onDeleteItem) {
            await onDeleteItem(target.relativePath, target.isFolder);
          } else {
            await vaultRepository.deleteItem(target.relativePath);
          }
        }
      }
      showToast(
        targets.length > 1
          ? `${targets.length} elementos eliminados`
          : `"${targets[0].name}" ha sido eliminado`
      );
      const deletedSet = new Set(targets.map((t) => t.relativePath));
      selectedPaths = selectedPaths.filter((p) => !deletedSet.has(p));
      itemsToDelete = [];
      isDeleteDialogOpen = false;
    } catch (err: unknown) {
      console.error('Error al borrar elemento(s):', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error al borrar: ${msg}`);
    } finally {
      isDeleting = false;
    }
  }

  function getFileGitStatus(relPath: string) {
    const clean = relPath.replace(/^\.\//, '');
    const forward = clean.replace(/\\/g, '/');
    const backslash = clean.replace(/\//g, '\\');
    const raw = gitStatuses
      ? (gitStatuses[relPath] ?? gitStatuses[clean] ?? gitStatuses[forward] ?? gitStatuses[backslash])
      : undefined;
    return getGitStatusInfo(raw);
  }

  function getEligiblePathsForGit(
    predicate: (status: NonNullable<ReturnType<typeof getGitStatusInfo>>) => boolean
  ): string[] {
    const rawPaths = getSelectedRelativePaths();
    if (!isGitRepo || !gitStatuses || rawPaths.length === 0) return [];

    const eligible: string[] = [];
    for (const p of rawPaths) {
      const cleanPath = p.replace(/^\.\//, '');
      const s = getFileGitStatus(cleanPath);
      if (s && predicate(s)) {
        eligible.push(p);
        continue;
      }

      // Si es una carpeta, verificar si algún archivo dentro de ella cumple la condición
      const isFolder =
        selectedNodes.some((n) => n.relativePath === p && n.isFolder) ||
        (contextMenuNode?.relativePath === p && contextMenuNode.isFolder);

      if (isFolder) {
        const forward = cleanPath.replace(/\\/g, '/');
        const prefix = forward.endsWith('/') ? forward : `${forward}/`;
        const hasMatch = Object.entries(gitStatuses).some(([key, raw]) => {
          const cleanKey = key.replace(/^\.\//, '').replace(/\\/g, '/');
          if (cleanKey.startsWith(prefix)) {
            const fs = getGitStatusInfo(raw);
            return fs ? predicate(fs) : false;
          }
          return false;
        });

        if (hasMatch) {
          eligible.push(p);
        }
      }
    }

    return eligible;
  }

  // Rutas elegibles para Git Add: modificados (que no estén en stash) o no versionados (untracked)
  let gitAddPaths = $derived(
    getEligiblePathsForGit((status) => {
      const isModifiedNotStashed = status.worktree === 'M' && !status.is_stashed;
      const isUntracked = status.worktree === '?' && !status.is_stashed;
      return isModifiedNotStashed || isUntracked;
    })
  );

  // Rutas elegibles para Git Restore: modificados en el árbol de trabajo (worktree === 'M')
  let gitRestorePaths = $derived(
    getEligiblePathsForGit((status) => status.worktree === 'M')
  );

  // Rutas elegibles para Git Restore --staged: archivos en stage (index !== null)
  let gitRestoreStagedPaths = $derived(
    getEligiblePathsForGit((status) => Boolean(status.index))
  );

  let canGitAdd = $derived(gitAddPaths.length > 0);
  let canGitRestore = $derived(gitRestorePaths.length > 0);
  let canGitRestoreStaged = $derived(gitRestoreStagedPaths.length > 0);

  // Acciones de Git sobre archivos seleccionados
  async function handleGitAdd() {
    const paths = gitAddPaths.length > 0 ? gitAddPaths : getSelectedRelativePaths();
    if (paths.length === 0) return;
    try {
      await vaultRepository.gitAdd(paths);
      showToast(
        paths.length > 1
          ? `${paths.length} elementos añadidos al stage (git add)`
          : 'Elemento añadido al stage (git add)'
      );
      if (onRefreshGit) onRefreshGit();
    } catch (err: unknown) {
      console.error('Error al ejecutar git add:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git add: ${msg}`);
    }
  }

  async function handleGitRestoreStaged() {
    const paths = gitRestoreStagedPaths.length > 0 ? gitRestoreStagedPaths : getSelectedRelativePaths();
    if (paths.length === 0) return;
    try {
      await vaultRepository.gitRestoreStaged(paths);
      showToast(
        paths.length > 1
          ? `${paths.length} elementos desmarcados de stage (git restore --staged)`
          : 'Elemento desmarcado de stage (git restore --staged)'
      );
      if (onRefreshGit) onRefreshGit();
    } catch (err: unknown) {
      console.error('Error al ejecutar git restore --staged:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git restore --staged: ${msg}`);
    }
  }

  let isRestoreDialogOpen = $state(false);
  let isRestoring = $state(false);
  let itemsToRestore = $state<string[]>([]);

  function handlePromptGitRestore() {
    const paths = gitRestorePaths.length > 0 ? gitRestorePaths : getSelectedRelativePaths();
    if (paths.length === 0) return;
    itemsToRestore = paths;
    isRestoreDialogOpen = true;
  }

  function cancelGitRestore() {
    if (isRestoring) return;
    isRestoreDialogOpen = false;
    itemsToRestore = [];
  }

  async function confirmGitRestore() {
    if (itemsToRestore.length === 0 || isRestoring) return;
    const targets = [...itemsToRestore];
    isRestoring = true;
    try {
      await vaultRepository.gitRestore(targets);
      showToast(
        targets.length > 1
          ? `${targets.length} elementos restaurados (git restore)`
          : 'Elemento restaurado (git restore)'
      );
      if (onGitRestore) {
        await onGitRestore(targets);
      }
      if (onRefreshGit) {
        onRefreshGit();
      }
      itemsToRestore = [];
      isRestoreDialogOpen = false;
    } catch (err: unknown) {
      console.error('Error al restaurar con git restore:', err);
      const msg = err instanceof Error ? err.message : String(err);
      showToast(`Error en git restore: ${msg}`);
    } finally {
      isRestoring = false;
    }
  }
</script>

{#if activeRibbonTab === 'files' || activeRibbonTab === 'search'}
  <ContextMenu.Root bind:open={isContextMenuOpen}>
    <ContextMenu.Trigger>
      {#snippet child({ props })}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <aside
          bind:this={sidebarPanelEl}
          {...props}
          class="sidebar-panel"
          class:is-resizing={isResizingSidebar}
          style="{props.style ? props.style + ';' : ''} width: {sidebarWidth}px; min-width: {sidebarWidth}px; max-width: {sidebarWidth}px;"
          tabindex="0"
          onkeydown={handleKeyDown}
        >
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="sidebar-header"
            oncontextmenu={() => {
              contextMenuNode = null;
              selectedPaths = [];
              lastFocusedPath = null;
              anchorPath = null;
            }}
          >
            <div class="sidebar-header-left">
              <button
                // class="sidebar-title"
                class="rust-badge-btn"
                onclick={onOpenVaultFolder}
                title={activeRibbonTab === 'files' ? (vaultPath || 'Bóveda de Archivos') : 'Buscar'}
              >
                {activeRibbonTab === 'files' ? vaultFolderName : 'Buscar'}
              </button>
              {#if isGitRepo && gitBranch}
                <button
                  type="button"
                  class="git-branch-badge"
                  onclick={onRefreshGit}
                  title="Rama Git: {gitBranch} (Clic para refrescar)"
                >
                  <GitBranch size={13} class="git-branch-icon" />
                  <span class="git-branch-name">{gitBranch}</span>
                </button>
              {/if}
            </div>
            <div class="sidebar-header-actions">
              <button
                type="button"
                class="sidebar-action-btn"
                onclick={() => locateActiveFile()}
                title="Ubicar archivo activo (Ctrl+Alt+1, Alt+L)"
                aria-label="Ubicar archivo activo"
                disabled={!activeTabPath || activeTabPath.startsWith('empty:')}
              >
                <Crosshair size={14} />
              </button>
              {#if onCollapse}
                <button
                  type="button"
                  class="sidebar-collapse-btn"
                  onclick={onCollapse}
                  title="Colapsar panel lateral (Alt+1, Ctrl+B)"
                  aria-label="Colapsar panel lateral (Alt+1, Ctrl+B)"
                >
                  <PanelLeftClose size={14} />
                </button>
              {/if}
            </div>
          </div>

          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            bind:this={sidebarContentEl}
            class="sidebar-content"
            onclick={(e) => {
              sidebarPanelEl?.focus();
              if (!(e.target as HTMLElement).closest('.file-tree-item')) {
                selectedPaths = [];
                lastFocusedPath = null;
                anchorPath = null;
              }
            }}
            oncontextmenu={(e) => {
              sidebarPanelEl?.focus();
              if (!(e.target as HTMLElement).closest('.file-tree-item')) {
                contextMenuNode = null;
                selectedPaths = [];
                lastFocusedPath = null;
                anchorPath = null;
              }
            }}
          >
            {#each displayTreeNodes as rootNode (rootNode.relativePath)}
              {@render renderNode(rootNode, 0)}
            {/each}
          </div>

          <!-- Tirador para redimensionar el panel lateral -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="sidebar-resizer"
            role="separator"
            aria-orientation="vertical"
            tabindex="-1"
            onpointerdown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onResizeStart(e);
            }}
            onmousedown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onResizeStart(e);
            }}
            ondblclick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (onCollapse) onCollapse();
            }}
            title="Arrastrar para cambiar tamaño (doble clic para colapsar)"
          ></div>
        </aside>
      {/snippet}
    </ContextMenu.Trigger>

    <ContextMenu.Portal>
      <ContextMenu.Content class="vault-context-menu">
        <div
          class="context-menu-header"
          title={selectedNodes.length > 1
            ? `${selectedNodes.length} elementos seleccionados`
            : (contextMenuNode ? contextMenuNode.relativePath : (vaultPath || 'Bóveda'))}
        >
          {#if selectedNodes.length > 1}
            <Files size={15} class="context-menu-header-icon" />
            <span class="context-menu-header-title">
              {selectedNodes.length} elementos seleccionados
            </span>
          {:else if contextMenuNode?.isFolder}
            <FolderIcon isOpen={false} name={contextMenuNode.name} size={15} class="context-menu-header-icon" />
            <span class="context-menu-header-title">
              {contextMenuNode.name}
            </span>
          {:else if contextMenuNode}
            <FileIcon path={contextMenuNode.relativePath} name={contextMenuNode.name} size={15} class="context-menu-header-icon" />
            <span class="context-menu-header-title">
              {contextMenuNode.name}
            </span>
          {:else}
            <FolderIcon isOpen={true} name={vaultFolderName} size={15} class="context-menu-header-icon" />
            <span class="context-menu-header-title">
              {vaultFolderName || 'Bóveda'}
            </span>
          {/if}
        </div>

        <ContextMenu.Separator class="context-menu-divider" />

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleReload}
        >
          <RefreshCw size={14} class="context-menu-item-icon" />
          <span>{reloadMenuLabel}</span>
          <span class="context-menu-shortcut">Ctrl+R</span>
        </ContextMenu.Item>

        <ContextMenu.Separator class="context-menu-divider" />

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleCopy}
        >
          <Copy size={14} class="context-menu-item-icon" />
          <span>{selectedPaths.length > 1 ? `Copiar (${selectedPaths.length} rutas)` : 'Copiar'}</span>
          <span class="context-menu-shortcut">Ctrl+C</span>
        </ContextMenu.Item>

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleCopyRelativePath}
        >
          <Link size={14} class="context-menu-item-icon" />
          <span>{selectedPaths.length > 1 ? 'Copiar rutas relativas' : 'Copiar ruta relativa'}</span>
        </ContextMenu.Item>

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleCopyFullPath}
        >
          <Copy size={14} class="context-menu-item-icon" />
          <span>{selectedPaths.length > 1 ? 'Copiar rutas completas' : 'Copiar ruta completa'}</span>
        </ContextMenu.Item>

        {#if isGitRepo && (canGitAdd || canGitRestoreStaged || canGitRestore)}
          <ContextMenu.Separator class="context-menu-divider" />

          {#if canGitAdd}
            <ContextMenu.Item
              class="context-menu-item"
              onSelect={handleGitAdd}
            >
              <Plus size={14} class="context-menu-item-icon" />
              <span>{gitAddPaths.length > 1 ? `Git Add (${gitAddPaths.length} elementos)` : 'Git Add'}</span>
            </ContextMenu.Item>
          {/if}

          {#if canGitRestoreStaged}
            <ContextMenu.Item
              class="context-menu-item"
              onSelect={handleGitRestoreStaged}
            >
              <Undo2 size={14} class="context-menu-item-icon" />
              <span>{gitRestoreStagedPaths.length > 1 ? `Git Restore --staged (${gitRestoreStagedPaths.length} elementos)` : 'Git Restore --staged'}</span>
            </ContextMenu.Item>
          {/if}

          {#if canGitRestore}
            <ContextMenu.Item
              class="context-menu-item"
              onSelect={handlePromptGitRestore}
            >
              <RotateCcw size={14} class="context-menu-item-icon" />
              <span>{gitRestorePaths.length > 1 ? `Git Restore (${gitRestorePaths.length} elementos)` : 'Git Restore'}</span>
            </ContextMenu.Item>
          {/if}
        {/if}

        {#if selectedPaths.length > 0 || contextMenuNode}
          <ContextMenu.Separator class="context-menu-divider" />

          <ContextMenu.Item
            class="context-menu-item delete"
            onSelect={handlePromptDelete}
          >
            <Trash2 size={14} class="context-menu-item-icon" />
            <span>{selectedPaths.length > 1 ? `Borrar (${selectedPaths.length} elementos)` : 'Borrar'}</span>
            <span class="context-menu-shortcut">Supr</span>
          </ContextMenu.Item>
        {/if}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
{/if}

<ConfirmDialog
  bind:open={isDeleteDialogOpen}
  title={itemsToDelete.length > 1
    ? `¿Eliminar ${itemsToDelete.length} elementos?`
    : `¿Eliminar ${itemsToDelete[0]?.isFolder ? 'carpeta' : 'archivo'}?`}
  description={itemsToDelete.length > 1
    ? `¿Estás seguro de que deseas eliminar permanentemente estos ${itemsToDelete.length} elementos y todo su contenido? Esta acción no se puede deshacer.`
    : itemsToDelete.length === 1
      ? `¿Estás seguro de que deseas eliminar permanentemente "${itemsToDelete[0]?.name}"${itemsToDelete[0]?.isFolder ? ' y todo su contenido' : ''}? Esta acción no se puede deshacer.`
      : ''}
  confirmText="Eliminar"
  loadingText="Eliminando..."
  cancelText="Cancelar"
  variant="danger"
  icon={Trash2}
  loading={isDeleting}
  onConfirm={confirmDelete}
  onCancel={cancelDelete}
>
  {#if itemsToDelete.length > 1}
    <div class="confirm-dialog-item-list">
      {#each itemsToDelete.slice(0, 5) as item}
        <div class="confirm-dialog-item">
          • {item.name} {item.isFolder ? '(carpeta)' : ''}
        </div>
      {/each}
      {#if itemsToDelete.length > 5}
        <div class="confirm-dialog-item more">
          ... y {itemsToDelete.length - 5} más
        </div>
      {/if}
    </div>
  {/if}
</ConfirmDialog>

<ConfirmDialog
  bind:open={isRestoreDialogOpen}
  title={itemsToRestore.length > 1
    ? `¿Restaurar ${itemsToRestore.length} elementos con Git?`
    : `¿Restaurar "${itemsToRestore[0] || ''}" con Git?`}
  description={itemsToRestore.length > 1
    ? `¿Estás seguro de que deseas descartar las modificaciones locales no confirmadas en Git de estos ${itemsToRestore.length} elementos? Esta acción sobreescribirá los cambios no preparados en tu espacio de trabajo.`
    : `¿Estás seguro de que deseas descartar las modificaciones locales no confirmadas en Git de "${itemsToRestore[0] || ''}"? Esta acción sobreescribirá los cambios no preparados en tu espacio de trabajo.`}
  confirmText="Restaurar cambios"
  loadingText="Restaurando..."
  cancelText="Cancelar"
  variant="danger"
  icon={RotateCcw}
  loading={isRestoring}
  onConfirm={confirmGitRestore}
  onCancel={cancelGitRestore}
>
  {#if itemsToRestore.length > 1}
    <div class="confirm-dialog-item-list">
      {#each itemsToRestore.slice(0, 5) as path}
        <div class="confirm-dialog-item">
          • {path}
        </div>
      {/each}
      {#if itemsToRestore.length > 5}
        <div class="confirm-dialog-item more">
          ... y {itemsToRestore.length - 5} más
        </div>
      {/if}
    </div>
  {/if}
</ConfirmDialog>

{#if toastMessage}
  <div class="vault-toast">
    <Check size={14} class="vault-toast-icon" />
    <span>{toastMessage}</span>
  </div>
{/if}

{#snippet renderNode(node: VaultTreeNode, depth: number)}
  {#if node.isFolder}
    <Collapsible.Root
      open={!!expandedFolders[node.relativePath]}
      onOpenChange={() => toggleFolder(node.relativePath)}
    >
      <Collapsible.Trigger>
        {#snippet child({ props })}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            {...props}
            data-path={node.relativePath}
            class="file-tree-item folder"
            class:selected={isSelected(node.relativePath)}
            class:context-target={isContextMenuOpen && (contextMenuNode?.relativePath === node.relativePath || isSelected(node.relativePath))}
            style="padding-left: {12 + depth * 14}px;"
            onclick={(e) => handleFolderClick(e, node, (props as Record<string, any>).onclick)}
            oncontextmenu={(e) => handleItemContextMenu(e, node)}
          >
            <ChevronRight
              class="chevron-icon {expandedFolders[node.relativePath] ? 'expanded' : ''}"
              size={12}
            />
            <FolderIcon
              isOpen={!!expandedFolders[node.relativePath]}
              name={node.name}
              size={15}
            />
            <span class="file-name">{node.name}</span>
          </div>
        {/snippet}
      </Collapsible.Trigger>

      {#if expandedFolders[node.relativePath]}
        <Collapsible.Content>
          {#each getChildren(node) as child (child.relativePath)}
            {@render renderNode(child, depth + 1)}
          {/each}
        </Collapsible.Content>
      {/if}
    </Collapsible.Root>
  {:else}
    {@const rawGitStatus = isGitRepo && gitStatuses ? (gitStatuses[node.relativePath] ?? gitStatuses[node.relativePath.replace(/^\.\//, '')]) : undefined}
    {@const fileGitStatus = getGitStatusInfo(rawGitStatus)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      data-path={node.relativePath}
      class="file-tree-item file"
      class:active={node.relativePath === activeTabPath}
      class:selected={isSelected(node.relativePath)}
      class:context-target={isContextMenuOpen && (contextMenuNode?.relativePath === node.relativePath || isSelected(node.relativePath))}
      class:git-staged={fileGitStatus?.index && !fileGitStatus?.worktree}
      class:git-modified={fileGitStatus?.worktree === 'M'}
      class:git-untracked={fileGitStatus?.worktree === '?'}
      class:git-stashed={fileGitStatus?.is_stashed && !fileGitStatus?.index && !fileGitStatus?.worktree}
      style="padding-left: {26 + depth * 14}px;"
      onclick={(e) => handleFileClick(e, node)}
      oncontextmenu={(e) => handleItemContextMenu(e, node)}
    >
      <FileIcon
        path={node.relativePath}
        name={node.name}
        size={14}
        class="file-icon {fileGitStatus?.worktree === 'M' ? 'file-icon-modified' : fileGitStatus?.index ? 'file-icon-staged' : fileGitStatus?.worktree === '?' ? 'file-icon-untracked' : fileGitStatus?.is_stashed ? 'file-icon-stashed' : ''}"
      />
      <span class="file-name" title={node.name}>{node.name}</span>

      {#if fileGitStatus}
        <div class="git-badges-group">
          {#if fileGitStatus.index === 'M'}
            <span class="git-badge staged" title="Modificado y en stage (preparado)">M</span>
          {:else if fileGitStatus.index === 'A'}
            <span class="git-badge staged" title="Nuevo archivo en stage (preparado)">A</span>
          {/if}

          {#if fileGitStatus.worktree === 'M'}
            <span class="git-badge modified" title="Modificaciones locales sin preparar">M</span>
          {:else if fileGitStatus.worktree === '?'}
            <span class="git-badge untracked" title="No controlado por Git (incógnita)">?</span>
          {/if}

          {#if fileGitStatus.is_stashed}
            <span class="git-badge stashed" title="En Stash de Git">S</span>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

<style>
  .sidebar-panel {
    position: relative;
    height: 100%;
    background-color: var(--bg-secondary, #f6f8fa);
    border-right: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    flex-direction: column;
    user-select: none;
    flex-shrink: 0;
    z-index: 5;
  }

  .sidebar-panel:focus {
    outline: none;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #656d76);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    gap: 8px;
  }

  .sidebar-header-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
  }

  .git-branch-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    font-weight: 500;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text-secondary, #656d76);
    background: var(--badge-bg, rgba(0, 0, 0, 0.06));
    padding: 1px 5px;
    border-radius: 4px;
    max-width: 110px;
  }

  .git-branch-icon {
    width: 11px;
    height: 11px;
    flex-shrink: 0;
  }

  .git-branch-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rust-badge-btn {
    font-weight: 700;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    padding: 3px 6px;
    border-radius: 5px;
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .rust-badge-btn:hover {
    background: var(--accent, #0969da);
    color: #ffffff;
  }

  .sidebar-header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .sidebar-collapse-btn,
  .sidebar-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .sidebar-collapse-btn:hover,
  .sidebar-action-btn:hover:not(:disabled) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
    color: var(--text-primary, #1f2328);
  }

  .sidebar-action-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  @keyframes locate-pulse {
    0% {
      box-shadow: 0 0 0 0 rgba(9, 105, 218, 0.6);
      background-color: var(--accent-bg, rgba(9, 105, 218, 0.28));
    }
    50% {
      box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.35);
      background-color: var(--accent-bg, rgba(9, 105, 218, 0.38));
    }
    100% {
      box-shadow: 0 0 0 0 rgba(9, 105, 218, 0);
      background-color: var(--accent-bg, rgba(9, 105, 218, 0.14));
    }
  }

  :global(.file-tree-item.locate-highlight) {
    animation: locate-pulse 1.4s ease-out !important;
    outline: 1px solid var(--accent, #0969da);
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .file-tree-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.12s ease;
    border-radius: 4px;
    margin: 1px 4px;
  }

  .file-tree-item:hover {
    background-color: var(--hover-bg, rgba(0, 0, 0, 0.04));
    color: var(--text-primary, #1f2328);
  }

  .file-tree-item.folder {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .file-tree-item.active {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    font-weight: 500;
  }

  .file-tree-item.selected {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.14));
    color: var(--accent, #0969da);
  }

  .file-tree-item.selected:hover {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.2));
  }

  .sidebar-panel:focus .file-tree-item.selected,
  .sidebar-panel:focus-visible .file-tree-item.selected {
    outline: 1px solid var(--accent, #0969da);
    outline-offset: -1px;
  }

  .chevron-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    transition: transform 0.15s ease;
    color: var(--text-secondary, #656d76);
  }

  .chevron-icon.expanded {
    transform: rotate(90deg);
  }

  .folder-icon {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    color: var(--accent, #0969da);
  }

  .file-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-secondary, #656d76);
  }

  .file-icon-modified {
    color: #dc2626;
  }

  .file-icon-staged {
    color: #16a34a;
  }

  .file-icon-untracked {
    color: #ca8a04;
  }

  .file-icon-stashed {
    color: #7c3aed;
  }

  .file-tree-item.active .file-icon {
    color: var(--accent, #0969da);
  }

  .file-tree-item.git-modified:not(.active) .file-name {
    color: #dc2626;
  }

  .file-tree-item.git-staged:not(.active) .file-name {
    color: #16a34a;
  }

  .file-tree-item.git-untracked:not(.active) .file-name {
    color: #ca8a04;
  }

  .file-tree-item.git-stashed:not(.active) .file-name {
    color: #7c3aed;
  }

  .file-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-badges-group {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .git-badges-group .git-badge {
    margin-left: 0;
  }

  .git-badge {
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 3px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .git-badge.staged {
    color: #16a34a;
    background: rgba(22, 163, 74, 0.16);
  }

  .git-badge.modified {
    color: #dc2626;
    background: rgba(220, 38, 38, 0.16);
  }

  .git-badge.untracked {
    color: #ca8a04;
    background: rgba(202, 138, 4, 0.16);
  }

  .git-badge.stashed {
    color: #7c3aed;
    background: rgba(124, 58, 237, 0.16);
  }

  .sidebar-resizer {
    position: absolute;
    top: 0;
    right: -5px;
    width: 10px;
    height: 100%;
    cursor: col-resize;
    z-index: 50;
    user-select: none;
    touch-action: none;
  }

  .sidebar-resizer::after {
    content: '';
    position: absolute;
    top: 0;
    left: 4px;
    width: 2px;
    height: 100%;
    background-color: transparent;
    transition: background-color 0.15s ease;
  }

  .sidebar-resizer:hover::after,
  :global(body.is-resizing-col) .sidebar-resizer::after,
  :global(.sidebar-panel.is-resizing) .sidebar-resizer::after {
    background-color: var(--accent, #0969da);
  }

  .file-tree-item.context-target {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.12));
    color: var(--accent, #0969da);
  }

  /* Menú Contextual (Click Derecho con bits-ui) */
  :global(.vault-context-menu) {
    min-width: 220px;
    max-width: 320px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.16));
    padding: 4px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    user-select: none;
    outline: none;
    animation: context-menu-fade 0.1s ease-out;
  }

  @keyframes context-menu-fade {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  :global(.vault-context-menu .context-menu-header) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    min-width: 0;
  }

  :global(.vault-context-menu .context-menu-header-icon) {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    color: var(--text-secondary, #656d76);
  }

  :global(.vault-context-menu .context-menu-header-title) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-transform: none;
    letter-spacing: normal;
  }

  :global(.vault-context-menu .context-menu-divider) {
    height: 1px;
    background: var(--border-primary, #d0d7de);
    margin: 3px 0;
  }

  :global(.vault-context-menu .context-menu-item) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: none;
    background: transparent;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s ease, color 0.1s ease;
    font-family: inherit;
    outline: none;
    box-sizing: border-box;
  }

  :global(.vault-context-menu .context-menu-item:hover),
  :global(.vault-context-menu .context-menu-item[data-highlighted]) {
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
    color: var(--accent, #0969da);
  }

  :global(.vault-context-menu .context-menu-item-icon) {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  :global(.vault-context-menu .context-menu-item.delete) {
    color: #cf222e;
  }

  :global(.vault-context-menu .context-menu-item.delete:hover),
  :global(.vault-context-menu .context-menu-item.delete[data-highlighted]) {
    background: rgba(207, 34, 46, 0.1);
    color: #a40e26;
  }

  :global(.vault-context-menu .context-menu-shortcut) {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    opacity: 0.75;
  }



  /* Notificación Toast rápida */
  .vault-toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-tertiary, #1f2328);
    color: var(--text-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: var(--popover-shadow, 0 4px 14px rgba(0, 0, 0, 0.25));
    z-index: 10000;
    pointer-events: none;
    animation: toast-fade-in 0.15s ease-out;
  }

  .vault-toast-icon {
    width: 14px;
    height: 14px;
    color: #2da44e;
    flex-shrink: 0;
  }

  @keyframes toast-fade-in {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0);
    }
  }
</style>
