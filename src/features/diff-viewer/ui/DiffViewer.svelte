<script lang="ts">
  import { DiffView, DiffModeEnum, DiffFile } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view.css';
  import { vaultRepository, type GitDiffResponse } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
  import {
    isImageFile,
    isSvgFile,
    isPdfFile,
    isDrawingFile,
    isDiagramFile,
    DEFAULT_APP_CONFIG,
  } from '@entities/file-type';
  import VisualDiffViewer from './VisualDiffViewer.svelte';
  import ImageHighlightDiffViewer from './ImageHighlightDiffViewer.svelte';
  import { workspaceDiffSettings, type DiffVisualTab } from '../model/diff-settings.svelte';
  import {
    GitCompare,
    GitCompareArrows,
    RotateCw,
    WrapText,
    Columns,
    AlignJustify,
    FileCode,
    ChevronsUpDown,
    ChevronsDownUp,
    AlertCircle,
    CheckCircle2,
    Eye,
    Sparkles,
    History
  } from 'lucide-svelte';

  interface Props {
    filePath: string;
    isStaged?: boolean;
    baseRevision?: string;
    onClose?: () => void;
  }

  let { filePath, isStaged = false, baseRevision, onClose }: Props = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let diffData = $state<GitDiffResponse | null>(null);
  let diffFileInstance = $state<DiffFile | null>(null);

  const isDrawing = $derived(isDrawingFile(filePath));
  const isDiagram = $derived(isDiagramFile(filePath));
  const isPdf = $derived(isPdfFile(filePath));
  const isSvg = $derived(isSvgFile(filePath));
  const isImage = $derived(isImageFile(filePath) && !isDrawing);
  const isPureBinary = $derived((isImage && !isSvg) || isPdf);
  const isVisualCandidate = $derived(
    isImage || isPdf || isDrawing || isDiagram
  );

  $effect(() => {
    workspaceDiffSettings.initFromWorkspace();
  });

  const activeVisualTab = $derived.by<DiffVisualTab>(() => {
    if (!isVisualCandidate) return 'text';
    if (workspaceDiffSettings.visualTab === 'highlight' && !(isImage && !isSvg)) {
      return 'side_by_side';
    }
    if (isPureBinary && workspaceDiffSettings.visualTab === 'text') {
      return 'side_by_side';
    }
    return workspaceDiffSettings.visualTab;
  });

  const viewMode = $derived(
    workspaceDiffSettings.textMode === 'unified'
      ? DiffModeEnum.Unified
      : DiffModeEnum.Split
  );
  const showRawDiff = $derived(workspaceDiffSettings.textMode === 'raw');
  let enableWrap = $state(false);

  let currentTheme = $derived<'light' | 'dark'>(
    appSettings.resolvedTheme === 'dark' ? 'dark' : 'light'
  );

  async function fetchDiff() {
    loading = true;
    error = null;
    try {
      const data = await vaultRepository.getGitFileDiff(filePath, isStaged, baseRevision);
      diffData = data;
    } catch (err: unknown) {
      console.error('Error al cargar git diff:', err);
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Escuchar cambios en filePath, isStaged o baseRevision para recargar
    const _p = filePath;
    const _s = isStaged;
    const _r = baseRevision;
    fetchDiff();
  });

  const fileName = $derived(filePath.split('/').pop() || filePath);

  $effect(() => {
    if (diffData?.diff && diffData.diff.trim().length > 0) {
      try {
        const instance = new DiffFile(
          diffData.old_file_name || fileName,
          diffData.old_content ?? '',
          diffData.new_file_name || fileName,
          diffData.new_content ?? '',
          [diffData.diff]
        );
        instance.initTheme(currentTheme);
        instance.initRaw();
        instance.buildSplitDiffLines();
        instance.buildUnifiedDiffLines();

        // Expansión automática segura basada en el límite configurable de config.jsonc
        // Evita colgar la interfaz en archivos extensos como .excalidraw o JSON minificados
        const maxExpandLines = DEFAULT_APP_CONFIG.diff?.max_expand_lines ?? 800;
        const totalLines = Math.max(
          instance.splitLineLength ?? 0,
          instance.unifiedLineLength ?? 0,
          diffData.old_content ? diffData.old_content.split('\n').length : 0,
          diffData.new_content ? diffData.new_content.split('\n').length : 0
        );

        if (totalLines <= maxExpandLines) {
          try {
            instance.onAllExpand('split');
            instance.onAllExpand('unified');
          } catch {}
        }

        diffFileInstance = instance;
      } catch (err) {
        console.error('Error inicializando DiffFile:', err);
        diffFileInstance = null;
      }
    } else {
      diffFileInstance = null;
    }
  });

  // Estadísticas de líneas agregadas / eliminadas
  let diffStats = $derived.by(() => {
    if (!diffData?.diff) return { added: 0, deleted: 0 };
    const lines = diffData.diff.split('\n');
    let added = 0;
    let deleted = 0;
    for (const line of lines) {
      if (line.startsWith('+++') || line.startsWith('---')) continue;
      if (line.startsWith('+')) added++;
      else if (line.startsWith('-')) deleted++;
    }
    return { added, deleted };
  });

  let hasDiff = $derived(
    Boolean(diffData?.diff && diffData.diff.trim().length > 0)
  );

  function handleExpandAll() {
    const mode = viewMode === DiffModeEnum.Split ? 'split' : 'unified';
    diffFileInstance?.onAllExpand?.(mode);
  }

  function handleCollapseAll() {
    const mode = viewMode === DiffModeEnum.Split ? 'split' : 'unified';
    diffFileInstance?.onAllCollapse?.(mode);
  }
</script>

<div class="diff-viewer-wrapper">
  <!-- BARRA DE HERRAMIENTAS SUPERIOR -->
  <header class="diff-toolbar">
    <div class="diff-toolbar-left">
      <div
        class="diff-file-badge"
        class:is-staged={isStaged && !baseRevision}
        class:is-revision={Boolean(baseRevision)}
        title={baseRevision ? `Comparando con revisión ${baseRevision}` : (isStaged ? 'Cambios en el stage (índice)' : 'Cambios en el área de trabajo')}
      >
        {#if baseRevision}
          <History size={14} class="badge-icon" />
          <span>REV: {baseRevision.slice(0, 7)}</span>
        {:else if isStaged}
          <GitCompareArrows size={14} class="badge-icon" />
          <span>STAGED</span>
        {:else}
          <GitCompare size={14} class="badge-icon" />
          <span>WORKING TREE</span>
        {/if}
      </div>

      <div class="diff-file-info" title={filePath}>
        <span class="diff-file-name">{fileName}</span>
        <span class="diff-file-path">{filePath}</span>
      </div>

      {#if hasDiff}
        <div class="diff-counts">
          {#if diffStats.added > 0}
            <span class="stat-added">+{diffStats.added}</span>
          {/if}
          {#if diffStats.deleted > 0}
            <span class="stat-deleted">-{diffStats.deleted}</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="diff-toolbar-right">
      {#if isVisualCandidate}
        <div class="view-mode-toggle" role="group" aria-label="Modo de visualización diff">
          <!-- 1. LADO A LADO -->
          <button
            type="button"
            class="toggle-btn"
            class:active={activeVisualTab === 'side_by_side'}
            onclick={() => workspaceDiffSettings.setVisualTab('side_by_side')}
            title="Vista dividida lado a lado (Disco vs Git)"
          >
            <Columns size={13} />
            <span>2 Paneles</span>
          </button>

          <!-- 2. RESALTADO PERCEPTUAL (Solo imágenes raster, no SVG) -->
          {#if isImage && !isSvg}
            <button
              type="button"
              class="toggle-btn"
              class:active={activeVisualTab === 'highlight'}
              onclick={() => workspaceDiffSettings.setVisualTab('highlight')}
              title="Diff perceptual: píxeles modificados resaltados en memoria"
            >
              <Sparkles size={13} />
              <span>Diferencias</span>
            </button>
          {/if}

          <!-- 3. MODO TEXTO (Solo si NO es un binario puro como imagen o PDF) -->
          {#if !isPureBinary}
            <button
              type="button"
              class="toggle-btn"
              class:active={activeVisualTab === 'text'}
              onclick={() => workspaceDiffSettings.setVisualTab('text')}
              title="Vista de código / diff de texto"
            >
              <FileCode size={13} />
              <span>Texto</span>
            </button>
          {/if}
        </div>
      {/if}

      <!-- SUBOPCIONES DE TEXTO (Solo si no es binario y está en pestaña texto) -->
      {#if !isPureBinary && (!isVisualCandidate || activeVisualTab === 'text')}
        <!-- MODO DE VISTA: SPLIT, UNIFIED O TEXTO PLANO -->
        <div class="view-mode-toggle" role="group" aria-label="Modo de vista diff">
          <button
            type="button"
            class="toggle-btn"
            class:active={!showRawDiff && viewMode === DiffModeEnum.Split}
            onclick={() => workspaceDiffSettings.setTextMode('split')}
            title="Vista dividida (Split)"
          >
            <Columns size={13} />
            <span>Dividida</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showRawDiff && viewMode === DiffModeEnum.Unified}
            onclick={() => workspaceDiffSettings.setTextMode('unified')}
            title="Vista unificada (Unified)"
          >
            <AlignJustify size={13} />
            <span>Unificada</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showRawDiff}
            onclick={() => workspaceDiffSettings.setTextMode('raw')}
            title="Vista de parche / texto sin formato (Raw diff)"
          >
            <FileCode size={13} />
            <span>Parche</span>
          </button>
        </div>

        {#if !showRawDiff && diffFileInstance}
          <!-- EXPANDIR / COLAPSAR TODO -->
          <button
            type="button"
            class="action-btn"
            onclick={handleExpandAll}
            title="Expandir todas las secciones"
          >
            <ChevronsUpDown size={14} />
          </button>
          <button
            type="button"
            class="action-btn"
            onclick={handleCollapseAll}
            title="Colapsar secciones no modificadas"
          >
            <ChevronsDownUp size={14} />
          </button>
        {/if}

        <!-- AJUSTE DE LÍNEA (WRAP) -->
        <button
          type="button"
          class="action-btn"
          class:active={enableWrap}
          onclick={() => (enableWrap = !enableWrap)}
          title={enableWrap ? 'Desactivar ajuste de línea' : 'Activar ajuste de línea'}
        >
          <WrapText size={14} />
        </button>
      {/if}

      <!-- RECARGAR DIFF -->
      <button
        type="button"
        class="action-btn"
        disabled={loading}
        onclick={fetchDiff}
        title="Recargar diferencias (F5)"
      >
        <RotateCw size={14} class={loading ? 'spinning' : ''} />
      </button>
    </div>
  </header>

  <!-- CONTENIDO PRINCIPAL -->
  <main class="diff-body">
    {#if isVisualCandidate && activeVisualTab === 'side_by_side'}
      <VisualDiffViewer {filePath} {isStaged} {baseRevision} />
    {:else if isImage && !isSvg && activeVisualTab === 'highlight'}
      <ImageHighlightDiffViewer {filePath} {isStaged} {baseRevision} />
    {:else if loading}
      <div class="diff-state-container">
        <div class="spinner"></div>
        <p class="diff-state-text">Cargando diferencias de Git...</p>
      </div>
    {:else if error}
      <div class="diff-state-container error">
        <div class="state-icon error-icon">
          <AlertCircle size={32} />
        </div>
        <h3 class="state-title">Error al generar diff</h3>
        <p class="state-desc">{error}</p>
        <button type="button" class="retry-btn" onclick={fetchDiff}>
          Reintentar
        </button>
      </div>
    {:else if !hasDiff}
      <div class="diff-state-container empty">
        <div class="state-icon success-icon">
          <CheckCircle2 size={32} />
        </div>
        <h3 class="state-title">Sin diferencias</h3>
        <p class="state-desc">
          No hay cambios en <strong>{fileName}</strong> {baseRevision ? `respecto a la revisión ${baseRevision.slice(0, 7)}` : (isStaged ? 'para el área de preparación (stage)' : 'para el árbol de trabajo')}.
        </p>
      </div>
    {:else if diffData}
      {#if showRawDiff || !diffFileInstance}
        <div class="diff-raw-container">
          <pre class="diff-raw-pre" class:wrap={enableWrap}><code>{#each diffData.diff.split('\n') as line}
            <div
              class="diff-raw-line"
              class:add={line.startsWith('+') && !line.startsWith('+++')}
              class:del={line.startsWith('-') && !line.startsWith('---')}
              class:hunk={line.startsWith('@@')}
              class:header={line.startsWith('diff --git') || line.startsWith('index ') || line.startsWith('---') || line.startsWith('+++')}
            >{line}</div>
          {/each}</code></pre>
        </div>
      {:else}
        <div class="diff-view-scroller">
          <DiffView
            diffFile={diffFileInstance}
            diffViewMode={viewMode}
            diffViewTheme={currentTheme}
            diffViewWrap={enableWrap}
            diffViewHighlight={true}
          />
        </div>
      {/if}
    {/if}
  </main>
</div>

<style>
  .diff-viewer-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
  }

  .diff-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 38px;
    min-height: 38px;
    padding: 0 14px;
    border-bottom: 1px solid var(--border-secondary);
    background-color: var(--bg-secondary);
    gap: 12px;
    user-select: none;
  }

  .diff-toolbar-left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    overflow: hidden;
  }

  .diff-file-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.4px;
    background-color: rgba(59, 130, 246, 0.15);
    color: #3b82f6;
    border: 1px solid rgba(59, 130, 246, 0.3);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .diff-file-badge.is-staged {
    background-color: rgba(34, 197, 94, 0.15);
    color: #22c55e;
    border-color: rgba(34, 197, 94, 0.3);
  }

  .diff-file-badge.is-revision {
    background-color: rgba(147, 51, 234, 0.15);
    color: #9333ea;
    border-color: rgba(147, 51, 234, 0.3);
  }

  .diff-file-info {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-file-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .diff-file-path {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .diff-counts {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    margin-left: 4px;
    flex-shrink: 0;
  }

  .stat-added {
    color: #22c55e;
  }

  .stat-deleted {
    color: #ef4444;
  }

  .diff-toolbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .view-mode-toggle {
    display: inline-flex;
    align-items: center;
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 5px;
    padding: 2px;
    gap: 2px;
  }

  .toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    color: var(--text-primary);
  }

  .toggle-btn.active {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 5px;
    border: 1px solid var(--border-secondary);
    background-color: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover:not(:disabled) {
    color: var(--text-primary);
    background-color: var(--bg-primary);
  }

  .action-btn.active {
    color: var(--accent);
    border-color: var(--accent);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .diff-body {
    flex: 1;
    overflow: hidden;
    position: relative;
    background-color: var(--bg-primary);
  }

  .diff-view-scroller {
    width: 100%;
    height: 100%;
    overflow: auto;
  }

  .diff-state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 24px;
    text-align: center;
    color: var(--text-secondary);
  }

  .diff-state-text {
    font-size: 13px;
    margin-top: 12px;
  }

  .state-icon {
    margin-bottom: 12px;
  }

  .error-icon {
    color: #ef4444;
  }

  .success-icon {
    color: #22c55e;
  }

  .state-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 6px 0;
  }

  .state-desc {
    font-size: 13px;
    color: var(--text-muted);
    max-width: 420px;
    margin: 0 0 16px 0;
    line-height: 1.5;
  }

  .retry-btn {
    padding: 6px 16px;
    background-color: var(--accent);
    color: #fff;
    border: none;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .retry-btn:hover {
    opacity: 0.9;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-secondary);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  :global(.spinning) {
    animation: spin 0.8s linear infinite;
  }

  /* Personalización para integración fluida de @git-diff-view */
  :global(.diff-tailwindcss-wrapper) {
    font-family: inherit !important;
    background-color: transparent !important;
    width: 100% !important;
    min-height: 100% !important;
  }

  :global(.diff-tailwindcss-wrapper .split-diff-view) {
    width: 100% !important;
    min-width: 100% !important;
  }

  :global(.diff-tailwindcss-wrapper .unified-diff-view) {
    width: 100% !important;
    min-width: 100% !important;
  }

  :global(.diff-tailwindcss-wrapper table) {
    width: 100% !important;
  }

  .diff-raw-container {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 16px;
    background-color: var(--bg-primary);
    font-family: var(--code-font, monospace);
    font-size: 13px;
    line-height: 1.6;
    box-sizing: border-box;
  }

  .diff-raw-pre {
    margin: 0;
    white-space: pre;
    font-family: inherit;
  }

  .diff-raw-pre.wrap {
    white-space: pre-wrap;
    word-break: break-all;
  }

  .diff-raw-line {
    padding: 1px 8px;
    border-radius: 2px;
  }

  .diff-raw-line.add {
    background-color: rgba(34, 197, 94, 0.15);
    color: #16a34a;
  }

  .diff-raw-line.del {
    background-color: rgba(239, 68, 68, 0.15);
    color: #dc2626;
  }

  .diff-raw-line.hunk {
    background-color: rgba(59, 130, 246, 0.12);
    color: #2563eb;
    font-weight: 600;
  }

  .diff-raw-line.header {
    color: var(--text-muted);
    font-weight: 500;
  }
</style>
