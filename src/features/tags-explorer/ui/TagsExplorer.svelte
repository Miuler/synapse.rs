<script lang="ts">
  import { onMount } from 'svelte';
  import { vaultApi, type TagCountItem } from '@entities/vault-item';
  import { Tag, Search, RefreshCw, FileText, ChevronRight, Hash } from 'lucide-svelte';

  interface Props {
    sidebarWidth: number;
    isResizingSidebar: boolean;
    onSelectNote: (notePath: string) => void;
    onResizeStart: (e: MouseEvent | PointerEvent) => void;
    onCollapse?: () => void;
  }

  let {
    sidebarWidth,
    isResizingSidebar,
    onSelectNote,
    onResizeStart,
    onCollapse
  }: Props = $props();

  let tags = $state<TagCountItem[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(false);
  let selectedTag = $state<string | null>(null);
  let matchingNotes = $state<string[]>([]);
  let isLoadingNotes = $state(false);

  async function loadTags() {
    isLoading = true;
    try {
      tags = await vaultApi.getVaultTags();
    } catch (err) {
      console.error('Error cargando tags del vault:', err);
    } finally {
      isLoading = false;
    }
  }

  async function handleSelectTag(tag: string) {
    if (selectedTag === tag) {
      selectedTag = null;
      matchingNotes = [];
      return;
    }
    selectedTag = tag;
    isLoadingNotes = true;
    try {
      matchingNotes = await vaultApi.getNotesByTag(tag);
    } catch (err) {
      console.error('Error cargando notas para el tag:', tag, err);
      matchingNotes = [];
    } finally {
      isLoadingNotes = false;
    }
  }

  let filteredTags = $derived(
    tags.filter((item) =>
      item.tag.toLowerCase().includes(searchQuery.trim().toLowerCase())
    )
  );

  onMount(() => {
    loadTags();
  });
</script>

<aside
  class="tags-explorer-sidebar"
  style="width: {sidebarWidth}px;"
>
  <div class="sidebar-header">
    <div class="header-title">
      <Tag size={16} class="header-icon" />
      <span>Etiquetas ({tags.length})</span>
    </div>
    <div class="header-actions">
      <button
        class="icon-btn"
        onclick={loadTags}
        title="Recargar etiquetas"
        disabled={isLoading}
      >
        <span class={isLoading ? 'spinning' : ''} style="display: inline-flex;">
          <RefreshCw size={13} />
        </span>
      </button>
    </div>
  </div>

  <div class="search-bar">
    <Search size={14} class="search-icon" />
    <input
      type="text"
      placeholder="Filtrar etiquetas..."
      bind:value={searchQuery}
      class="search-input"
    />
  </div>

  <div class="tags-container">
    {#if isLoading}
      <div class="empty-state">Cargando etiquetas...</div>
    {:else if filteredTags.length === 0}
      <div class="empty-state">
        {#if searchQuery.trim()}
          No se encontraron etiquetas para "{searchQuery}"
        {:else}
          No hay etiquetas (#tags) en tus notas todavía.
        {/if}
      </div>
    {:else}
      <div class="tag-list">
        {#each filteredTags as item}
          <div class="tag-group">
            <button
              class="tag-pill-btn"
              class:active={selectedTag === item.tag}
              onclick={() => handleSelectTag(item.tag)}
            >
              <div class="tag-left">
                <Hash size={12} class="hash-icon" />
                <span class="tag-name">{item.tag}</span>
              </div>
              <span class="tag-badge">{item.count}</span>
            </button>

            {#if selectedTag === item.tag}
              <div class="tag-notes-list">
                {#if isLoadingNotes}
                  <div class="note-loading">Cargando notas...</div>
                {:else if matchingNotes.length === 0}
                  <div class="note-loading">Sin notas vinculadas</div>
                {:else}
                  {#each matchingNotes as notePath}
                    <button
                      class="tag-note-item"
                      onclick={() => onSelectNote(notePath)}
                      title={notePath}
                    >
                      <FileText size={13} class="note-icon" />
                      <span class="note-name">{notePath.split('/').pop() || notePath}</span>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Resizer Handle -->
  <div
    role="separator"
    tabindex="-1"
    aria-label="Redimensionar panel de etiquetas"
    class="sidebar-resizer"
    class:is-resizing={isResizingSidebar}
    onpointerdown={onResizeStart}
  ></div>
</aside>

<style>
  .tags-explorer-sidebar {
    position: relative;
    height: 100%;
    background-color: var(--bg-secondary, #f6f8fa);
    border-right: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    flex-direction: column;
    user-select: none;
    overflow: hidden;
  }

  .sidebar-header {
    height: 38px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  :global(.header-icon) {
    color: var(--accent, #0969da);
  }

  .icon-btn {
    border: none;
    background: transparent;
    color: var(--text-secondary, #656d76);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .search-bar {
    position: relative;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
  }

  :global(.search-icon) {
    position: absolute;
    left: 18px;
    color: var(--text-muted, #8c959f);
  }

  .search-input {
    width: 100%;
    padding: 4px 8px 4px 26px;
    font-size: 12px;
    background-color: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-secondary, #d0d7de);
    border-radius: 6px;
    color: var(--text-primary, #1f2328);
    outline: none;
  }

  .search-input:focus {
    border-color: var(--accent, #0969da);
  }

  .tags-container {
    flex: 1;
    overflow-y: auto;
    padding: 8px 6px;
  }

  .empty-state {
    padding: 24px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-muted, #8c959f);
  }

  .tag-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tag-group {
    display: flex;
    flex-direction: column;
  }

  .tag-pill-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .tag-pill-btn:hover {
    background-color: var(--bg-tertiary, #eaeef2);
  }

  .tag-pill-btn.active {
    background-color: var(--bg-primary, #ffffff);
    border-color: var(--border-primary, #d0d7de);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  }

  .tag-left {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  :global(.hash-icon) {
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .tag-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tag-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 10px;
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-secondary, #656d76);
  }

  .tag-notes-list {
    margin: 4px 0 6px 16px;
    padding-left: 8px;
    border-left: 2px solid var(--border-secondary, #d0d7de);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .note-loading {
    font-size: 11px;
    color: var(--text-muted, #8c959f);
    padding: 4px;
  }

  .tag-note-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: 4px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-secondary, #656d76);
    transition: all 0.1s ease;
  }

  .tag-note-item:hover {
    background-color: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
  }

  :global(.note-icon) {
    flex-shrink: 0;
    color: var(--text-muted, #8c959f);
  }

  .note-name {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sidebar-resizer {
    position: absolute;
    top: 0;
    right: 0;
    width: 4px;
    height: 100%;
    cursor: col-resize;
    z-index: 10;
  }

  .sidebar-resizer:hover,
  .sidebar-resizer.is-resizing {
    background-color: var(--accent, #0969da);
  }
</style>
