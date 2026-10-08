<script lang="ts">
  import { tick } from "svelte";
  import { DropdownMenu, Popover, Toggle } from 'bits-ui';
  import {
    Command,
    Sparkles,
    Code,
    BookOpen,
    ChevronDown,
    Check,
    Search,
    Sun,
    Moon,
    Database,
    Loader2,
    AlertCircle,
    FileText,
    RefreshCw,
  } from 'lucide-svelte';
  import { appSettings } from '@entities/settings';
  import { searchRepository, isTauriEnvironment, type FullTextIndexStatus } from '@shared/repositories';
  import { listen } from '@tauri-apps/api/event';

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

  function selectMarkdownView(mode: MarkdownViewMode) {
    isViewMenuOpen = false;
    if (onChangeMarkdownView) {
      onChangeMarkdownView(mode);
    } else if (onToggleMarkdownView) {
      onToggleMarkdownView();
    }
  }

  // Estados para el selector de codificación
  let isEncodingMenuOpen = $state(false);
  let searchQuery = $state('');
  let selectedMenuIndex = $state(0);
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

  // Estados para el monitor de indexación de fondo
  let indexStatus = $state<FullTextIndexStatus | null>(null);
  let isIndexingMenuOpen = $state(false);
  let isRebuilding = $state(false);

  async function refreshIndexStatus() {
    try {
      const s = await searchRepository.getStatus();
      if (s) {
        indexStatus = s;
      }
    } catch {}
  }

  async function handleRebuildIndex() {
    if (isRebuilding) return;
    try {
      isRebuilding = true;
      await searchRepository.rebuildIndex();
      await refreshIndexStatus();
    } catch (e) {
      console.error('Error al reconstruir índice:', e);
    } finally {
      isRebuilding = false;
    }
  }

  $effect(() => {
    refreshIndexStatus();

    let unlisten: (() => void) | undefined;
    if (isTauriEnvironment()) {
      listen<FullTextIndexStatus>('vault:indexing-status', (event) => {
        indexStatus = event.payload;
      }).then((u) => {
        unlisten = u;
      }).catch((e) => {
        console.warn('Error escuchando eventos de indexación:', e);
      });
    }

    return () => {
      if (unlisten) unlisten();
    };
  });

  $effect(() => {
    if (!indexStatus?.is_indexing) return;
    const interval = setInterval(async () => {
      await refreshIndexStatus();
    }, 800);
    return () => clearInterval(interval);
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
      <Command size={14} class="icon" />
      <span>Ctrl+P</span>
    </button>

    <div class="divider"></div>

    <div class="status-item">
      <span class="dot {syncStatus}"></span>
      <span>{syncStatus === 'synced' ? 'Guardado' : syncStatus === 'saving' ? 'Guardando...' : 'Error de guardado'}</span>
    </div>

    <div class="divider"></div>

    <!-- Monitor de Indexación en Segundo Plano -->
    <div class="indexing-container">
      <Popover.Root
        bind:open={isIndexingMenuOpen}
        onOpenChange={(open) => {
          if (open) refreshIndexStatus();
        }}
      >
        <Popover.Trigger>
          {#snippet child({ props })}
            <button
              type="button"
              class="status-item clickable indexing-btn"
              class:indexing={indexStatus?.is_indexing}
              class:error={Boolean(indexStatus?.last_error)}
              class:active={isIndexingMenuOpen}
              title="Estado de indexación y búsqueda en segundo plano (clic para ver progreso)"
              {...props}
            >
              {#if indexStatus?.is_indexing}
                <Loader2 size={13} class="icon spin indexing-icon" />
                <span class="indexing-text">
                  Indexando
                  {#if indexStatus.total_to_index && indexStatus.total_to_index > 0}
                    ({indexStatus.indexed_in_batch || 0}/{indexStatus.total_to_index})
                  {/if}
                </span>
              {:else if indexStatus?.last_error}
                <AlertCircle size={13} class="icon error-icon" />
                <span>Error índice</span>
              {:else}
                <Database size={13} class="icon" />
                <span>Índice al día</span>
              {/if}
            </button>
          {/snippet}
        </Popover.Trigger>

        <Popover.Portal>
          <Popover.Content class="indexing-popover" side="top" align="start" sideOffset={6}>
            <div class="popover-header">
              <div class="header-left">
                <Database size={14} class="header-icon" />
                <span class="header-title">Índice de Búsqueda</span>
              </div>
              <span
                class="status-pill"
                class:pill-indexing={indexStatus?.is_indexing}
                class:pill-ready={!indexStatus?.is_indexing && !indexStatus?.last_error}
                class:pill-error={Boolean(indexStatus?.last_error)}
              >
                {#if indexStatus?.is_indexing}
                  <span class="pulse-dot"></span>
                  Indexando
                {:else if indexStatus?.last_error}
                  Error
                {:else}
                  Al día
                {/if}
              </span>
            </div>

            <div class="popover-body">
              {#if indexStatus?.is_indexing}
                <div class="progress-section">
                  <div class="progress-info">
                    <span class="progress-title">Indexando documentos...</span>
                    <span class="progress-count">
                      {indexStatus.indexed_in_batch || 0} de {indexStatus.total_to_index || 0}
                    </span>
                  </div>
                  <div class="progress-bar-bg">
                    <div
                      class="progress-bar-fill"
                      style="width: {indexStatus.total_to_index ? Math.min(100, Math.round(((indexStatus.indexed_in_batch || 0) / indexStatus.total_to_index) * 100)) : 0}%"
                    ></div>
                  </div>
                </div>

                {#if indexStatus.current_file}
                  <div class="current-file-section">
                    <span class="section-label">Procesando ahora:</span>
                    <div class="file-badge">
                      <FileText size={12} class="file-icon" />
                      <span class="file-name" title={indexStatus.current_file}>{indexStatus.current_file}</span>
                    </div>
                  </div>
                {/if}
              {:else}
                <div class="stats-grid">
                  <div class="stat-card">
                    <span class="stat-num">{indexStatus?.indexed_docs || 0}</span>
                    <span class="stat-label">Documentos indexados</span>
                  </div>
                  <div class="stat-card">
                    <span class="stat-num">{indexStatus?.pending || 0}</span>
                    <span class="stat-label">Pendientes en cola</span>
                  </div>
                </div>

                <div class="engine-info">
                  <span class="engine-label">Motor:</span>
                  <span class="engine-value">
                    {indexStatus?.in_memory_fallback ? 'Memoria RAM (Fallback)' : 'Tantivy en disco (.synapse/fts)'}
                  </span>
                </div>
              {/if}

              {#if indexStatus?.recent_files && indexStatus.recent_files.length > 0}
                <div class="recent-files-section">
                  <span class="section-label">Archivos procesados recientemente:</span>
                  <div class="recent-files-list">
                    {#each indexStatus.recent_files.slice(-5).reverse() as file}
                      <div class="recent-file-item">
                        <FileText size={12} class="recent-file-icon" />
                        <span class="recent-file-name" title={file}>{file}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if indexStatus?.last_error}
                <div class="error-box">
                  <AlertCircle size={14} class="error-box-icon" />
                  <span>{indexStatus.last_error}</span>
                </div>
              {/if}
            </div>

            <div class="popover-footer">
              <button
                type="button"
                class="rebuild-btn"
                disabled={isRebuilding || indexStatus?.is_indexing}
                onclick={handleRebuildIndex}
              >
                {#if isRebuilding}
                  <Loader2 size={13} class="icon spin" />
                  <span>Reconstruyendo...</span>
                {:else}
                  <RefreshCw size={13} class="icon" />
                  <span>Reindexar bóveda</span>
                {/if}
              </button>
            </div>
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
    </div>
  </div>

  <div class="right-group">
    <!-- Selector desplegable de Modo de Vista Markdown (En vivo / Fuente / Lectura) -->
    {#if isMarkdownFile}
      <div class="md-view-container">
        <DropdownMenu.Root bind:open={isViewMenuOpen}>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <button
                type="button"
                class="status-item clickable md-view-btn"
                class:live-mode={markdownViewMode === 'live'}
                class:source-mode={markdownViewMode === 'source'}
                class:reading-mode={markdownViewMode === 'reading'}
                class:active={isViewMenuOpen}
                title="Cambiar modo de vista Markdown: En vivo, Fuente o Lectura"
                {...props}
              >
                {#if markdownViewMode === 'live'}
                  <Sparkles size={14} class="icon" />
                  <span>En vivo</span>
                {:else if markdownViewMode === 'source'}
                  <Code size={14} class="icon" />
                  <span>Fuente</span>
                {:else}
                  <BookOpen size={14} class="icon" />
                  <span>Lectura</span>
                {/if}
                <ChevronDown size={12} class="chevron-icon {isViewMenuOpen ? 'open' : ''}" />
              </button>
            {/snippet}
          </DropdownMenu.Trigger>

          <DropdownMenu.Portal>
            <DropdownMenu.Content class="view-dropdown" side="top" align="end" sideOffset={6}>
              <div class="view-dropdown-header">
                <span class="header-title">Vista Markdown</span>
                <span class="header-current">
                  {markdownViewMode === 'live' ? 'En vivo' : markdownViewMode === 'source' ? 'Fuente' : 'Lectura'}
                </span>
              </div>

              <div class="view-options-list">
                {#each MARKDOWN_VIEW_MODES as item}
                  {@const isCurrent = markdownViewMode === item.id}
                  <DropdownMenu.Item
                    class="view-item {isCurrent ? 'current' : ''} mode-{item.id}"
                    onSelect={() => selectMarkdownView(item.id)}
                  >
                    <div class="item-icon-box">
                      {#if item.id === 'live'}
                        <Sparkles size={14} class="item-icon" />
                      {:else if item.id === 'source'}
                        <Code size={14} class="item-icon" />
                      {:else}
                        <BookOpen size={14} class="item-icon" />
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
                      <Check size={14} class="check-icon" />
                    {/if}
                  </DropdownMenu.Item>
                {/each}
              </div>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      </div>
      <div class="divider"></div>
    {/if}

    <Toggle.Root
      class="status-item clickable vim-btn"
      pressed={isVimMode}
      onPressedChange={() => { if (onToggleVim) onToggleVim(); }}
      title={isVimMode ? 'Desactivar modo VIM en el editor' : 'Activar modo VIM en el editor'}
    >
      <span class="vim-badge">VIM</span>
      <span>{isVimMode ? 'ON' : 'OFF'}</span>
    </Toggle.Root>
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
    <div class="encoding-container">
      <Popover.Root
        bind:open={isEncodingMenuOpen}
        onOpenChange={(open) => {
          if (open) {
            searchQuery = '';
            const currentIdx = filteredEncodings.findIndex(e => isCurrentEncoding(e.id, e.label));
            selectedMenuIndex = currentIdx !== -1 ? currentIdx : 0;
            tick().then(() => {
              searchInputRef?.focus();
              scrollSelectedIntoView();
            });
          }
        }}
      >
        <Popover.Trigger>
          {#snippet child({ props })}
            <button
              type="button"
              class="status-item clickable encoding-btn"
              class:active={isEncodingMenuOpen}
              title="Cambiar codificación del archivo y guardar con la nueva codificación"
              {...props}
            >
              <span class="encoding-text">{encoding || '---'}</span>
              <ChevronDown size={12} class="chevron-icon {isEncodingMenuOpen ? 'open' : ''}" />
            </button>
          {/snippet}
        </Popover.Trigger>

        <Popover.Portal>
          <Popover.Content class="encoding-dropdown" side="top" align="end" sideOffset={6}>
            <div class="encoding-dropdown-header">
              <span class="header-title">Guardar con codificación</span>
              <span class="header-current">Actual: {encoding || '---'}</span>
            </div>

            <div class="encoding-search-box">
              <Search size={14} class="search-icon" />
              <input
                type="text"
                class="encoding-search-input"
                placeholder="Buscar codificación..."
                bind:value={searchQuery}
                bind:this={searchInputRef}
                onkeydown={handleKeydown}
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
                      <Check size={14} class="check-icon" />
                    {/if}
                  </button>
                {/each}
              {/if}
            </div>
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
    </div>

    <div class="divider"></div>

    <!-- Botón de Alternar Tema (Claro / Oscuro / Sistema) -->
    <button
      type="button"
      class="status-item clickable theme-btn"
      onclick={() => appSettings.cycleTheme()}
      title="Cambiar tema: {appSettings.theme === 'system' ? `Sistema (${appSettings.resolvedTheme === 'dark' ? 'Oscuro' : 'Claro'})` : appSettings.theme === 'dark' ? 'Oscuro' : 'Claro'} (Clic para alternar)"
    >
      {#if appSettings.resolvedTheme === 'dark'}
        <Moon size={13} class="icon theme-icon" />
      {:else}
        <Sun size={13} class="icon theme-icon" />
      {/if}
      <span class="theme-text">{appSettings.theme === 'system' ? 'Auto' : appSettings.theme === 'dark' ? 'Oscuro' : 'Claro'}</span>
    </button>
  </div>
</footer>

<style>
  .status-bar {
    height: var(--status-bar-height, 28px);
    background-color: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    z-index: 20;
    flex-shrink: 0;
  }

  .left-group, .right-group {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    height: 20px;
    background: transparent;
    border: none;
    color: inherit;
    cursor: default;
    border-radius: 4px;
    transition: all 0.15s ease;
    font-size: inherit;
    font-family: inherit;
  }

  .status-item.clickable {
    cursor: pointer;
  }

  .status-item.clickable:hover,
  .status-item.clickable.active {
    background: var(--hover-bg, rgba(0, 0, 0, 0.05));
    color: var(--accent, #0969da);
  }

  .theme-btn {
    font-weight: 500;
  }

  .theme-icon {
    color: var(--accent, #0969da);
  }

  .theme-text {
    font-size: 11px;
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
  :global(.view-dropdown) {
    width: 290px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.14));
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

  :global(.view-dropdown .view-dropdown-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.view-dropdown .view-options-list) {
    padding: 4px 0;
    display: flex;
    flex-direction: column;
  }

  :global(.view-dropdown .view-item) {
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

  :global(.view-dropdown .view-item:hover),
  :global(.view-dropdown .view-item[data-highlighted]) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.04));
  }

  :global(.view-dropdown .view-item.current) {
    background: rgba(9, 105, 218, 0.05);
  }

  :global(.view-dropdown .view-item.current:hover),
  :global(.view-dropdown .view-item.current[data-highlighted]) {
    background: rgba(9, 105, 218, 0.1);
  }

  :global(.view-dropdown .item-icon-box) {
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

  :global(.view-dropdown .view-item.mode-live .item-icon-box) {
    color: #1a7f37;
    background: rgba(46, 160, 67, 0.1);
  }

  :global(.view-dropdown .view-item.mode-source .item-icon-box) {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.1);
  }

  :global(.view-dropdown .view-item.mode-reading .item-icon-box) {
    color: #8250df;
    background: rgba(130, 80, 223, 0.1);
  }

  :global(.view-dropdown .item-icon) {
    width: 14px;
    height: 14px;
  }

  :global(.view-dropdown .item-text) {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  :global(.view-dropdown .item-title-row) {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.view-dropdown .item-label) {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  :global(.view-dropdown .item-badge) {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 4px;
    font-weight: 500;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.view-dropdown .view-item.current.mode-live .item-label) {
    color: #1a7f37;
  }

  :global(.view-dropdown .view-item.current.mode-source .item-label) {
    color: var(--accent, #0969da);
  }

  :global(.view-dropdown .view-item.current.mode-reading .item-label) {
    color: #8250df;
  }

  :global(.view-dropdown .item-desc) {
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
  :global(.encoding-dropdown) {
    width: 290px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.14));
    z-index: 1000;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: fadeInSlideUp 0.15s ease-out;
    outline: none;
  }

  :global(.encoding-dropdown .encoding-dropdown-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.header-title) {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-primary, #1f2328);
  }

  :global(.header-current) {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
  }

  :global(.encoding-dropdown .encoding-search-box) {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    gap: 6px;
    background: var(--bg-primary, #ffffff);
  }

  :global(.encoding-dropdown .search-icon) {
    width: 12px;
    height: 12px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }

  :global(.encoding-dropdown .encoding-search-input) {
    flex: 1;
    border: none;
    outline: none;
    font-size: 11px;
    background: transparent;
    color: var(--text-primary, #1f2328);
  }

  :global(.encoding-dropdown .encoding-options-list) {
    max-height: 240px;
    overflow-y: auto;
    padding: 4px 0;
  }

  :global(.encoding-dropdown .empty-results) {
    padding: 12px;
    text-align: center;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }

  :global(.encoding-dropdown .encoding-item) {
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

  :global(.encoding-dropdown .encoding-item.selected),
  :global(.encoding-dropdown .encoding-item:hover) {
    background: var(--accent-bg, rgba(9, 105, 218, 0.08));
  }

  :global(.encoding-dropdown .encoding-item.current .item-label) {
    color: var(--accent, #0969da);
    font-weight: 600;
  }

  :global(.check-icon) {
    width: 13px;
    height: 13px;
    color: var(--accent, #0969da);
    flex-shrink: 0;
  }

  .vim-btn {
    font-weight: 600;
  }

  :global(.vim-btn[data-state="on"]),
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

  :global(.vim-btn[data-state="on"] .vim-badge) {
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
    box-shadow: 0 0 6px rgba(210, 153, 34, 0.6);
  }

  .dot.error {
    background-color: #f85149;
    box-shadow: 0 0 6px rgba(248, 81, 73, 0.6);
  }

  /* Contenedor y botón de indexación */
  .indexing-container {
    position: relative;
    display: flex;
    align-items: center;
  }

  .indexing-btn {
    font-weight: 500;
    gap: 5px;
    display: flex;
    align-items: center;
  }

  .indexing-btn.indexing {
    color: var(--accent, #0969da);
    background: rgba(9, 105, 218, 0.08);
    font-weight: 600;
  }

  .indexing-btn.error {
    color: #cf222e;
    background: rgba(207, 34, 46, 0.08);
  }

  .indexing-icon {
    color: var(--accent, #0969da);
  }

  .error-icon {
    color: #cf222e;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  /* Popover flotante de estado de indexación */
  :global(.indexing-popover) {
    width: 320px;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow, 0 8px 24px rgba(0, 0, 0, 0.14));
    z-index: 1000;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: fadeInSlideUp 0.15s ease-out;
    outline: none;
  }

  :global(.indexing-popover .popover-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }

  :global(.indexing-popover .header-left) {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.indexing-popover .header-icon) {
    color: var(--accent, #0969da);
  }

  :global(.indexing-popover .header-title) {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  :global(.indexing-popover .status-pill) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 12px;
  }

  :global(.indexing-popover .pill-ready) {
    background: rgba(46, 160, 67, 0.12);
    color: #1a7f37;
    border: 1px solid rgba(46, 160, 67, 0.25);
  }

  :global(.indexing-popover .pill-indexing) {
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
    border: 1px solid rgba(9, 105, 218, 0.25);
  }

  :global(.indexing-popover .pill-error) {
    background: rgba(207, 34, 46, 0.12);
    color: #cf222e;
    border: 1px solid rgba(207, 34, 46, 0.25);
  }

  :global(.indexing-popover .pulse-dot) {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent, #0969da);
    animation: pulse 1.2s infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.4;
      transform: scale(0.85);
    }
  }

  :global(.indexing-popover .popover-body) {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(.indexing-popover .progress-section) {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  :global(.indexing-popover .progress-info) {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-primary, #1f2328);
  }

  :global(.indexing-popover .progress-title) {
    font-weight: 500;
  }

  :global(.indexing-popover .progress-count) {
    font-weight: 600;
    color: var(--accent, #0969da);
  }

  :global(.indexing-popover .progress-bar-bg) {
    width: 100%;
    height: 6px;
    background: var(--border-primary, #d0d7de);
    border-radius: 3px;
    overflow: hidden;
  }

  :global(.indexing-popover .progress-bar-fill) {
    height: 100%;
    background: var(--accent, #0969da);
    border-radius: 3px;
    transition: width 0.2s ease;
  }

  :global(.indexing-popover .current-file-section) {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  :global(.indexing-popover .section-label) {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  :global(.indexing-popover .file-badge) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
    font-family: monospace;
    font-size: 10px;
    color: var(--text-primary, #1f2328);
    overflow: hidden;
  }

  :global(.indexing-popover .file-name) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.indexing-popover .stats-grid) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  :global(.indexing-popover .stat-card) {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
  }

  :global(.indexing-popover .stat-num) {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-primary, #1f2328);
  }

  :global(.indexing-popover .stat-label) {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
  }

  :global(.indexing-popover .engine-info) {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--text-secondary, #656d76);
    padding: 0 2px;
  }

  :global(.indexing-popover .engine-value) {
    font-weight: 500;
  }

  :global(.indexing-popover .recent-files-section) {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  :global(.indexing-popover .recent-files-list) {
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-height: 100px;
    overflow-y: auto;
  }

  :global(.indexing-popover .recent-file-item) {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    font-family: monospace;
    color: var(--text-secondary, #656d76);
    overflow: hidden;
  }

  :global(.indexing-popover .recent-file-name) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.indexing-popover .error-box) {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 8px;
    border-radius: 6px;
    background: rgba(207, 34, 46, 0.08);
    border: 1px solid rgba(207, 34, 46, 0.25);
    color: #cf222e;
    font-size: 10px;
  }

  :global(.indexing-popover .popover-footer) {
    padding: 8px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    display: flex;
    justify-content: flex-end;
  }

  :global(.indexing-popover .rebuild-btn) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 500;
    background: var(--bg-primary, #ffffff);
    border: 1px solid var(--border-primary, #d0d7de);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  :global(.indexing-popover .rebuild-btn:hover:not(:disabled)) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.05));
    border-color: var(--accent, #0969da);
    color: var(--accent, #0969da);
  }

  :global(.indexing-popover .rebuild-btn:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
