<script lang="ts">
  import { tick } from "svelte";

  export type MarkdownViewMode = 'live' | 'source' | 'reading';

  interface Props {
    wordCount?: number;
    charCount?: number;
    line?: number;
    col?: number;
    hasSelection?: boolean;
    syncStatus?: 'synced' | 'saving' | 'error';
    isVimMode?: boolean;
    encoding?: string;
    isMarkdownFile?: boolean;
    markdownViewMode?: MarkdownViewMode;
    onToggleVim?: () => void;
    onToggleMarkdownView?: () => void;
    onChangeMarkdownView?: (newMode: MarkdownViewMode) => void;
    onOpenCommandPalette?: () => void;
    onChangeEncoding?: (newEncoding: string) => void;
  }

  let {
    wordCount = 0,
    charCount = 0,
    line = 0,
    col = 0,
    hasSelection = false,
    syncStatus = 'synced',
    isVimMode = false,
    encoding = '---',
    isMarkdownFile = false,
    markdownViewMode = 'live',
    onToggleVim,
    onToggleMarkdownView,
    onChangeMarkdownView,
    onOpenCommandPalette,
    onChangeEncoding
  }: Props = $props();

  // Estados para el selector de vistas Markdown
  let isViewMenuOpen = $state(false);
  let selectedViewIndex = $state(0);
  let viewContainerRef = $state<HTMLDivElement | null>(null);

  const MARKDOWN_VIEW_MODES: {
    id: MarkdownViewMode;
    label: string;
    badge: string;
    description: string;
  }[] = [
    {
      id: 'live',
      label: 'En vivo',
      badge: 'Live Preview',
      description: 'Editor con estilos enriquecidos y diagramas Mermaid en vivo',
    },
    {
      id: 'source',
      label: 'Fuente',
      badge: 'Source Mode',
      description: 'Código Markdown puro con resaltado de sintaxis',
    },
    {
      id: 'reading',
      label: 'Lectura',
      badge: 'Reading View',
      description: 'Documento renderizado final de solo lectura',
    },
  ];

  function toggleViewMenu(e: MouseEvent) {
    e.stopPropagation();
    isEncodingMenuOpen = false;
    isViewMenuOpen = !isViewMenuOpen;
    if (isViewMenuOpen) {
      const currentIdx = MARKDOWN_VIEW_MODES.findIndex(m => m.id === markdownViewMode);
      selectedViewIndex = currentIdx !== -1 ? currentIdx : 0;
    }
  }

  function selectMarkdownView(mode: MarkdownViewMode) {
    isViewMenuOpen = false;
    if (onChangeMarkdownView) {
      onChangeMarkdownView(mode);
    } else if (onToggleMarkdownView) {
      onToggleMarkdownView();
    }
  }

  function handleViewKeydown(e: KeyboardEvent) {
    if (!isViewMenuOpen) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedViewIndex = (selectedViewIndex + 1) % MARKDOWN_VIEW_MODES.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedViewIndex = (selectedViewIndex - 1 + MARKDOWN_VIEW_MODES.length) % MARKDOWN_VIEW_MODES.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const selected = MARKDOWN_VIEW_MODES[selectedViewIndex];
      if (selected) {
        selectMarkdownView(selected.id);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      isViewMenuOpen = false;
    }
  }

  // Estados para el selector de codificación
  let isEncodingMenuOpen = $state(false);
  let searchQuery = $state('');
  let selectedMenuIndex = $state(0);
  let encodingContainerRef = $state<HTMLDivElement | null>(null);
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let listRef = $state<HTMLDivElement | null>(null);

  const ENCODINGS = [
    { id: 'UTF-8', label: 'UTF-8', description: 'Unicode (Estándar recomendado)' },
    { id: 'UTF-8 con BOM', label: 'UTF-8 con BOM', description: 'Unicode con marca de orden de bytes' },
    { id: 'ASCII', label: 'ASCII', description: 'US-ASCII de 7 bits' },
    { id: 'Windows-1252', label: 'Windows-1252', description: 'ANSI / Europa Occidental' },
    { id: 'ISO-8859-1', label: 'ISO-8859-1 (Latin-1)', description: 'Europa Occidental' },
    { id: 'ISO-8859-2', label: 'ISO-8859-2 (Latin-2)', description: 'Europa Central y Oriental' },
    { id: 'ISO-8859-15', label: 'ISO-8859-15 (Latin-9)', description: 'Europa Occidental con símbolo Euro' },
    { id: 'Windows-1250', label: 'Windows-1250', description: 'Europa Central' },
    { id: 'Windows-1251', label: 'Windows-1251', description: 'Cirílico' },
    { id: 'UTF-16 LE', label: 'UTF-16 LE', description: 'Unicode 16-bit Little Endian' },
    { id: 'UTF-16 BE', label: 'UTF-16 BE', description: 'Unicode 16-bit Big Endian' },
    { id: 'Shift_JIS', label: 'Shift_JIS', description: 'Japonés' },
    { id: 'GBK', label: 'GBK / GB2312', description: 'Chino Simplificado' },
    { id: 'Big5', label: 'Big5', description: 'Chino Tradicional' },
    { id: 'EUC-KR', label: 'EUC-KR', description: 'Coreano' },
  ];

  let filteredEncodings = $derived(
    searchQuery.trim()
      ? ENCODINGS.filter(e =>
          e.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
          e.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
          e.id.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : ENCODINGS
  );

  function isCurrentEncoding(encId: string, label: string): boolean {
    if (!encoding || encoding === '---') return false;
    const normCurrent = encoding.trim().toLowerCase().replace(/[-_\s]/g, '');
    const normId = encId.toLowerCase().replace(/[-_\s]/g, '');
    const normLabel = label.toLowerCase().replace(/[-_\s]/g, '');
    return normCurrent === normId || normCurrent === normLabel;
  }

  function toggleMenu(e: MouseEvent) {
    e.stopPropagation();
    isViewMenuOpen = false;
    isEncodingMenuOpen = !isEncodingMenuOpen;
    if (isEncodingMenuOpen) {
      searchQuery = '';
      const currentIdx = filteredEncodings.findIndex(e => isCurrentEncoding(e.id, e.label));
      selectedMenuIndex = currentIdx !== -1 ? currentIdx : 0;
      tick().then(() => {
        if (searchInputRef) searchInputRef.focus();
        scrollSelectedIntoView();
      });
    }
  }

  function scrollSelectedIntoView() {
    if (!listRef) return;
    const selectedEl = listRef.children[selectedMenuIndex] as HTMLElement | undefined;
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: 'nearest' });
    }
  }

  function selectEncoding(encId: string) {
    isEncodingMenuOpen = false;
    if (onChangeEncoding) {
      onChangeEncoding(encId);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isEncodingMenuOpen) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredEncodings.length > 0) {
        selectedMenuIndex = (selectedMenuIndex + 1) % filteredEncodings.length;
        tick().then(scrollSelectedIntoView);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredEncodings.length > 0) {
        selectedMenuIndex = (selectedMenuIndex - 1 + filteredEncodings.length) % filteredEncodings.length;
        tick().then(scrollSelectedIntoView);
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredEncodings[selectedMenuIndex]) {
        selectEncoding(filteredEncodings[selectedMenuIndex].id);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      isEncodingMenuOpen = false;
    }
  }

  $effect(() => {
    if (!isEncodingMenuOpen && !isViewMenuOpen) return;

    function handleClickOutside(event: MouseEvent) {
      const target = event.target as Node;
      if (isEncodingMenuOpen && encodingContainerRef && !encodingContainerRef.contains(target)) {
        isEncodingMenuOpen = false;
      }
      if (isViewMenuOpen && viewContainerRef && !viewContainerRef.contains(target)) {
        isViewMenuOpen = false;
      }
    }

    document.addEventListener('pointerdown', handleClickOutside);
    return () => {
      document.removeEventListener('pointerdown', handleClickOutside);
    };
  });
</script>

<footer class="status-bar">
  <div class="left-group">
    <button
      type="button"
      class="status-item clickable"
      onclick={() => { if (onOpenCommandPalette) onOpenCommandPalette(); }}
      title="Abrir paleta de comandos"
    >
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"/>
      </svg>
      <span>Ctrl+P</span>
    </button>

    <div class="divider"></div>

    <div class="status-item">
      <span class="dot {syncStatus}"></span>
      <span>{syncStatus === 'synced' ? 'Guardado' : syncStatus === 'saving' ? 'Guardando...' : 'Error de guardado'}</span>
    </div>
  </div>

  <div class="right-group">
    <!-- Selector desplegable de Modo de Vista Markdown (En vivo / Fuente / Lectura) -->
    {#if isMarkdownFile}
      <div class="md-view-container" bind:this={viewContainerRef}>
        <button
          type="button"
          class="status-item clickable md-view-btn"
          class:live-mode={markdownViewMode === 'live'}
          class:source-mode={markdownViewMode === 'source'}
          class:reading-mode={markdownViewMode === 'reading'}
          class:active={isViewMenuOpen}
          onclick={toggleViewMenu}
          title="Cambiar modo de vista Markdown: En vivo, Fuente o Lectura"
        >
          {#if markdownViewMode === 'live'}
            <!-- Ícono En vivo / Sparkles / Live Preview -->
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
            </svg>
            <span>En vivo</span>
          {:else if markdownViewMode === 'source'}
            <!-- Ícono Modo Fuente / Código -->
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="16 18 22 12 16 6"/>
              <polyline points="8 6 2 12 8 18"/>
            </svg>
            <span>Fuente</span>
          {:else}
            <!-- Ícono Modo Lectura / Documento -->
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
              <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
            </svg>
            <span>Lectura</span>
          {/if}
          <svg class="chevron-icon" class:open={isViewMenuOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>

        {#if isViewMenuOpen}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div class="view-dropdown" role="dialog" aria-modal="true" tabindex="-1" onkeydown={handleViewKeydown}>
            <div class="view-dropdown-header">
              <span class="header-title">Vista Markdown</span>
              <span class="header-current">
                {markdownViewMode === 'live' ? 'En vivo' : markdownViewMode === 'source' ? 'Fuente' : 'Lectura'}
              </span>
            </div>

            <div class="view-options-list">
              {#each MARKDOWN_VIEW_MODES as item, idx}
                {@const isCurrent = markdownViewMode === item.id}
                <button
                  type="button"
                  class="view-item"
                  class:selected={idx === selectedViewIndex}
                  class:current={isCurrent}
                  class:mode-live={item.id === 'live'}
                  class:mode-source={item.id === 'source'}
                  class:mode-reading={item.id === 'reading'}
                  onmouseenter={() => { selectedViewIndex = idx; }}
                  onclick={() => selectMarkdownView(item.id)}
                >
                  <div class="item-icon-box">
                    {#if item.id === 'live'}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
                      </svg>
                    {:else if item.id === 'source'}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="16 18 22 12 16 6"/>
                        <polyline points="8 6 2 12 8 18"/>
                      </svg>
                    {:else}
                      <svg class="item-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
                        <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
                      </svg>
                    {/if}
                  </div>

                  <div class="item-text">
                    <div class="item-title-row">
                      <span class="item-label">{item.label}</span>
                      <span class="item-badge">{item.badge}</span>
                    </div>
                    <span class="item-desc">{item.description}</span>
                  </div>

                  {#if isCurrent}
                    <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      <div class="divider"></div>
    {/if}

    <button
      type="button"
      class="status-item clickable vim-btn"
      class:active={isVimMode}
      onclick={() => { if (onToggleVim) onToggleVim(); }}
      title={isVimMode ? 'Desactivar modo VIM en el editor' : 'Activar modo VIM en el editor'}
    >
      <span class="vim-badge">VIM</span>
      <span>{isVimMode ? 'ON' : 'OFF'}</span>
    </button>
    <div class="divider"></div>
    {#if hasSelection}
      <span class="selection-badge">sel</span>
    {/if}
    <div
      class="status-item"
      title={hasSelection ? "Palabras en la selección actual" : "Palabras en todo el documento"}
    >
      <span>{wordCount} palabras</span>
    </div>
    <div
      class="status-item"
      title={hasSelection ? "Caracteres en la selección actual" : "Caracteres en todo el documento"}
    >
      <span>{charCount} caracteres</span>
    </div>
    <div class="divider"></div>
    <div
      class="status-item"
      title={hasSelection ? "Líneas y columnas/caracteres en la selección" : "Posición del cursor: Línea y Columna"}
    >
      <span>Lín {line}, Col {col}</span>
    </div>
    <div class="divider"></div>
    
    <!-- Selector de Codificación de Caracteres -->
    <div class="encoding-container" bind:this={encodingContainerRef}>
      <button
        type="button"
        class="status-item clickable encoding-btn"
        class:active={isEncodingMenuOpen}
        onclick={toggleMenu}
        title="Cambiar codificación del archivo y guardar con la nueva codificación"
      >
        <span class="encoding-text">{encoding || '---'}</span>
        <svg class="chevron-icon" class:open={isEncodingMenuOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="6 9 12 15 18 9"/>
        </svg>
      </button>

      {#if isEncodingMenuOpen}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div class="encoding-dropdown" role="dialog" aria-modal="true" tabindex="-1" onkeydown={handleKeydown}>
          <div class="encoding-dropdown-header">
            <span class="header-title">Guardar con codificación</span>
            <span class="header-current">Actual: {encoding || '---'}</span>
          </div>

          <div class="encoding-search-box">
            <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"/>
              <line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input
              type="text"
              class="encoding-search-input"
              placeholder="Buscar codificación..."
              bind:value={searchQuery}
              bind:this={searchInputRef}
              oninput={() => { selectedMenuIndex = 0; }}
            />
          </div>

          <div class="encoding-options-list" bind:this={listRef}>
            {#if filteredEncodings.length === 0}
              <div class="empty-results">No se encontraron codificaciones</div>
            {:else}
              {#each filteredEncodings as item, idx}
                {@const isCurrent = isCurrentEncoding(item.id, item.label)}
                <button
                  type="button"
                  class="encoding-item"
                  class:selected={idx === selectedMenuIndex}
                  class:current={isCurrent}
                  onmouseenter={() => { selectedMenuIndex = idx; }}
                  onclick={() => selectEncoding(item.id)}
                >
                  <div class="item-text">
                    <span class="item-label">{item.label}</span>
                    <span class="item-desc">{item.description}</span>
                  </div>
                  {#if isCurrent}
                    <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  {/if}
                </button>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</footer>

<style>
  .status-bar {
    height: 26px;
    flex-shrink: 0;
    background-color: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    position: relative;
  }

  .left-group,
  .right-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-item.clickable {
    background: transparent;
    border: none;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: all 0.15s ease;
    font-size: inherit;
    font-family: inherit;
  }

  .status-item.clickable:hover,
  .status-item.clickable.active {
    background: rgba(0, 0, 0, 0.05);
    color: var(--accent, #0969da);
  }

  /* Contenedor del selector de vista Markdown */
  .md-view-container {
    position: relative;
    display: flex;
    align-items: center;
  }

  .md-view-btn {
    font-weight: 500;
    gap: 5px;
    display: flex;
    align-items: center;
  }

  .md-view-btn.live-mode {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.08);
    font-weight: 600;
  }

  .md-view-btn.live-mode:hover,
  .md-view-btn.live-mode.active {
    background: rgba(46, 160, 67, 0.16);
  }

  .md-view-btn.source-mode {
    color: var(--accent, #0969da);
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
    font-weight: 600;
  }

  .md-view-btn.source-mode:hover,
  .md-view-btn.source-mode.active {
    background: var(--accent-bg, rgba(9, 105, 218, 0.16));
  }

  .md-view-btn.reading-mode {
    color: #8250df;
    background: rgba(130, 80, 223, 0.08);
    font-weight: 600;
  }

  .md-view-btn.reading-mode:hover,
  .md-view-btn.reading-mode.active {
    background: rgba(130, 80, 223, 0.16);
  }

  /* Menú desplegable para vistas Markdown */
  .view-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    width: 290px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    z-index: 1000;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: fadeInSlideUp 0.15s ease-out;
    outline: none;
  }

  .view-dropdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  .view-options-list {
    padding: 4px 0;
    display: flex;
    flex-direction: column;
  }

  .view-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s ease;
    outline: none;
  }

  .view-item:hover,
  .view-item.selected {
    background: rgba(0, 0, 0, 0.04);
  }

  .view-item.current {
    background: rgba(9, 105, 218, 0.05);
  }

  .view-item.current.selected {
    background: rgba(9, 105, 218, 0.1);
  }

  .item-icon-box {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
  }

  .view-item.mode-live .item-icon-box {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.1);
  }

  .view-item.mode-source .item-icon-box {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.1);
  }

  .view-item.mode-reading .item-icon-box {
    color: #8250df;
    background: rgba(130, 80, 223, 0.1);
  }

  .item-icon {
    width: 14px;
    height: 14px;
  }

  .item-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .item-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .item-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .item-badge {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 4px;
    font-weight: 500;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .view-item.current.mode-live .item-label {
    color: #1a7f37;
  }

  .view-item.current.mode-source .item-label {
    color: var(--accent, #0969da);
  }

  .view-item.current.mode-reading .item-label {
    color: #8250df;
  }

  .item-desc {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
    line-height: 1.3;
  }

  /* Selector de Codificación */
  .encoding-container {
    position: relative;
    display: flex;
    align-items: center;
  }

  .encoding-btn {
    font-weight: 500;
    gap: 4px;
    display: flex;
    align-items: center;
  }

  .encoding-text {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron-icon {
    width: 10px;
    height: 10px;
    opacity: 0.7;
    transition: transform 0.15s ease;
  }

  .chevron-icon.open {
    transform: rotate(180deg);
  }

  /* Menú flotante emergente */
  .encoding-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    width: 290px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    z-index: 1000;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: fadeInSlideUp 0.15s ease-out;
    outline: none;
  }

  @keyframes fadeInSlideUp {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .encoding-dropdown-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  .header-title {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-primary, #1f2328);
  }

  .header-current {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
  }

  .encoding-search-box {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    gap: 6px;
    background: var(--bg-primary, #ffffff);
  }

  .search-icon {
    width: 12px;
    height: 12px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  .encoding-search-input {
    flex: 1;
    border: none;
    outline: none;
    font-size: 11px;
    background: transparent;
    color: var(--text-primary, #1f2328);
  }

  .encoding-options-list {
    max-height: 240px;
    overflow-y: auto;
    padding: 4px 0;
  }

  .empty-results {
    padding: 12px;
    text-align: center;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  .encoding-item {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s ease;
  }

  .encoding-item.selected {
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
  }

  .encoding-item.current .item-label {
    color: var(--accent, #0969da);
    font-weight: 600;
  }

  .check-icon {
    width: 13px;
    height: 13px;
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .vim-btn {
    font-weight: 600;
  }

  .vim-btn.active {
    color: var(--accent, #0969da);
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
  }

  .vim-badge {
    font-size: 10px;
    padding: 1px 4px;
    background: var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    border-radius: 3px;
    font-weight: 700;
  }

  .vim-btn.active .vim-badge {
    background: var(--accent, #0969da);
    color: #ffffff;
  }

  .selection-badge {
    font-size: 9px;
    padding: 1px 4px;
    background: var(--accent-bg, rgba(9, 105, 218, 0.12));
    color: var(--accent, #0969da);
    border-radius: 3px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .divider {
    width: 1px;
    height: 12px;
    background-color: var(--border-primary, #d0d7de);
  }

  .icon {
    width: 12px;
    height: 12px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot.synced {
    background-color: #3fb950;
    box-shadow: 0 0 6px rgba(63, 185, 80, 0.6);
  }

  .dot.saving {
    background-color: #d29922;
  }

  .dot.error {
    background-color: #f85149;
  }
</style>
