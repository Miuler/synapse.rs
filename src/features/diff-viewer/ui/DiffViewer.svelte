<script lang="ts">
  import { DiffView, DiffModeEnum, DiffFile } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view.css';
  import { vaultRepository, type GitDiffResponse } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
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
    CheckCircle2
  } from 'lucide-svelte';

  interface Props {
    filePath: string;
    isStaged: boolean;
    onClose?: () => void;
  }

  let { filePath, isStaged, onClose }: Props = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let diffData = $state<GitDiffResponse | null>(null);
  let diffFileInstance = $state<DiffFile | null>(null);

  let viewMode = $state<DiffModeEnum>(DiffModeEnum.Split);
  let showRawDiff = $state(false);
  let enableWrap = $state(false);

  let currentTheme = $derived<'light' | 'dark'>(
    appSettings.resolvedTheme === 'dark' ? 'dark' : 'light'
  );

  async function fetchDiff() {
    loading = true;
    error = null;
    try {
      const data = await vaultRepository.getGitFileDiff(filePath, isStaged);
      diffData = data;
    } catch (err: unknown) {
      console.error('Error al cargar git diff:', err);
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Escuchar cambios en filePath o isStaged para recargar
    const _p = filePath;
    const _s = isStaged;
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
        try {
          instance.onAllExpand('split');
          instance.onAllExpand('unified');
        } catch {}
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
      <div class="diff-file-badge" class:is-staged={isStaged} title={isStaged ? 'Cambios en el stage (índice)' : 'Cambios en el área de trabajo'}>
        {#if isStaged}
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
      <!-- MODO DE VISTA: SPLIT, UNIFIED O TEXTO PLANO -->
      <div class="view-mode-toggle" role="group" aria-label="Modo de vista diff">
        <button
          type="button"
          class="toggle-btn"
          class:active={!showRawDiff && viewMode === DiffModeEnum.Split}
          onclick={() => {
            showRawDiff = false;
            viewMode = DiffModeEnum.Split;
          }}
          title="Vista dividida (Split)"
        >
          <Columns size={13} />
          <span>Dividida</span>
        </button>
        <button
          type="button"
          class="toggle-btn"
          class:active={!showRawDiff && viewMode === DiffModeEnum.Unified}
          onclick={() => {
            showRawDiff = false;
            viewMode = DiffModeEnum.Unified;
          }}
          title="Vista unificada (Unified)"
        >
          <AlignJustify size={13} />
          <span>Unificada</span>
        </button>
        <button
          type="button"
          class="toggle-btn"
          class:active={showRawDiff}
          onclick={() => (showRawDiff = true)}
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
    {#if loading}
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
          No hay cambios en <strong>{fileName}</strong> para {isStaged ? 'el área de preparación (stage)' : 'el árbol de trabajo'}.
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
