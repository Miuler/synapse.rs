<script lang="ts">
  import type { VaultItem } from '@entities/vault-item';
  import { Command, Dialog } from 'bits-ui';
  import { Search, TextSearch } from 'lucide-svelte';
  import { FileIcon } from '@entities/file-type';
  import {
    vaultRepository,
    searchRepository,
    type SearchResult,
    type FullTextHit,
  } from '@shared/repositories';

  interface Props {
    isOpen?: boolean;
    vaultItems?: VaultItem[];
    recentFiles?: string[];
    onSelectFile?: (path: string, matchedTerms?: string[]) => void;
    onClose?: () => void;
  }

  let {
    isOpen = $bindable(false),
    vaultItems = [],
    recentFiles = [],
    onSelectFile,
    onClose
  }: Props = $props();

  // Constante de paginado para carga bajo demanda en scroll
  const PAGE_SIZE = 25;

  let searchQuery = $state('');
  let nucleoResults = $state<SearchResult[]>([]);
  let visibleCount = $state(PAGE_SIZE);
  let listContainerEl = $state<HTMLElement | null>(null);

  // Modo búsqueda full-text si la consulta inicia con '?'
  const isFtsMode = $derived(searchQuery.startsWith('?'));
  const ftsQuery = $derived(isFtsMode ? searchQuery.slice(1).trim() : '');
  let ftsResults = $state<FullTextHit[]>([]);
  let ftsTotalHits = $state(0);
  let ftsElapsedMs = $state(0);
  let isFtsLoading = $state(false);

  // Estadísticas de total de archivos y filtrados en memoria
  let totalFiles = $state<number>(0);
  let matchedFiles = $state<number>(0);

  // Dimensiones redimensionables del cuadro de diálogo con persistencia en localStorage
  function getInitialDialogWidth(): number {
    try {
      const saved = localStorage.getItem('synapse_quick_open_width');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 400 && p <= 1600) return p;
      }
    } catch {}
    return 620;
  }

  function getInitialDialogHeight(): number {
    try {
      const saved = localStorage.getItem('synapse_quick_open_height');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 280 && p <= 1200) return p;
      }
    } catch {}
    return 480;
  }

  let dialogWidth = $state<number>(getInitialDialogWidth());
  let dialogHeight = $state<number>(getInitialDialogHeight());
  let isResizing = $state(false);

  function startResize(e: PointerEvent, direction: 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w') {
    e.preventDefault();
    e.stopPropagation();
    isResizing = true;
    document.body.style.userSelect = 'none';

    const startX = e.clientX;
    const startY = e.clientY;
    const startW = dialogWidth;
    const startH = dialogHeight;

    const onPointerMove = (ev: PointerEvent) => {
      const deltaX = ev.clientX - startX;
      const deltaY = ev.clientY - startY;
      const maxW = typeof window !== 'undefined' ? window.innerWidth * 0.95 : 1400;
      const maxH = typeof window !== 'undefined' ? window.innerHeight * 0.85 : 900;

      if (direction === 'se') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'sw') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'ne') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      } else if (direction === 'nw') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      } else if (direction === 'e') {
        dialogWidth = Math.max(400, Math.min(startW + deltaX * 2, maxW));
      } else if (direction === 'w') {
        dialogWidth = Math.max(400, Math.min(startW - deltaX * 2, maxW));
      } else if (direction === 's') {
        dialogHeight = Math.max(280, Math.min(startH + deltaY, maxH));
      } else if (direction === 'n') {
        dialogHeight = Math.max(280, Math.min(startH - deltaY, maxH));
      }
    };

    const onPointerUp = () => {
      isResizing = false;
      document.body.style.userSelect = '';
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerUp);
      try {
        localStorage.setItem('synapse_quick_open_width', String(Math.round(dialogWidth)));
        localStorage.setItem('synapse_quick_open_height', String(Math.round(dialogHeight)));
      } catch {}
    };

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerUp);
  }

  interface FileItemDisplay {
    id: string;
    title: string;
    path: string;
    fileName: string;
    isRecent?: boolean;
  }

  // Prepara los candidatos completos para la lista priorizando el nombre del archivo
  let allCandidates = $derived.by<FileItemDisplay[]>(() => {
    if (searchQuery.trim().length > 0) {
      return nucleoResults.map((r) => {
        const path = r.note_path || r.text;
        const fileName = path.split('/').pop() || path;
        const isRecent = Boolean(r.is_recent || recentFiles.includes(path));
        return {
          id: path,
          title: r.text,
          path: path,
          fileName,
          isRecent,
        };
      });
    }

    const list: FileItemDisplay[] = [];
    const addedPaths = new Set<string>();

    // 1. Si no hay búsqueda, PRIORIDAD a los últimos archivos que abrió el usuario
    for (const path of recentFiles) {
      if (!path || path.startsWith('empty:') || addedPaths.has(path)) continue;
      const item = vaultItems.find((v) => v.relative_path === path);
      const fileName = path.split('/').pop() || path;
      list.push({
        id: path,
        title: item ? item.title : fileName.replace(/\.[^/.]+$/, ''),
        path: path,
        fileName,
        isRecent: true,
      });
      addedPaths.add(path);
    }

    // 2. Añadir el resto de archivos de la bóveda para permitir explorarlos paginados
    for (const item of vaultItems) {
      if (!item.relative_path || item.relative_path.startsWith('empty:') || addedPaths.has(item.relative_path)) continue;
      const fileName = item.relative_path.split('/').pop() || item.relative_path;
      list.push({
        id: item.relative_path,
        title: item.title,
        path: item.relative_path,
        fileName,
        isRecent: false,
      });
      addedPaths.add(item.relative_path);
    }

    return list;
  });

  // Lista visible en el DOM recortada al tamaño de página (apertura instantánea sin retrasos)
  let fileList = $derived(allCandidates.slice(0, visibleCount));

  // Determinar si existen archivos recientes para condicionalmente renderizar líneas divisoras
  let hasRecents = $derived(fileList.some((f) => f.isRecent));

  // Reiniciar el límite de página visible al abrir o cambiar la búsqueda
  $effect(() => {
    const _q = searchQuery;
    const _open = isOpen;
    visibleCount = PAGE_SIZE;
    if (listContainerEl) {
      listContainerEl.scrollTop = 0;
    }
  });

  // Obtener el total de archivos en memoria al abrir el diálogo
  $effect(() => {
    if (isOpen) {
      vaultRepository.getVaultFilesCount().then((count) => {
        if (count > 0) {
          totalFiles = count;
          if (!searchQuery.trim()) {
            matchedFiles = count;
          }
        } else if (vaultItems.length > 0) {
          totalFiles = vaultItems.length;
          if (!searchQuery.trim()) {
            matchedFiles = vaultItems.length;
          }
        }
      });
    }
  });

  // Búsqueda interactiva ultrarrápida con Nucleo en Rust (solo cuando no es modo FTS)
  $effect(() => {
    const q = searchQuery.trim();
    if (!q || !isOpen || isFtsMode) {
      nucleoResults = [];
      matchedFiles = totalFiles;
      return;
    }

    let active = true;
    vaultRepository.searchNotes(q).then((resp) => {
      if (active) {
        nucleoResults = resp.results;
        if (resp.total_files > 0) totalFiles = resp.total_files;
        matchedFiles = resp.matched_files;
      }
    });

    return () => {
      active = false;
    };
  });

  // Búsqueda interactiva Full-Text con Tantivy cuando inicia con '?'
  $effect(() => {
    if (!isOpen || !isFtsMode) {
      ftsResults = [];
      ftsTotalHits = 0;
      ftsElapsedMs = 0;
      isFtsLoading = false;
      return;
    }

    const q = ftsQuery;
    if (!q) {
      ftsResults = [];
      ftsTotalHits = 0;
      ftsElapsedMs = 0;
      isFtsLoading = false;
      return;
    }

    isFtsLoading = true;
    let active = true;

    const timer = setTimeout(() => {
      searchRepository
        .search(q, 50)
        .then((resp) => {
          if (active) {
            ftsResults = resp.hits;
            ftsTotalHits = resp.total_hits;
            ftsElapsedMs = resp.elapsed_ms;
            isFtsLoading = false;
          }
        })
        .catch((err) => {
          if (active) {
            console.error('Error en búsqueda FTS desde QuickOpen:', err);
            isFtsLoading = false;
          }
        });
    }, 120);

    return () => {
      active = false;
      clearTimeout(timer);
    };
  });

  // Atajo global Ctrl+O o Cmd+O para abrir/cerrar el diálogo de archivos
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (e.key === 'o' || e.key === 'O')) {
        e.preventDefault();
        isOpen = !isOpen;
        if (isOpen) {
          searchQuery = '';
          visibleCount = PAGE_SIZE;
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLElement;
    if (!target) return;
    const distanceToBottom = target.scrollHeight - target.scrollTop - target.clientHeight;
    if (distanceToBottom < 100 && visibleCount < allCandidates.length) {
      visibleCount = Math.min(visibleCount + PAGE_SIZE, allCandidates.length);
    }
  }

  function handleKeydownRoot(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' && visibleCount < allCandidates.length) {
      const highlighted = listContainerEl?.querySelector('[data-highlighted], [data-selected]');
      if (highlighted && highlighted.nextElementSibling === null) {
        visibleCount = Math.min(visibleCount + PAGE_SIZE, allCandidates.length);
      }
    }
  }

  function closeDialog() {
    isOpen = false;
    searchQuery = '';
    visibleCount = PAGE_SIZE;
    if (onClose) onClose();
  }

  function selectFile(path: string, matchedTerms?: string[]) {
    if (onSelectFile) onSelectFile(path, matchedTerms);
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
    <Dialog.Overlay class="quick-open-backdrop" />
    <Dialog.Content
      class={`quick-open-container ${isResizing ? 'is-resizing' : ''}`}
      style="width: {dialogWidth}px; height: {dialogHeight}px;"
    >
      <!-- Handles de redimensionamiento desde las 4 esquinas y 4 bordes -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner nw" onpointerdown={(e) => startResize(e, 'nw')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner ne" onpointerdown={(e) => startResize(e, 'ne')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner sw" onpointerdown={(e) => startResize(e, 'sw')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle corner se" onpointerdown={(e) => startResize(e, 'se')}>
        <svg class="corner-grip-icon" viewBox="0 0 10 10" width="10" height="10">
          <path d="M9 1L1 9 M9 5L5 9 M9 9L9 9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
        </svg>
      </div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge n" onpointerdown={(e) => startResize(e, 'n')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge s" onpointerdown={(e) => startResize(e, 's')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge e" onpointerdown={(e) => startResize(e, 'e')}></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-handle edge w" onpointerdown={(e) => startResize(e, 'w')}></div>

      <Dialog.Title class="sr-only">Buscador Rápido de Archivos</Dialog.Title>
      <Command.Root
        class="quick-open-command-root"
        loop
        shouldFilter={!isFtsMode && !searchQuery.trim()}
        onkeydown={handleKeydownRoot}
      >
        <div class="quick-open-input-wrapper" class:is-fts={isFtsMode}>
          {#if isFtsMode}
            <TextSearch size={16} class="search-icon fts-active-icon" />
          {:else}
            <Search size={16} class="search-icon" />
          {/if}
          <Command.Input
            class="command-input"
            bind:value={searchQuery}
            placeholder={isFtsMode ? "Buscar en el contenido de las notas... (Tantivy)" : "Buscar archivo por nombre... (Ctrl+O, escribe '?' para contenido)"}
          />
          {#if isFtsMode}
            <span class="fts-indicator-badge">FTS</span>
          {/if}
          {#if isFtsLoading}
            <span class="quick-fts-spinner"></span>
          {/if}
          <span class="esc-badge">ESC</span>
        </div>

        <Command.List
          bind:ref={listContainerEl}
          class="quick-open-results-container"
          onscroll={handleScroll}
        >
          {#if isFtsMode}
            {#if ftsQuery.length === 0}
              <div class="fts-mode-hint">
                <div class="fts-hint-title">
                  <TextSearch size={15} />
                  <span>Búsqueda Full-Text activada</span>
                </div>
                <p class="fts-hint-desc">
                  Escribe cualquier término tras el signo <code>?</code> para buscar en el interior de tus notas, diagramas y código.
                </p>
                <div class="fts-hint-tags">
                  <span>Ej: <code>?servidor</code></span>
                  <span><code>?"frase exacta"</code></span>
                  <span><code>?kind:markdown</code></span>
                </div>
              </div>
            {:else if ftsResults.length === 0 && !isFtsLoading}
              <Command.Empty class="empty-state">
                No se encontraron coincidencias en el contenido para <strong>"{ftsQuery}"</strong>
              </Command.Empty>
            {:else}
              {#each ftsResults as hit (hit.note_path)}
                <Command.Item
                  class="palette-item quick-fts-item"
                  value={hit.note_path}
                  onSelect={() => selectFile(hit.note_path, hit.matched_terms)}
                >
                  <div class="quick-fts-main">
                    <div class="quick-fts-header">
                      <FileIcon path={hit.note_path} size={15} class="file-icon" />
                      <span class="item-name">
                        {#each hit.title as seg}
                          {#if seg.highlighted}
                            <mark class="quick-fts-mark">{seg.text}</mark>
                          {:else}
                            {seg.text}
                          {/if}
                        {/each}
                      </span>
                      <span class="quick-fts-kind {hit.kind}">{hit.kind}</span>
                      <span class="item-path">{hit.note_path}</span>
                    </div>
                    {#if hit.snippet && hit.snippet.length > 0}
                      <div class="quick-fts-snippet">
                        {#each hit.snippet as seg}
                          {#if seg.highlighted}
                            <mark class="quick-fts-mark">{seg.text}</mark>
                          {:else}
                            {seg.text}
                          {/if}
                        {/each}
                      </div>
                    {/if}
                  </div>
                </Command.Item>
              {/each}
            {/if}
          {:else}
            <Command.Empty class="empty-state">No se encontraron archivos</Command.Empty>

            {#each fileList as file, i (file.id)}
              <!-- Líneas de sección solo si existen archivos recientes -->
              {#if hasRecents && i === 0 && file.isRecent}
                <div class="section-divider">
                  <span class="section-divider-label">Recientes</span>
                </div>
              {:else if hasRecents && !file.isRecent && (i === 0 || fileList[i - 1]?.isRecent)}
                <div class="section-divider">
                  <span class="section-divider-label">Archivos</span>
                </div>
              {/if}

              <Command.Item
                class="palette-item"
                value={`${file.fileName} ${file.title} ${file.path}`}
                onSelect={() => selectFile(file.path)}
              >
                <FileIcon path={file.path} name={file.fileName} size={15} class="file-icon" />
                <span class="item-name">{file.fileName}</span>
                <span class="item-path">
                  {file.path}{file.title && file.title !== file.fileName && file.title !== file.fileName.replace(/\.[^/.]+$/, '') ? ` · ${file.title}` : ''}
                </span>
              </Command.Item>
            {/each}

            {#if visibleCount < allCandidates.length}
              <div class="scroll-more-indicator">
                Mostrando {visibleCount} de {allCandidates.length} archivos (desplázate para cargar más)
              </div>
            {/if}
          {/if}
        </Command.List>

        <footer class="quick-open-footer">
          <div class="footer-shortcuts">
            <span><kbd>↑</kbd> <kbd>↓</kbd> Navegar</span>
            <span><kbd>↵</kbd> Abrir</span>
            <span><kbd>esc</kbd> Cerrar</span>
          </div>
          <div class="footer-stats">
            {#if isFtsMode}
              {#if ftsQuery.length > 0}
                <span>{ftsTotalHits.toLocaleString()} {ftsTotalHits === 1 ? 'coincidencia' : 'coincidencias'} ({ftsElapsedMs.toFixed(1)} ms) en contenido</span>
              {:else}
                <span>Modo Full-Text (?...)</span>
              {/if}
            {:else if searchQuery.trim().length > 0}
              <span>{matchedFiles.toLocaleString()} de {totalFiles.toLocaleString()} archivos</span>
            {:else if totalFiles > 0}
              <span>{totalFiles.toLocaleString()} archivos</span>
            {/if}
          </div>
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

  :global(.quick-open-backdrop) {
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

  :global(.quick-open-container) {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    min-width: 400px;
    max-width: 95vw;
    min-height: 280px;
    max-height: 85vh;
    background-color: var(--bg-primary, #ffffff);
    border-radius: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--modal-shadow, 0 24px 48px rgba(0, 0, 0, 0.1));
    overflow: hidden;
    display: flex;
    flex-direction: column;
    z-index: 1001;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  :global(.quick-open-container.is-resizing) {
    animation: none !important;
    user-select: none !important;
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  /* Handles de redimensionamiento */
  :global(.quick-open-container .resize-handle) {
    position: absolute;
    z-index: 100;
  }

  :global(.quick-open-container .resize-handle.corner) {
    width: 18px;
    height: 18px;
  }

  :global(.quick-open-container .resize-handle.corner.nw) {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  :global(.quick-open-container .resize-handle.corner.ne) {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  :global(.quick-open-container .resize-handle.corner.sw) {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  :global(.quick-open-container .resize-handle.corner.se) {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 3px 4px;
  }

  :global(.quick-open-container .corner-grip-icon) {
    color: var(--text-secondary, #656d76);
    opacity: 0.4;
    pointer-events: none;
    transition: opacity 0.15s ease, color 0.15s ease;
  }

  :global(.quick-open-container .resize-handle.corner.se:hover .corner-grip-icon) {
    opacity: 1;
    color: var(--accent, #0969da);
  }

  :global(.quick-open-container .resize-handle.edge.n) {
    top: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.quick-open-container .resize-handle.edge.s) {
    bottom: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.quick-open-container .resize-handle.edge.w) {
    left: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.quick-open-container .resize-handle.edge.e) {
    right: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.quick-open-container .quick-open-command-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1 1 100%;
    min-height: 0;
    outline: none;
  }

  :global(.quick-open-container .quick-open-input-wrapper) {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    flex-shrink: 0;
  }

  :global(.quick-open-container .search-icon) {
    width: 18px;
    height: 18px;
    color: var(--text-secondary, #656d76);
    margin-right: 12px;
    flex-shrink: 0;
  }

  :global(.quick-open-container .command-input) {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  :global(.quick-open-container .command-input::placeholder) {
    color: var(--text-secondary, #656d76);
  }

  :global(.quick-open-container .esc-badge) {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.quick-open-container .quick-open-results-container) {
    flex: 1 1 0%;
    min-height: 0;
    max-height: none !important;
    height: auto !important;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.quick-open-container .empty-state) {
    padding: 24px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  :global(.quick-open-container .section-divider) {
    display: flex;
    align-items: center;
    padding: 10px 16px 4px;
    user-select: none;
    pointer-events: none;
  }

  :global(.quick-open-container .section-divider-label) {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #656d76);
    opacity: 0.75;
  }

  :global(.quick-open-container .section-divider::after) {
    content: '';
    flex: 1;
    margin-left: 10px;
    height: 1px;
    background-color: var(--border-primary, #d0d7de);
    opacity: 0.5;
  }

  :global(.quick-open-container .palette-item) {
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

  :global(.quick-open-container .palette-item:hover),
  :global(.quick-open-container .palette-item[data-selected]),
  :global(.quick-open-container .palette-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--text-primary, #1f2328);
  }

  :global(.quick-open-container .file-icon) {
    width: 16px;
    height: 16px;
    margin-right: 10px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.quick-open-container .item-name) {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin-right: 8px;
    white-space: nowrap;
  }

  :global(.quick-open-container .item-path) {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.75;
  }

  :global(.quick-open-container .quick-open-footer) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background-color: var(--footer-bg, rgba(0, 0, 0, 0.03));
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
    margin-top: auto;
  }

  :global(.quick-open-container .footer-shortcuts) {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  :global(.quick-open-container .footer-stats) {
    display: flex;
    align-items: center;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary, #656d76);
    opacity: 0.85;
    user-select: none;
    white-space: nowrap;
  }

  :global(.quick-open-container .quick-open-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.quick-open-container .scroll-more-indicator) {
    padding: 10px 16px;
    text-align: center;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    border-top: 1px dashed var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    user-select: none;
  }

  :global(.quick-open-container .quick-open-input-wrapper.is-fts) {
    border-bottom: 2px solid var(--accent, #0969da);
  }

  :global(.quick-open-container .fts-active-icon) {
    color: var(--accent, #0969da) !important;
  }

  :global(.quick-open-container .fts-indicator-badge) {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--mono, monospace);
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.12);
    padding: 2px 5px;
    border-radius: 4px;
    border: 1px solid rgba(9, 105, 218, 0.25);
    flex-shrink: 0;
  }

  :global(.quick-open-container .quick-fts-spinner) {
    width: 13px;
    height: 13px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .fts-mode-hint {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    color: var(--text-secondary, #656d76);
  }

  .fts-hint-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent, #0969da);
  }

  .fts-hint-desc {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .fts-hint-desc code,
  .fts-hint-tags code {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
  }

  .fts-hint-tags {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  .quick-fts-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
  }

  .quick-fts-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .quick-fts-kind {
    font-size: 10px;
    text-transform: uppercase;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .quick-fts-kind.markdown {
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
  }

  .quick-fts-kind.mermaid {
    background: rgba(227, 98, 9, 0.12);
    color: #e36209;
  }

  .quick-fts-kind.code {
    background: rgba(130, 80, 223, 0.12);
    color: #8250df;
  }

  .quick-fts-kind.excalidraw {
    background: rgba(26, 127, 55, 0.12);
    color: #1a7f37;
  }

  .quick-fts-snippet {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary, #656d76);
    padding-left: 24px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    word-break: break-word;
  }

  :global(.quick-open-container mark.quick-fts-mark) {
    background-color: var(--search-match-bg, rgba(234, 179, 8, 0.35));
    color: inherit;
    font-weight: 600;
    border-radius: 2px;
    padding: 0 1px;
  }
</style>
