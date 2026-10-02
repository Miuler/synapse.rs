<script lang="ts">
  import type { VaultItem } from '@entities/vault-item';
  import { vaultRepository, type GitFileStatusKind } from '@shared/repositories';
  import { AlertDialog, ContextMenu, Collapsible } from 'bits-ui';

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
    onResizeStart: (e: PointerEvent) => void;
    onResizeMove: (e: PointerEvent) => void;
    onResizeEnd: (e: PointerEvent) => void;
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
    onResizeStart,
    onResizeMove,
    onResizeEnd,
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

  async function handleCopyRelativePath() {
    const relPath = contextMenuNode ? contextMenuNode.relativePath : '.';
    const ok = await copyTextToClipboard(relPath);
    if (ok) {
      showToast('Ruta relativa copiada al portapapeles');
    }
  }

  async function handleCopyFullPath() {
    const relPath = contextMenuNode ? contextMenuNode.relativePath : '.';
    const itemAbsPath = contextMenuNode?.item?.abs_path;
    const fullPath = getFullAbsolutePath(relPath, itemAbsPath);
    const ok = await copyTextToClipboard(fullPath);
    if (ok) {
      showToast('Ruta completa copiada al portapapeles');
    }
  }

  // Estado del diálogo de confirmación de borrado
  interface ItemToDelete {
    name: string;
    relativePath: string;
    isFolder: boolean;
  }

  let itemToDelete = $state<ItemToDelete | null>(null);
  let isDeleting = $state(false);

  function handlePromptDelete() {
    if (!contextMenuNode) return;
    const target: ItemToDelete = {
      name: contextMenuNode.name,
      relativePath: contextMenuNode.relativePath,
      isFolder: contextMenuNode.isFolder,
    };
    itemToDelete = target;
  }

  function cancelDelete() {
    if (isDeleting) return;
    itemToDelete = null;
  }

  async function confirmDelete() {
    if (!itemToDelete || isDeleting) return;
    const target = itemToDelete;
    isDeleting = true;
    try {
      if (onDeleteItem) {
        await onDeleteItem(target.relativePath, target.isFolder);
      } else {
        await vaultRepository.deleteItem(target.relativePath);
      }
      showToast(`"${target.name}" ha sido eliminado`);
      itemToDelete = null;
    } catch (err: unknown) {
      console.error('Error al borrar elemento:', err);
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
        <aside
          class="sidebar-panel"
          class:is-resizing={isResizingSidebar}
          style="width: {sidebarWidth}px;"
          {...props}
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
                  <svg class="git-branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="6" y1="3" x2="6" y2="15" />
                    <circle cx="18" cy="6" r="3" />
                    <circle cx="6" cy="18" r="3" />
                    <path d="M18 9a9 9 0 0 1-9 9" />
                  </svg>
                  <span class="git-branch-name">{gitBranch}</span>
                </span>
              {/if}
            </div>
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
          </div>

          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="sidebar-content"
            oncontextmenu={(e) => {
              if (!(e.target as HTMLElement).closest('.file-tree-item')) {
                contextMenuNode = null;
              }
            }}
          >
            {#each treeNodes as rootNode (rootNode.relativePath)}
              {@render renderNode(rootNode, 0)}
            {/each}
          </div>

          <!-- Tirador para redimensionar el panel lateral -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="sidebar-resizer"
            onpointerdown={onResizeStart}
            onpointermove={onResizeMove}
            onpointerup={onResizeEnd}
            onpointercancel={onResizeEnd}
          ></div>
        </aside>
      {/snippet}
    </ContextMenu.Trigger>

    <ContextMenu.Portal>
      <ContextMenu.Content class="vault-context-menu">
        <div class="context-menu-header" title={contextMenuNode ? contextMenuNode.relativePath : (vaultPath || 'Bóveda')}>
          {#if contextMenuNode?.isFolder}
            <svg class="context-menu-header-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z" />
            </svg>
          {:else if contextMenuNode}
            <svg class="context-menu-header-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          {:else}
            <svg class="context-menu-header-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
          {/if}
          <span class="context-menu-header-title">
            {contextMenuNode ? contextMenuNode.name : (vaultFolderName || 'Bóveda')}
          </span>
        </div>

        <ContextMenu.Separator class="context-menu-divider" />

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleCopyRelativePath}
        >
          <svg class="context-menu-item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
          </svg>
          <span>Copiar ruta relativa</span>
        </ContextMenu.Item>

        <ContextMenu.Item
          class="context-menu-item"
          onSelect={handleCopyFullPath}
        >
          <svg class="context-menu-item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
          <span>Copiar ruta completa</span>
        </ContextMenu.Item>

        {#if contextMenuNode}
          <ContextMenu.Separator class="context-menu-divider" />

          <ContextMenu.Item
            class="context-menu-item delete"
            onSelect={handlePromptDelete}
          >
            <svg class="context-menu-item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              <line x1="10" y1="11" x2="10" y2="17" />
              <line x1="14" y1="11" x2="14" y2="17" />
            </svg>
            <span>Borrar</span>
          </ContextMenu.Item>
        {/if}
      </ContextMenu.Content>
    </ContextMenu.Portal>
  </ContextMenu.Root>
{/if}

<AlertDialog.Root
  open={!!itemToDelete}
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
          <svg class="delete-modal-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <line x1="10" y1="11" x2="10" y2="17" />
            <line x1="14" y1="11" x2="14" y2="17" />
          </svg>
        </div>
        <div class="delete-modal-text">
          <AlertDialog.Title class="delete-modal-title">
            ¿Eliminar {itemToDelete?.isFolder ? 'carpeta' : 'archivo'}?
          </AlertDialog.Title>
          <AlertDialog.Description class="delete-modal-desc">
            ¿Estás seguro de que deseas eliminar permanentemente <strong>{itemToDelete?.name}</strong>{itemToDelete?.isFolder ? ' y todo su contenido' : ''}? Esta acción no se puede deshacer.
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
    <svg class="vault-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="20 6 9 17 4 12" />
    </svg>
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
            class:context-target={isContextMenuOpen && contextMenuNode?.relativePath === node.relativePath}
            style="padding-left: {12 + depth * 14}px;"
            oncontextmenu={() => { contextMenuNode = node; }}
          >
            <svg
              class="chevron-icon"
              class:expanded={expandedFolders[node.relativePath]}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
            <svg
              class="folder-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              {#if expandedFolders[node.relativePath]}
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
              {:else}
                <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z" />
              {/if}
            </svg>
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
      class:context-target={isContextMenuOpen && contextMenuNode?.relativePath === node.relativePath}
      class:git-modified={fileGitStatus === 'modified'}
      class:git-untracked={fileGitStatus === 'untracked'}
      style="padding-left: {26 + depth * 14}px;"
      onclick={() => onSelectTab(node.relativePath)}
      oncontextmenu={() => { contextMenuNode = node; }}
    >
      {#if fileGitStatus === 'modified'}
        <svg
          class="file-icon file-icon-modified"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <circle cx="16" cy="16" r="2.5" fill="currentColor" />
        </svg>
      {:else}
        <svg
          class="file-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
      {/if}
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
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    z-index: 20;
    user-select: none;
    touch-action: none;
    transition: background-color 0.15s ease;
  }

  .sidebar-resizer:hover,
  .sidebar-panel.is-resizing .sidebar-resizer {
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
