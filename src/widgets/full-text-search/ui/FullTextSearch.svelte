<script lang="ts">
  import { Command, Dialog } from 'bits-ui';
  import { Search, RefreshCw } from 'lucide-svelte';
  import { FileIcon } from '@shared/ui/icons';
  import {
    searchRepository,
    type FullTextHit,
    type FullTextIndexStatus,
  } from '@shared/repositories';

  interface Props {
    isOpen?: boolean;
    onSelectHit?: (path: string, matchedTerms: string[]) => void;
    onClose?: () => void;
  }

  let {
    isOpen = $bindable(false),
    onSelectHit,
    onClose,
  }: Props = $props();

  let searchQuery = $state('');
  let hits = $state<FullTextHit[]>([]);
  let totalHits = $state(0);
  let elapsedMs = $state(0);
  let isLoading = $state(false);
  let isRebuilding = $state(false);

  let status = $state<FullTextIndexStatus>({
    indexed_docs: 0,
    pending: 0,
    is_indexing: false,
    in_memory_fallback: false,
  });

  // Dimensiones redimensionables del cuadro de diálogo con persistencia en localStorage
  function getInitialDialogWidth(): number {
    try {
      const saved = localStorage.getItem('synapse_fts_width');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 450 && p <= 1600) return p;
      }
    } catch {}
    return 720;
  }

  function getInitialDialogHeight(): number {
    try {
      const saved = localStorage.getItem('synapse_fts_height');
      if (saved) {
        const p = parseInt(saved, 10);
        if (!isNaN(p) && p >= 320 && p <= 1200) return p;
      }
    } catch {}
    return 520;
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
      const maxW = typeof window !== 'undefined' ? window.innerWidth * 0.96 : 1400;
      const maxH = typeof window !== 'undefined' ? window.innerHeight * 0.9 : 900;

      if (direction === 'se') {
        dialogWidth = Math.max(450, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(320, Math.min(startH + deltaY, maxH));
      } else if (direction === 'sw') {
        dialogWidth = Math.max(450, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(320, Math.min(startH + deltaY, maxH));
      } else if (direction === 'ne') {
        dialogWidth = Math.max(450, Math.min(startW + deltaX * 2, maxW));
        dialogHeight = Math.max(320, Math.min(startH - deltaY, maxH));
      } else if (direction === 'nw') {
        dialogWidth = Math.max(450, Math.min(startW - deltaX * 2, maxW));
        dialogHeight = Math.max(320, Math.min(startH - deltaY, maxH));
      } else if (direction === 'e') {
        dialogWidth = Math.max(450, Math.min(startW + deltaX * 2, maxW));
      } else if (direction === 'w') {
        dialogWidth = Math.max(450, Math.min(startW - deltaX * 2, maxW));
      } else if (direction === 's') {
        dialogHeight = Math.max(320, Math.min(startH + deltaY, maxH));
      } else if (direction === 'n') {
        dialogHeight = Math.max(320, Math.min(startH - deltaY, maxH));
      }
    };

    const onPointerUp = () => {
      isResizing = false;
      document.body.style.userSelect = '';
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerUp);
      try {
        localStorage.setItem('synapse_fts_width', String(Math.round(dialogWidth)));
        localStorage.setItem('synapse_fts_height', String(Math.round(dialogHeight)));
      } catch {}
    };

    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerUp);
  }

  // Refrescar estado del índice al abrir
  $effect(() => {
    if (isOpen) {
      searchRepository.getStatus().then((st) => {
        if (st) status = st;
      });
    }
  });

  // Búsqueda con Tantivy y debounce de 120ms
  $effect(() => {
    const q = searchQuery.trim();
    if (!q || !isOpen) {
      hits = [];
      totalHits = 0;
      elapsedMs = 0;
      isLoading = false;
      return;
    }

    isLoading = true;
    let active = true;

    const timer = setTimeout(() => {
      searchRepository
        .search(q, 50)
        .then((resp) => {
          if (active) {
            hits = resp.hits;
            totalHits = resp.total_hits;
            elapsedMs = resp.elapsed_ms;
            status = resp.status;
            isLoading = false;
          }
        })
        .catch((err) => {
          if (active) {
            console.error('Error al realizar búsqueda full-text:', err);
            isLoading = false;
          }
        });
    }, 120);

    return () => {
      active = false;
      clearTimeout(timer);
    };
  });

  // Atajo global Ctrl+Shift+F para abrir/cerrar
  $effect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && (e.key === 'f' || e.key === 'F')) {
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
    hits = [];
    if (onClose) onClose();
  }

  function selectHit(hit: FullTextHit) {
    if (onSelectHit) {
      onSelectHit(hit.note_path, hit.matched_terms);
    }
    closeDialog();
  }

  async function handleRebuildIndex() {
    if (isRebuilding) return;
    isRebuilding = true;
    try {
      await searchRepository.rebuildIndex();
      const updated = await searchRepository.getStatus();
      if (updated) status = updated;
      if (searchQuery.trim()) {
        const resp = await searchRepository.search(searchQuery.trim(), 50);
        hits = resp.hits;
        totalHits = resp.total_hits;
        elapsedMs = resp.elapsed_ms;
        status = resp.status;
      }
    } finally {
      isRebuilding = false;
    }
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
    <Dialog.Overlay class="fts-backdrop" />
    <Dialog.Content
      class={`fts-container ${isResizing ? 'is-resizing' : ''}`}
      style="width: {dialogWidth}px; height: {dialogHeight}px;"
    >
      <!-- Handles de redimensionamiento -->
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

      <Dialog.Title class="sr-only">Búsqueda en el Contenido de Notas (Full-Text)</Dialog.Title>

      <Command.Root class="fts-command-root" loop shouldFilter={false}>
        <!-- Barra de búsqueda superior -->
        <div class="fts-input-wrapper">
          <Search size={18} class="search-icon" />
          <Command.Input
            class="command-input"
            bind:value={searchQuery}
            placeholder="Buscar en el contenido de las notas... (Ctrl+Shift+F)"
          />
          {#if isLoading}
            <span class="fts-spinner" aria-label="Buscando..."></span>
          {/if}
          <span class="esc-badge">ESC</span>
        </div>

        <!-- Contenedor de resultados -->
        <Command.List class="fts-results-container">
          {#if searchQuery.trim().length === 0}
            <!-- Guía de sintaxis y tips cuando no hay texto -->
            <div class="fts-guide-box">
              <div class="guide-title">Búsqueda rápida en el contenido de las notas</div>
              <div class="guide-items">
                <div class="guide-item">
                  <span class="guide-badge">"frase exacta"</span>
                  <span class="guide-desc">Coincidencia de palabras adyacentes</span>
                </div>
                <div class="guide-item">
                  <span class="guide-badge">-excluir</span>
                  <span class="guide-desc">Omite notas con esa palabra</span>
                </div>
                <div class="guide-item">
                  <span class="guide-badge">path:carpeta</span>
                  <span class="guide-desc">Filtra por subdirectorio</span>
                </div>
                <div class="guide-item">
                  <span class="guide-badge">kind:markdown</span>
                  <span class="guide-desc">Filtra: markdown, mermaid, code, excalidraw</span>
                </div>
                <div class="guide-item">
                  <span class="guide-badge">title:termino</span>
                  <span class="guide-desc">Busca exclusivamente en títulos</span>
                </div>
              </div>
            </div>
          {:else if hits.length === 0 && !isLoading}
            <Command.Empty class="empty-state">
              No se encontraron coincidencias para <strong>"{searchQuery}"</strong>
            </Command.Empty>
          {:else}
            {#each hits as hit (hit.note_path)}
              <Command.Item
                value={hit.note_path}
                onSelect={() => selectHit(hit)}
                class="fts-hit-item"
              >
                <div class="fts-hit-main">
                  <div class="fts-hit-header">
                    <FileIcon path={hit.note_path} size={16} class="fts-icon" />
                    <span class="fts-hit-title">
                      {#each hit.title as seg}
                        {#if seg.highlighted}
                          <mark class="fts-mark">{seg.text}</mark>
                        {:else}
                          {seg.text}
                        {/if}
                      {/each}
                    </span>
                    <span class="fts-kind-tag {hit.kind}">{hit.kind}</span>
                    <span class="fts-hit-path" title={hit.note_path}>{hit.note_path}</span>
                  </div>

                  {#if hit.snippet && hit.snippet.length > 0}
                    <div class="fts-hit-snippet">
                      {#each hit.snippet as seg}
                        {#if seg.highlighted}
                          <mark class="fts-mark">{seg.text}</mark>
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
        </Command.List>

        <!-- Barra de estado inferior -->
        <div class="fts-footer">
          <div class="footer-left">
            {#if status.is_indexing}
              <span class="fts-spinner-small"></span>
              <span class="status-text">Indexando ({status.pending} pendientes)...</span>
            {:else}
              <span class="status-text">Índice: {status.indexed_docs} docs</span>
            {/if}

            {#if status.in_memory_fallback}
              <span class="fallback-tag" title="El índice se mantiene en memoria">Memoria</span>
            {/if}

            <button
              type="button"
              class="rebuild-btn"
              onclick={handleRebuildIndex}
              disabled={isRebuilding}
              title="Reconstruir índice completo"
            >
              <RefreshCw size={11} class={isRebuilding ? 'spin-icon' : ''} />
              <span>{isRebuilding ? 'Reconstruyendo...' : 'Reconstruir'}</span>
            </button>
          </div>

          <div class="footer-right">
            {#if searchQuery.trim().length > 0}
              <span class="stats-text">
                {totalHits} {totalHits === 1 ? 'resultado' : 'resultados'} ({elapsedMs.toFixed(1)} ms)
              </span>
            {/if}
            <div class="footer-shortcuts">
              <span><kbd>↑</kbd><kbd>↓</kbd> navegar</span>
              <span><kbd>↵</kbd> abrir</span>
              <span><kbd>esc</kbd> cerrar</span>
            </div>
          </div>
        </div>
      </Command.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.fts-backdrop) {
    position: fixed;
    inset: 0;
    z-index: 9998;
    background-color: var(--dialog-backdrop, rgba(0, 0, 0, 0.45));
    backdrop-filter: blur(2px);
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  :global(.fts-container) {
    position: fixed;
    top: 10%;
    left: 50%;
    transform: translateX(-50%);
    z-index: 9999;
    max-width: 96vw;
    max-height: 85vh;
    background-color: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 16px 36px var(--dialog-shadow, rgba(0, 0, 0, 0.22));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
    animation: slideDown 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  :global(.fts-container.is-resizing) {
    animation: none !important;
    user-select: none !important;
  }

  @keyframes slideDown {
    from { transform: translateX(-50%) translateY(-12px) scale(0.98); opacity: 0; }
    to { transform: translateX(-50%) translateY(0) scale(1); opacity: 1; }
  }

  /* Handles de redimensionamiento */
  :global(.fts-container .resize-handle) {
    position: absolute;
    z-index: 100;
  }

  :global(.fts-container .resize-handle.corner) {
    width: 18px;
    height: 18px;
  }

  :global(.fts-container .resize-handle.corner.nw) {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  :global(.fts-container .resize-handle.corner.ne) {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  :global(.fts-container .resize-handle.corner.sw) {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  :global(.fts-container .resize-handle.corner.se) {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 3px 4px;
  }

  :global(.fts-container .corner-grip-icon) {
    color: var(--text-secondary, #656d76);
    opacity: 0.4;
    pointer-events: none;
    transition: opacity 0.15s ease, color 0.15s ease;
  }

  :global(.fts-container .resize-handle.corner.se:hover .corner-grip-icon) {
    opacity: 1;
    color: var(--accent, #0969da);
  }

  :global(.fts-container .resize-handle.edge.n) {
    top: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.fts-container .resize-handle.edge.s) {
    bottom: 0;
    left: 18px;
    right: 18px;
    height: 6px;
    cursor: ns-resize;
  }

  :global(.fts-container .resize-handle.edge.w) {
    left: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.fts-container .resize-handle.edge.e) {
    right: 0;
    top: 18px;
    bottom: 18px;
    width: 6px;
    cursor: ew-resize;
  }

  :global(.fts-container .fts-command-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    flex: 1 1 100%;
    min-height: 0;
    outline: none;
  }

  :global(.fts-container .fts-input-wrapper) {
    display: flex;
    align-items: center;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    flex-shrink: 0;
    gap: 10px;
  }

  :global(.fts-container .search-icon) {
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.fts-container .command-input) {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    font-size: 15px;
    outline: none;
    font-family: inherit;
  }

  :global(.fts-container .command-input::placeholder) {
    color: var(--text-secondary, #656d76);
  }

  :global(.fts-container .esc-badge) {
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--text-secondary, #656d76);
    background: var(--bg-primary, #ffffff);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-primary, #d0d7de);
    flex-shrink: 0;
  }

  :global(.fts-container .fts-spinner) {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  :global(.fts-container .fts-spinner-small) {
    width: 11px;
    height: 11px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    display: inline-block;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  :global(.fts-container .fts-results-container) {
    flex: 1 1 0%;
    min-height: 0;
    height: auto !important;
    overflow-y: auto;
    padding: 6px 0;
    outline: none;
  }

  :global(.fts-container .empty-state) {
    padding: 32px 20px;
    text-align: center;
    color: var(--text-secondary, #656d76);
    font-size: 14px;
  }

  /* Guía de sintaxis cuando está vacío */
  .fts-guide-box {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    color: var(--text-secondary, #656d76);
  }

  .guide-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .guide-items {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 10px;
  }

  .guide-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }

  .guide-badge {
    font-family: var(--mono, monospace);
    font-size: 11px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    padding: 2px 6px;
    color: var(--accent, #0969da);
    white-space: nowrap;
  }

  .guide-desc {
    color: var(--text-secondary, #656d76);
  }

  /* Ítem de resultado */
  :global(.fts-container .fts-hit-item) {
    display: flex;
    flex-direction: column;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-secondary, #656d76);
    transition: background-color 0.1s ease, color 0.1s ease;
    user-select: none;
    outline: none;
    border-bottom: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.05));
  }

  :global(.fts-container .fts-hit-item:hover),
  :global(.fts-container .fts-hit-item[data-selected]),
  :global(.fts-container .fts-hit-item[data-highlighted]) {
    background-color: var(--accent-bg, rgba(9, 105, 218, 0.08));
    color: var(--text-primary, #1f2328);
  }

  .fts-hit-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
  }

  .fts-hit-header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  :global(.fts-container .fts-icon) {
    flex-shrink: 0;
  }

  .fts-hit-title {
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 14px;
  }

  .fts-kind-tag {
    font-size: 10px;
    text-transform: uppercase;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .fts-kind-tag.markdown {
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
  }

  .fts-kind-tag.mermaid {
    background: rgba(227, 98, 9, 0.12);
    color: #e36209;
  }

  .fts-kind-tag.code {
    background: rgba(130, 80, 223, 0.12);
    color: #8250df;
  }

  .fts-kind-tag.excalidraw {
    background: rgba(26, 127, 55, 0.12);
    color: #1a7f37;
  }

  .fts-hit-path {
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-left: auto;
    opacity: 0.7;
    max-width: 40%;
  }

  .fts-hit-snippet {
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

  /* Resaltado de coincidencias */
  :global(.fts-container mark.fts-mark) {
    background-color: var(--search-match-bg, rgba(234, 179, 8, 0.35));
    color: inherit;
    font-weight: 600;
    border-radius: 2px;
    padding: 0 1px;
  }

  /* Barra de pie */
  .fts-footer {
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

  .footer-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .status-text {
    font-weight: 500;
    white-space: nowrap;
  }

  .fallback-tag {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    background: rgba(207, 34, 46, 0.1);
    color: #cf222e;
    font-weight: 500;
  }

  .rebuild-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 10px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .rebuild-btn:hover:not(:disabled) {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border-color: var(--accent, #0969da);
  }

  .rebuild-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spin-icon {
    animation: spin 0.8s linear infinite;
  }

  .stats-text {
    font-weight: 500;
    opacity: 0.85;
    white-space: nowrap;
  }

  .footer-shortcuts {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  :global(.fts-container .fts-footer kbd) {
    font-family: var(--mono, monospace);
    background: var(--bg-secondary, #f6f8fa);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
  }
</style>
