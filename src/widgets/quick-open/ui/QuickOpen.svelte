<script lang="ts">
  import { tick } from 'svelte';
  import type { VaultItem } from '@entities/vault-item';

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
  let selectedIndex = $state(0);
  let inputElement = $state<HTMLInputElement | null>(null);
  let resultsContainerRef = $state<HTMLDivElement | null>(null);
  let resultsListRef = $state<HTMLUListElement | null>(null);

  interface FileItemDisplay {
    id: string;
    title: string;
    path: string;
    isRecent?: boolean;
  }

  // Lista de archivos calculada reactivamente:
  // - Si searchQuery está vacío: muestra los últimos abiertos por defecto (recientes)
  // - Si se escribe texto: busca en todos los archivos de la bóveda que coincidan en el nombre
  let displayedFiles = $derived.by<FileItemDisplay[]>(() => {
    const q = searchQuery.trim().toLowerCase();

    if (!q) {
      const list: FileItemDisplay[] = [];
      const addedPaths = new Set<string>();

      // 1. Mostrar primero los archivos abiertos recientemente
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

      // 2. Completar con el resto de archivos de la bóveda
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
    }

    // Filtrar archivos que coincidan en su nombre o ruta con la búsqueda
    const matches: { item: FileItemDisplay; score: number }[] = [];

    for (const item of vaultItems) {
      if (!item.relative_path || item.relative_path.startsWith('empty:')) continue;
      const titleLower = (item.title || '').toLowerCase();
      const pathLower = (item.relative_path || '').toLowerCase();

      if (titleLower.includes(q) || pathLower.includes(q)) {
        let score = 0;
        if (titleLower === q) score += 100;
        else if (titleLower.startsWith(q)) score += 50;
        else if (pathLower.startsWith(q)) score += 30;
        else if (titleLower.includes(q)) score += 20;
        else score += 10;

        matches.push({
          item: {
            id: item.relative_path,
            title: item.title,
            path: item.relative_path,
            isRecent: recentFiles.includes(item.relative_path),
          },
          score,
        });
      }
    }

    matches.sort((a, b) => b.score - a.score);
    return matches.map((m) => m.item);
  });

  // Ajustar selectedIndex reactivamente
  $effect(() => {
    if (displayedFiles.length > 0) {
      if (selectedIndex >= displayedFiles.length || selectedIndex < 0) {
        selectedIndex = 0;
      }
    } else {
      selectedIndex = 0;
    }
  });

  // Atajo global Ctrl+O o Cmd+O para abrir/cerrar el diálogo de archivos
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'o' || e.key === 'O')) {
        e.preventDefault();
        isOpen = !isOpen;
        if (isOpen) {
          resetState();
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  // Enfocar input automáticamente al abrir
  $effect(() => {
    if (isOpen) {
      resetState();
      setTimeout(() => inputElement?.focus(), 40);
    }
  });

  function resetState() {
    selectedIndex = 0;
    searchQuery = '';
  }

  function closeDialog() {
    isOpen = false;
    if (onClose) onClose();
  }

  function selectFile(path: string) {
    if (onSelectFile) onSelectFile(path);
    closeDialog();
  }

  function getPageSize(): number {
    if (resultsContainerRef && resultsListRef) {
      const firstItem = resultsListRef.querySelector('.palette-item') as HTMLElement | null;
      if (firstItem && firstItem.offsetHeight > 0) {
        return Math.max(1, Math.floor(resultsContainerRef.clientHeight / firstItem.offsetHeight));
      }
    }
    return 7;
  }

  function scrollSelectedIntoView(blockMode: ScrollLogicalPosition = 'nearest') {
    if (!resultsListRef) return;
    const selectedEl = resultsListRef.querySelector('.palette-item.selected') as HTMLElement | null;
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: blockMode });
    }
  }

  function handleInputKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        selectedIndex = (selectedIndex + 1) % displayedFiles.length;
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        selectedIndex = (selectedIndex - 1 + displayedFiles.length) % displayedFiles.length;
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        selectedIndex = 0;
        tick().then(() => {
          if (resultsContainerRef) resultsContainerRef.scrollTop = 0;
          scrollSelectedIntoView('start');
        });
      }
    } else if (e.key === 'End') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        selectedIndex = displayedFiles.length - 1;
        tick().then(() => {
          if (resultsContainerRef) resultsContainerRef.scrollTop = resultsContainerRef.scrollHeight;
          scrollSelectedIntoView('end');
        });
      }
    } else if (e.key === 'PageDown') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        const pageSize = getPageSize();
        selectedIndex = Math.min(displayedFiles.length - 1, selectedIndex + pageSize);
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'PageUp') {
      e.preventDefault();
      if (displayedFiles.length > 0) {
        const pageSize = getPageSize();
        selectedIndex = Math.max(0, selectedIndex - pageSize);
        tick().then(() => scrollSelectedIntoView('nearest'));
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (displayedFiles[selectedIndex]) {
        selectFile(displayedFiles[selectedIndex].path);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeDialog();
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="backdrop" onclick={closeDialog}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="palette-container" onclick={(e) => e.stopPropagation()}>
      <div class="input-wrapper">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={inputElement}
          bind:value={searchQuery}
          onkeydown={handleInputKeydown}
          placeholder="Buscar archivo por nombre... (Ctrl+O)"
          type="text"
        />
        <span class="esc-badge">ESC</span>
      </div>

      <div class="results-container" bind:this={resultsContainerRef}>
        {#if displayedFiles.length === 0}
          <div class="empty-state">No se encontraron archivos</div>
        {:else}
          <ul class="palette-list" bind:this={resultsListRef}>
            {#each displayedFiles as file, index (file.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <li
                class="palette-item"
                class:selected={index === selectedIndex}
                onclick={() => selectFile(file.path)}
                onmouseenter={() => (selectedIndex = index)}
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
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <footer class="palette-footer">
        <span><kbd>↑</kbd> <kbd>↓</kbd> <kbd>PgUp</kbd> <kbd>PgDn</kbd> Navegar</span>
        <span><kbd>Home</kbd> <kbd>End</kbd> Inicio/Fin</span>
        <span><kbd>↵</kbd> Abrir</span>
        <span><kbd>esc</kbd> Cerrar</span>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
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

  .palette-container {
    width: 620px;
    max-width: 90%;
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from { transform: translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateY(0) scale(1); opacity: 1; }
  }

  .input-wrapper {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
  }

  .search-icon {
    width: 18px;
    height: 18px;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    flex-shrink: 0;
  }

  input {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  input::placeholder {
    color: var(--text-secondary, #656d76);
  }

  .esc-badge {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .results-container {
    max-height: 340px;
    overflow-y: auto;
    padding: 6px 0;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  .palette-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .palette-item {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    transition: background-color 0.1s ease, color 0.1s ease;
  }

  .palette-item.selected {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  .palette-item.selected .category-tag {
    color: var(--accent, #0969da);
  }

  .category-tag.file-tag {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.08);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    min-width: unset;
    margin-right: 10px;
  }

  .file-icon {
    width: 16px;
    height: 16px;
    color: var(--accent, #0969da);
    margin-right: 10px;
    flex-shrink: 0;
  }

  .item-name {
    font-weight: 500;
    color: var(--text-primary, #1f2328);
    margin-right: 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-path {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    margin-left: auto;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-left: 12px;
    max-width: 45%;
  }

  .palette-footer {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 16px;
    background-color: rgba(0, 0, 0, 0.03);
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  .palette-footer kbd {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }
</style>
