<script lang="ts">
  import type { VaultItem } from '@entities/vault-item';
  import { Command, Dialog } from 'bits-ui';

  interface Props {
    isOpen?: boolean;
    vaultItems?: VaultItem[];
    recentFiles?: string[];
    onSelectFile?: (path: string) => void;
    onClose?: () => void;
  }

  let {
    isOpen = $bindable(false),
    vaultItems = [],
    recentFiles = [],
    onSelectFile,
    onClose
  }: Props = $props();

  let searchQuery = $state('');

  interface FileItemDisplay {
    id: string;
    title: string;
    path: string;
    isRecent?: boolean;
  }

  // Prepara los archivos de la bóveda para la lista
  let fileList = $derived.by<FileItemDisplay[]>(() => {
    const list: FileItemDisplay[] = [];
    const addedPaths = new Set<string>();

    for (const path of recentFiles) {
      if (!path || path.startsWith('empty:') || addedPaths.has(path)) continue;
      const item = vaultItems.find((v) => v.relative_path === path);
      list.push({
        id: path,
        title: item ? item.title : (path.split('/').pop()?.replace(/\.[^/.]+$/, '') || path),
        path: path,
        isRecent: true,
      });
      addedPaths.add(path);
    }

    for (const item of vaultItems) {
      if (!item.relative_path || item.relative_path.startsWith('empty:') || addedPaths.has(item.relative_path)) continue;
      list.push({
        id: item.relative_path,
        title: item.title,
        path: item.relative_path,
        isRecent: false,
      });
      addedPaths.add(item.relative_path);
    }

    return list;
  });

  // Atajo global Ctrl+O o Cmd+O para abrir/cerrar el diálogo de archivos
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'o' || e.key === 'O')) {
        e.preventDefault();
        isOpen = !isOpen;
        if (isOpen) {
          searchQuery = '';
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  function closeDialog() {
    isOpen = false;
    searchQuery = '';
    if (onClose) onClose();
  }

  function selectFile(path: string) {
    if (onSelectFile) onSelectFile(path);
    closeDialog();
  }
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(open) => {
    isOpen = open;
    if (!open) {
      closeDialog();
    }
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="palette-backdrop" />
    <Dialog.Content class="palette-container">
      <Dialog.Title class="sr-only">Buscador Rápido de Archivos</Dialog.Title>
      <Command.Root class="command-root" loop>
        <div class="input-wrapper">
          <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <Command.Input
            class="command-input"
            bind:value={searchQuery}
            placeholder="Buscar archivo por nombre... (Ctrl+O)"
          />
          <span class="esc-badge">ESC</span>
        </div>

        <Command.List class="results-container">
          <Command.Empty class="empty-state">No se encontraron archivos</Command.Empty>

          {#each fileList as file (file.id)}
            <Command.Item
              class="palette-item"
              value={`${file.title} ${file.path}`}
              onSelect={() => selectFile(file.path)}
            >
              <span class="category-tag file-tag">
                {file.isRecent && !searchQuery.trim() ? 'RECIENTE' : 'ARCHIVO'}
              </span>
              <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
              </svg>
              <span class="item-name">{file.title}</span>
              <span class="item-path">{file.path}</span>
            </Command.Item>
          {/each}
        </Command.List>

        <footer class="palette-footer">
          <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
          <span><kbd>↵</kbd> Abrir</span>
          <span><kbd>esc</kbd> Cerrar</span>
        </footer>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  :global(.palette-backdrop) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(10, 12, 16, 0.7);
    backdrop-filter: blur(10px);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.palette-container) {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    width: 620px;
    max-width: 90%;
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 1001;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  :global(.palette-container .command-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
    outline: none;
  }

  :global(.palette-container .input-wrapper) {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
  }

  :global(.palette-container .search-icon) {
    width: 18px;
    height: 18px;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    flex-shrink: 0;
  }

  :global(.palette-container .command-input) {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  :global(.palette-container .command-input::placeholder) {
    color: var(--text-secondary, #656d76);
  }

  :global(.palette-container .esc-badge) {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.palette-container .results-container) {
    max-height: 340px;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.palette-container .empty-state) {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  :global(.palette-container .palette-item) {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    transition: background-color 0.1s ease, color 0.1s ease;
    user-select: none;
    outline: none;
  }

  :global(.palette-container .palette-item:hover),
  :global(.palette-container .palette-item[data-selected]),
  :global(.palette-container .palette-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  :global(.palette-container .palette-item[data-selected] .category-tag),
  :global(.palette-container .palette-item[data-highlighted] .category-tag) {
    color: var(--accent, #0969da);
  }

  :global(.palette-container .category-tag.file-tag) {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.08);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    min-width: unset;
    margin-right: 10px;
  }

  :global(.palette-container .file-icon) {
    width: 16px;
    height: 16px;
    margin-right: 10px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.palette-container .item-name) {
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    margin-right: 8px;
    white-space: nowrap;
  }

  :global(.palette-container .item-path) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.75;
  }

  :global(.palette-container .palette-footer) {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 16px;
    background-color: rgba(0, 0, 0, 0.03);
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  :global(.palette-container .palette-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }
</style>
