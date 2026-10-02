<script lang="ts">
  import type { VaultItem } from '@entities/vault-item';
  import { vaultRepository, type GitFileStatusKind } from '@shared/repositories';
  import { AlertDialog, ContextMenu, Collapsible } from 'bits-ui';
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

  // Estado de selección múltiple
  let selectedPaths = $state<string[]>([]);
  let lastFocusedPath = $state<string | null>(null);

  function isSelected(path: string): boolean {
    return selectedPaths.includes(path);
  }

  // Sincronizar selección inicial con la pestaña activa si no hay selección
  $effect(() => {
    if (activeTabPath && selectedPaths.length === 0) {
      selectedPaths = [activeTabPath];
      lastFocusedPath = activeTabPath;
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

  // Alternar selección individual con Ctrl/Cmd
  function toggleItemSelection(path: string) {
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter((p) => p !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
    lastFocusedPath = path;
  }

  // Selección de rango con Shift
  function selectRange(startPath: string, endPath: string) {
    const visible = getVisibleNodes(treeNodes);
    const startIdx = visible.findIndex((n) => n.relativePath === startPath);
    const endIdx = visible.findIndex((n) => n.relativePath === endPath);

    if (startIdx === -1 || endIdx === -1) {
      selectedPaths = [endPath];
      lastFocusedPath = endPath;
      return;
    }

    const min = Math.min(startIdx, endIdx);
    const max = Math.max(startIdx, endIdx);
    const range = visible.slice(min, max + 1).map((n) => n.relativePath);
    selectedPaths = Array.from(new Set([...selectedPaths, ...range]));
    lastFocusedPath = endPath;
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
    if (e.metaKey || e.ctrlKey) {
      e.preventDefault();
      e.stopPropagation();
      toggleItemSelection(node.relativePath);
      return;
    }
    if (e.shiftKey && lastFocusedPath) {
      e.preventDefault();
      e.stopPropagation();
      selectRange(lastFocusedPath, node.relativePath);
      return;
    }
    selectedPaths = [node.relativePath];
    lastFocusedPath = node.relativePath;
    defaultTriggerClick?.(e);
  }

  function handleFileClick(e: MouseEvent, node: VaultTreeNode) {
    if (e.metaKey || e.ctrlKey) {
      e.stopPropagation();
      toggleItemSelection(node.relativePath);
      return;
    }
    if (e.shiftKey && lastFocusedPath) {
      e.stopPropagation();
      selectRange(lastFocusedPath, node.relativePath);
      return;
    }
    selectedPaths = [node.relativePath];
    lastFocusedPath = node.relativePath;
    onSelectTab(node.relativePath);
  }

  function handleItemContextMenu(e: MouseEvent, node: VaultTreeNode) {
    // Si el elemento no está en la selección actual, aislar la selección a este elemento
    if (!selectedPaths.includes(node.relativePath)) {
      selectedPaths = [node.relativePath];
      lastFocusedPath = node.relativePath;
    }
    contextMenuNode = node;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Delete' || (e.key === 'Backspace' && (e.metaKey || e.ctrlKey))) {
      if ((selectedNodes.length > 0 || contextMenuNode) && itemsToDelete.length === 0) {
        e.preventDefault();
        handlePromptDelete();
      }
    } else if (e.key === 'Escape') {
      selectedPaths = [];
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'a' || e.key === 'A')) {
      const target = e.target as HTMLElement;
      if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
        e.preventDefault();
        const visible = getVisibleNodes(treeNodes);
        selectedPaths = visible.map((n) => n.relativePath);
      }
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 'c' || e.key === 'C')) {
      const target = e.target as HTMLElement;
      if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
        e.preventDefault();
        handleCopy();
      }
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

  function handlePromptDelete() {
    if (selectedNodes.length > 0) {
      itemsToDelete = selectedNodes.map((n) => ({
        name: n.name,
        relativePath: n.relativePath,
        isFolder: n.isFolder,
      }));
    } else if (contextMenuNode) {
      itemsToDelete = [
        {
          name: contextMenuNode.name,
          relativePath: contextMenuNode.relativePath,
          isFolder: contextMenuNode.isFolder,
        },
      ];
    }
  }

  function cancelDelete() {
    if (isDeleting) return;
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
              <span
                class="sidebar-title"
                title={activeRibbonTab === 'files' ? (vaultPath || 'Bóveda de Archivos') : 'Buscar'}
              >
                {activeRibbonTab === 'files' ? vaultFolderName : 'Buscar'}
              </span>
              {#if isGitRepo && gitBranch}
                <span class="git-branch-badge" title="Rama Git: {gitBranch}">
                  <GitBranch size={13} class="git-branch-icon" />
                  <span class="git-branch-name">{gitBranch}</span>
                </span>
              {/if}
            </div>
            <div class="sidebar-header-actions">
              {#if isConnectedToRust}
                <button
                  type="button"
                  class="rust-badge-btn"
                  onclick={onOpenVaultFolder}
                  title="Abrir carpeta / bóveda en disco"
                >
                  RUST
                </button>
              {/if}
              {#if onCollapse}
                <button
                  type="button"
                  class="sidebar-collapse-btn"
                  onclick={onCollapse}
                  title="Colapsar panel lateral (Ctrl+B)"
                  aria-label="Colapsar panel lateral"
                >
                  <PanelLeftClose size={14} />
                </button>
              {/if}
            </div>
          </div>

          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="sidebar-content"
            onclick={(e) => {
              if (!(e.target as HTMLElement).closest('.file-tree-item')) {
                selectedPaths = [];
                lastFocusedPath = null;
              }
            }}
            oncontextmenu={(e) => {
              if (!(e.target as HTMLElement).closest('.file-tree-item')) {
                contextMenuNode = null;
                selectedPaths = [];
                lastFocusedPath = null;
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

<AlertDialog.Root
  open={itemsToDelete.length > 0}
  onOpenChange={(open) => {
    if (!open && !isDeleting) {
      cancelDelete();
    }
  }}
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="delete-modal-overlay" />
    <AlertDialog.Content class="delete-modal">
      <div class="delete-modal-header">
        <div class="delete-modal-icon-wrap">
          <Trash2 size={24} class="delete-modal-icon" />
        </div>
        <div class="delete-modal-text">
          <AlertDialog.Title class="delete-modal-title">
            {#if itemsToDelete.length > 1}
              ¿Eliminar {itemsToDelete.length} elementos?
            {:else}
              ¿Eliminar {itemsToDelete[0]?.isFolder ? 'carpeta' : 'archivo'}?
            {/if}
          </AlertDialog.Title>
          <AlertDialog.Description class="delete-modal-desc">
            {#if itemsToDelete.length > 1}
              ¿Estás seguro de que deseas eliminar permanentemente estos <strong>{itemsToDelete.length}</strong> elementos y todo su contenido? Esta acción no se puede deshacer.
              <div class="delete-modal-item-list">
                {#each itemsToDelete.slice(0, 5) as item}
                  <div class="delete-modal-item">
                    • {item.name} {item.isFolder ? '(carpeta)' : ''}
                  </div>
                {/each}
                {#if itemsToDelete.length > 5}
                  <div class="delete-modal-item more">
                    ... y {itemsToDelete.length - 5} más
                  </div>
                {/if}
              </div>
            {:else if itemsToDelete.length === 1}
              ¿Estás seguro de que deseas eliminar permanentemente <strong>{itemsToDelete[0]?.name}</strong>{itemsToDelete[0]?.isFolder ? ' y todo su contenido' : ''}? Esta acción no se puede deshacer.
            {/if}
          </AlertDialog.Description>
        </div>
      </div>
      <div class="delete-modal-actions">
        <AlertDialog.Cancel
          class="delete-modal-btn cancel"
          disabled={isDeleting}
        >
          Cancelar
        </AlertDialog.Cancel>
        <AlertDialog.Action
          class="delete-modal-btn confirm"
          onclick={(e) => {
            e.preventDefault();
            confirmDelete();
          }}
          disabled={isDeleting}
        >
          {isDeleting ? 'Eliminando...' : 'Eliminar'}
        </AlertDialog.Action>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

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

  .sidebar-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    font-size: 9px;
    font-weight: 700;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    padding: 2px 6px;
    border-radius: 4px;
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

  /* Diálogo Modal de Confirmación de Borrado con bits-ui AlertDialog */
  :global(.delete-modal-overlay) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10001;
    animation: overlay-fade-in 0.15s ease-out;
  }

  @keyframes overlay-fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.delete-modal) {
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.2);
    width: 90%;
    max-width: 420px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: modal-scale-in 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 10002;
    outline: none;
    box-sizing: border-box;
  }

  @keyframes modal-scale-in {
    from {
      opacity: 0;
      transform: translate(-50%, -50%) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  :global(.delete-modal .delete-modal-header) {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }

  :global(.delete-modal .delete-modal-icon-wrap) {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(207, 34, 46, 0.12);
    color: #cf222e;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  :global(.delete-modal .delete-modal-icon) {
    width: 18px;
    height: 18px;
  }

  :global(.delete-modal .delete-modal-text) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.delete-modal .delete-modal-title) {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
    line-height: 1.25;
  }

  :global(.delete-modal .delete-modal-desc) {
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    line-height: 1.45;
    margin: 0;
    word-break: break-word;
  }

  :global(.delete-modal .delete-modal-desc strong) {
    color: var(--text-primary, #1f2328);
  }

  :global(.delete-modal .delete-modal-item-list) {
    margin-top: 10px;
    padding: 8px 12px;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    max-height: 120px;
    overflow-y: auto;
    font-size: 12px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  :global(.delete-modal .delete-modal-item) {
    color: var(--text-primary, #1f2328);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.delete-modal .delete-modal-item.more) {
    color: var(--text-secondary, #656d76);
    font-style: italic;
  }

  :global(.delete-modal .delete-modal-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }

  :global(.delete-modal .delete-modal-btn) {
    padding: 7px 14px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
    font-family: inherit;
    outline: none;
  }

  :global(.delete-modal .delete-modal-btn.cancel) {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #d0d7de);
  }

  :global(.delete-modal .delete-modal-btn.cancel:hover:not(:disabled)) {
    background: var(--border-primary, #d0d7de);
  }

  :global(.delete-modal .delete-modal-btn.confirm) {
    background: #cf222e;
    color: #ffffff;
    border-color: #a40e26;
  }

  :global(.delete-modal .delete-modal-btn.confirm:hover:not(:disabled)) {
    background: #a40e26;
  }

  :global(.delete-modal .delete-modal-btn:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
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
