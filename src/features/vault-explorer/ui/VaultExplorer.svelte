<script lang="ts">
  import type { VaultItem } from '@entities/vault-item';

  interface Props {
    activeRibbonTab: string;
    sidebarWidth: number;
    isResizingSidebar: boolean;
    isConnectedToRust: boolean;
    vaultItems: VaultItem[];
    activeTabPath: string | null;
    onSelectTab: (path: string) => void;
    onOpenVaultFolder: () => void;
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
    onSelectTab,
    onOpenVaultFolder,
    onResizeStart,
    onResizeMove,
    onResizeEnd,
  }: Props = $props();
</script>

{#if activeRibbonTab === 'files' || activeRibbonTab === 'search'}
  <aside
    class="sidebar-panel"
    class:is-resizing={isResizingSidebar}
    style="width: {sidebarWidth}px;"
  >
    <div class="sidebar-header">
      <span>{activeRibbonTab === 'files' ? 'Bóveda de Notas' : 'Buscar'}</span>
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
    <div class="sidebar-content">
      {#each vaultItems as item}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="file-tree-item"
          class:active={item.relative_path === activeTabPath}
          onclick={() => onSelectTab(item.relative_path)}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path
              d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
            />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <span class="file-name">{item.relative_path || `${item.title}.md`}</span>
        </div>
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
{/if}

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
    gap: 8px;
    padding: 6px 14px;
    font-size: 13px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .file-tree-item:hover {
    background-color: rgba(0, 0, 0, 0.04);
    color: var(--text-primary, #1f2328);
  }

  .file-tree-item.active {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    font-weight: 500;
  }

  .file-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
</style>
