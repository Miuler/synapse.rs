<script lang="ts">
  import { tick } from 'svelte';
  import type { VaultItem } from '@entities/vault-item';
  import { vaultRepository, type GitFileStatusKind } from '@shared/repositories';
  import { ContextMenu, Collapsible } from 'bits-ui';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import { FileIcon, FolderIcon } from '@shared/ui/icons';
  import { GitBranch, Link, Copy, Trash2, Check, ChevronRight, PanelLeftClose, Files } from 'lucide-svelte';

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
  }: Props = $props();

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

  function toggleFolder(folderPath: string) {
    expandedFolders[folderPath] = !expandedFolders[folderPath];
  }

  // Construir el árbol jerárquico a partir de la lista plana de vaultItems
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

  // Expandir carpetas padres automáticamente cuando un archivo se selecciona como pestaña activa
  $effect(() => {
    const path = activeTabPath;
    if (path && path.includes('/')) {
      const parts = path.split('/').filter(Boolean);
      let currentPath = '';
      for (let i = 0; i < parts.length - 1; i++) {
        currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];
        expandedFolders[currentPath] = true;
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

  // Sincronizar selección inicial con la pestaña activa si no hay selección
  $effect(() => {
    if (activeTabPath && selectedPaths.length === 0) {
      selectedPaths = [activeTabPath];
      lastFocusedPath = activeTabPath;
      anchorPath = activeTabPath;
    }
  });

  // Obtener la lista aplanada de nodos visibles actualmente en el árbol
  function getVisibleNodes(nodes: VaultTreeNode[]): VaultTreeNode[] {
    const visible: VaultTreeNode[] = [];
    for (const node of nodes) {
      visible.push(node);
      if (node.isFolder && expandedFolders[node.relativePath]) {
        visible.push(...getVisibleNodes(node.children));
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
    const visible = getVisibleNodes(treeNodes);
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
    const visible = getVisibleNodes(treeNodes);
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
        if (n.isFolder) collect(n.children);
      }
    }
    collect(treeNodes);
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
        const visible = getVisibleNodes(treeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node && node.isFolder) {
          e.preventDefault();
          if (!expandedFolders[node.relativePath]) {
            expandedFolders[node.relativePath] = true;
          } else if (node.children.length > 0) {
            const firstChild = node.children[0];
            selectedPaths = [firstChild.relativePath];
            lastFocusedPath = firstChild.relativePath;
            anchorPath = firstChild.relativePath;
            scrollPathIntoView(firstChild.relativePath);
          }
        }
      }
    } else if (e.key === 'ArrowLeft') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(treeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node && node.isFolder && expandedFolders[node.relativePath]) {
          e.preventDefault();
          expandedFolders[node.relativePath] = false;
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
        const visible = getVisibleNodes(treeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node) {
          e.preventDefault();
          handleActivateNode(node);
        }
      }
    } else if (e.key === ' ') {
      if (lastFocusedPath) {
        const visible = getVisibleNodes(treeNodes);
        const node = visible.find((n) => n.relativePath === lastFocusedPath);
        if (node) {
          e.preventDefault();
          handleActivateNode(node);
        }
      }
    } else if (e.key === 'Home') {
      const visible = getVisibleNodes(treeNodes);
      if (visible.length > 0) {
        e.preventDefault();
        const first = visible[0];
        selectedPaths = [first.relativePath];
        lastFocusedPath = first.relativePath;
        anchorPath = first.relativePath;
        scrollPathIntoView(first.relativePath);
      }
    } else if (e.key === 'End') {
      const visible = getVisibleNodes(treeNodes);
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
      const visible = getVisibleNodes(treeNodes);
      selectedPaths = visible.map((n) => n.relativePath);
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'c' || e.key === 'C')) {
      e.preventDefault();
      handleCopy();
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
                <span
                  class="git-branch-badge"
                  title="Rama Git: {gitBranch}"
                  >
                    <GitBranch size={13} class="git-branch-icon" />
                    <span class="git-branch-name">{gitBranch}</span>
                </span>
              {/if}
            </div>
            <div class="sidebar-header-actions">
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
            {#each treeNodes as rootNode (rootNode.relativePath)}
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

      <Collapsible.Content>
        {#each node.children as child (child.relativePath)}
          {@render renderNode(child, depth + 1)}
        {/each}
      </Collapsible.Content>
    </Collapsible.Root>
  {:else}
    {@const fileGitStatus = isGitRepo && gitStatuses ? gitStatuses[node.relativePath] : undefined}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      data-path={node.relativePath}
      class="file-tree-item file"
      class:active={node.relativePath === activeTabPath}
      class:selected={isSelected(node.relativePath)}
      class:context-target={isContextMenuOpen && (contextMenuNode?.relativePath === node.relativePath || isSelected(node.relativePath))}
      class:git-modified={fileGitStatus === 'modified'}
      class:git-untracked={fileGitStatus === 'untracked'}
      style="padding-left: {26 + depth * 14}px;"
      onclick={(e) => handleFileClick(e, node)}
      oncontextmenu={(e) => handleItemContextMenu(e, node)}
    >
      <FileIcon
        path={node.relativePath}
        name={node.name}
        size={14}
        class="file-icon {fileGitStatus === 'modified' ? 'file-icon-modified' : ''}"
      />
      <span class="file-name" title={node.name}>{node.name}</span>

      {#if fileGitStatus === 'modified'}
        <span class="git-badge modified" title="Modificado en Git">M</span>
      {:else if fileGitStatus === 'untracked'}
        <span class="git-badge untracked" title="No controlado por Git">?</span>
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
    background: rgba(0, 0, 0, 0.06);
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

  .sidebar-collapse-btn {
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

  .sidebar-collapse-btn:hover {
    background: rgba(0, 0, 0, 0.06);
    color: var(--text-primary, #1f2328);
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
    background-color: rgba(0, 0, 0, 0.04);
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
    color: #d97706;
  }

  .file-tree-item.active .file-icon {
    color: var(--accent, #0969da);
  }

  .file-tree-item.git-modified:not(.active) .file-name {
    color: #b45309;
  }

  .file-tree-item.git-untracked:not(.active) .file-name {
    color: #15803d;
  }

  .file-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .git-badge.modified {
    color: #b45309;
    background: rgba(217, 119, 6, 0.14);
  }

  .git-badge.untracked {
    color: #15803d;
    background: rgba(22, 163, 74, 0.14);
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
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.16);
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
    background: var(--text-primary, #1f2328);
    color: var(--bg-primary, #ffffff);
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
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
