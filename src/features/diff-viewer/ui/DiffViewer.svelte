<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import CodeMirrorDiffView, { type DiffChunksInfo } from './CodeMirrorDiffView.svelte';
  import { vaultRepository, type GitDiffResponse } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
  import {
    isImageFile,
    isSvgFile,
    isPdfFile,
    isDrawingFile,
    isDocxFile,
    isDiagramFile,
    isD2File,
    isPlantUmlFile,
    DEFAULT_APP_CONFIG,
  } from '@entities/file-type';
  import VisualDiffViewer from './VisualDiffViewer.svelte';
  import DocxDiffView from './DocxDiffView.svelte';
  import DocxPanels from './DocxPanels.svelte';
  import { t } from '@shared/lib/i18n';
  import ImageHighlightDiffViewer from './ImageHighlightDiffViewer.svelte';
  import { renderKindOf } from '../lib/render-to-image';
  import { diffSides } from '../lib/diff-sides';
  import { needsRenderedPixelDiff } from '../lib/render-kind';
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
    ChevronUp,
    ChevronDown,
    Search,
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
    /** Compara `filePath` con este otro archivo de la bóveda en lugar de con git. */
    compareWith?: string;
    /** Parche de un commit: el archivo en ese commit contra su commit padre. */
    commitRevision?: string;
    /** Con `commitRevision`: la otra versión a comparar (en vez del commit padre). */
    fromRevision?: string;
    /** Sincronizado con el botón Lectura/Edición de la cabecera (solo aplica a diffs del árbol de trabajo). */
    readOnly?: boolean;
    /** Modo Vim en el visor de diff de CodeMirror. */
    vimMode?: boolean;
    onClose?: () => void;
    /** Abre el diff de stage del archivo (cuando todos sus cambios ya están en stage). */
    onOpenStaged?: () => void;
  }

  let { filePath, isStaged = false, baseRevision, compareWith, commitRevision, fromRevision, readOnly = true, vimMode, onClose, onOpenStaged }: Props = $props();

  // Qué es cada lado, igual en todas las vistas: izquierda / «−» = antes, derecha / «+» = después
  const sides = $derived(diffSides({ filePath, isStaged, baseRevision, compareWith, commitRevision, fromRevision }));

  let loading = $state(true);
  let error = $state<string | null>(null);
  let diffData = $state<GitDiffResponse | null>(null);
  let collapseUnchanged = $state(true);
  let cmDiffRef = $state<CodeMirrorDiffView | null>(null);
  let chunkInfo = $state<DiffChunksInfo>({ total: 0, current: 0 });
  let isSavingLocal = $state(false);
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;
  let latestEditedContent: string | null = null;

  const isEditable = $derived(!readOnly && !isStaged && !baseRevision && !commitRevision && !compareWith);
  const effectiveVimMode = $derived(vimMode ?? appSettings.vimMode);

  $effect(() => {
    if (!isEditable) {
      void flushPendingSave();
    }
  });

  const isDrawing = $derived(isDrawingFile(filePath));
  const isDiagram = $derived(isDiagramFile(filePath));
  const isPdf = $derived(isPdfFile(filePath));
  const isSvg = $derived(isSvgFile(filePath));
  const isDocx = $derived(isDocxFile(filePath));
  const isImage = $derived(isImageFile(filePath) && !isDrawing);
  // Al comparar dos archivos, el modo visual solo tiene sentido si los dos son del mismo tipo
  const sameKind = $derived(
    !compareWith ||
      (isImageFile(compareWith) && !isDrawingFile(compareWith)) === isImage &&
        isPdfFile(compareWith) === isPdf &&
        isDocxFile(compareWith) === isDocx &&
        isDrawingFile(compareWith) === isDrawing &&
        isDiagramFile(compareWith) === isDiagram &&
        isD2File(compareWith) === isD2File(filePath) &&
        isPlantUmlFile(compareWith) === isPlantUmlFile(filePath) &&
        isSvgFile(compareWith) === isSvg,
  );
  // Tipos distintos que se pueden dibujar (imagen contra Excalidraw, Mermaid contra D2…) o draw.io:
  // cada lado se convierte en imagen y se comparan con los visores de imágenes
  const renderToImage = $derived(
    needsRenderedPixelDiff(renderKindOf(compareWith ?? filePath), renderKindOf(filePath), sameKind),
  );
  // Al comparar con otro tipo, si el otro es una imagen raster el diff de texto tampoco sirve
  const compareIsRaster = $derived(
    renderToImage && !!compareWith && isImageFile(compareWith) && !isSvgFile(compareWith) && !isDrawingFile(compareWith),
  );
  const isVisualCandidate = $derived(renderToImage || (sameKind && (isImage || isPdf || isDrawing || isDiagram || isDocx)));
  const isPureBinary = $derived((isImage && !isSvg) || isPdf || compareIsRaster || (isDocx && sameKind));
  // Diff de píxeles: imágenes raster (en el backend) y dibujos de Excalidraw (exportados a imagen)
  // y diagramas (renderizados a SVG y rasterizados en memoria)
  const hasPixelDiff = $derived(renderToImage || (isImage && !isSvg) || isDrawing || isDiagram);

  $effect(() => {
    workspaceDiffSettings.initFromWorkspace();
  });

  // Modo propio de esta pestaña (una pestaña duplicada puede ver otro modo); el ajuste del espacio de
  // trabajo solo da el modo inicial y recuerda el último elegido para los diffs nuevos.
  let ownVisualTab = $state<DiffVisualTab | null>(null);
  let ownTextMode = $state<typeof workspaceDiffSettings.textMode | null>(null);
  const chosenVisualTab = $derived(ownVisualTab ?? workspaceDiffSettings.visualTab);
  const chosenTextMode = $derived(ownTextMode ?? workspaceDiffSettings.textMode);
  // Al cargar las preferencias, la pestaña fija su modo: cambiarlo en otra ya no la arrastra
  $effect(() => {
    if (!workspaceDiffSettings.isLoaded) return;
    untrack(() => {
      ownVisualTab ??= workspaceDiffSettings.visualTab;
      ownTextMode ??= workspaceDiffSettings.textMode;
    });
  });
  // Los documentos de Word abren en «Cambios»; lo elegido en otros tipos de archivo no cuenta
  let docxTab = $state<DiffVisualTab>('highlight');
  let reloadKey = $state(0);
  function chooseVisualTab(tab: DiffVisualTab) {
    if (isDocx) {
      docxTab = tab;
      return;
    }
    ownVisualTab = tab;
    workspaceDiffSettings.setVisualTab(tab);
  }
  function syncEditedIntoDiffData() {
    if (latestEditedContent !== null && diffData && diffData.new_content !== latestEditedContent) {
      diffData = { ...diffData, new_content: latestEditedContent };
    }
  }

  async function flushPendingSave() {
    if (!saveTimeout || latestEditedContent === null) return;
    clearTimeout(saveTimeout);
    saveTimeout = null;
    const contentToSave = latestEditedContent;
    try {
      isSavingLocal = true;
      await vaultRepository.saveNote({
        relativePath: filePath,
        title: fileName,
        content: contentToSave,
        encoding: 'UTF-8',
      });
    } catch (err) {
      console.error('Error al guardar cambios locales desde diff:', err);
    } finally {
      isSavingLocal = false;
    }
  }

  function handleEditorContentChange(newContent: string) {
    if (!isEditable) return;
    latestEditedContent = newContent;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      void flushPendingSave();
    }, 400);
  }

  onDestroy(() => {
    void flushPendingSave();
  });

  function chooseTextMode(mode: typeof workspaceDiffSettings.textMode) {
    if (mode === 'raw' && latestEditedContent !== null) {
      void flushPendingSave().then(() => fetchDiff());
    } else {
      syncEditedIntoDiffData();
    }
    ownTextMode = mode;
    workspaceDiffSettings.setTextMode(mode);
  }

  const activeVisualTab = $derived.by<DiffVisualTab>(() => {
    if (!isVisualCandidate) return 'text';
    if (isDocx) return docxTab === 'side_by_side' ? 'side_by_side' : 'highlight';
    if (chosenVisualTab === 'highlight' && !hasPixelDiff) {
      return 'side_by_side';
    }
    if (isPureBinary && chosenVisualTab === 'text') {
      return 'side_by_side';
    }
    return chosenVisualTab;
  });

  // Vistas ya mostradas: se mantienen montadas para no recalcularlas al volver a ellas
  let visited = $state<Record<DiffVisualTab, boolean>>({ text: false, side_by_side: false, highlight: false });
  $effect(() => {
    const tab = activeVisualTab;
    if (!visited[tab]) visited[tab] = true;
  });

  const viewMode = $derived<'split' | 'unified'>(
    chosenTextMode === 'unified' ? 'unified' : 'split'
  );
  const showRawDiff = $derived(chosenTextMode === 'raw');
  let enableWrap = $state(false);

  let currentTheme = $derived<'light' | 'dark'>(
    appSettings.resolvedTheme === 'dark' ? 'dark' : 'light'
  );

  async function fetchDiff() {
    loading = true;
    error = null;
    latestEditedContent = null;
    chunkInfo = { total: 0, current: 0 };
    try {
      // De un documento de Word solo interesa si todo está ya en stage; lo demás lo calcula su vista
      if (isDocx && sameKind && (compareWith || commitRevision)) {
        diffData = null;
        return;
      }
      const data = compareWith
        ? await vaultRepository.diffVaultFiles(compareWith, filePath)
        : commitRevision
          ? await vaultRepository.diffGitRevisions(filePath, fromRevision ?? `${commitRevision}^`, commitRevision)
          : await vaultRepository.getGitFileDiff(filePath, isStaged, baseRevision);
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
    const _f = fromRevision;
    const _k = commitRevision;
    const _c = compareWith;
    fetchDiff();
  });

  const fileName = $derived(filePath.split('/').pop() || filePath);

  $effect(() => {
    if (!diffData) return;
    const maxExpandLines = DEFAULT_APP_CONFIG.diff?.max_expand_lines ?? 0;
    if (maxExpandLines > 0) {
      const totalLines = Math.max(
        diffData.old_content ? diffData.old_content.split('\n').length : 0,
        diffData.new_content ? diffData.new_content.split('\n').length : 0
      );
      collapseUnchanged = totalLines > maxExpandLines;
    } else {
      collapseUnchanged = true;
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
    syncEditedIntoDiffData();
    collapseUnchanged = false;
  }

  function handleCollapseAll() {
    syncEditedIntoDiffData();
    collapseUnchanged = true;
  }

  function handleToggleWrap() {
    syncEditedIntoDiffData();
    enableWrap = !enableWrap;
  }
</script>

<div class="diff-viewer-wrapper">
  <!-- BARRA DE HERRAMIENTAS SUPERIOR -->
  <header class="diff-toolbar">
    <div class="diff-toolbar-left">
      <div
        class="diff-file-badge"
        class:is-staged={isStaged && !baseRevision}
        class:is-revision={Boolean(baseRevision || commitRevision)}
        title={compareWith ? t('diffViewer.badge.compareTwoFiles') : commitRevision && fromRevision ? t('diffViewer.badge.compareRevisions', { from: fromRevision, to: commitRevision }) : commitRevision ? t('diffViewer.badge.commitChanges', { commit: commitRevision }) : baseRevision ? t('diffViewer.badge.compareWithRev', { rev: baseRevision }) : (isStaged ? t('diffViewer.badge.stagedChanges') : t('diffViewer.badge.workingTreeChanges'))}
      >
        {#if compareWith}
          <GitCompare size={14} class="badge-icon" />
          <span>{t('diffViewer.badge.files')}</span>
        {:else if commitRevision && fromRevision}
          <GitCompare size={14} class="badge-icon" />
          <span>{t('diffViewer.badge.versions', { from: fromRevision.slice(0, 7), to: commitRevision.slice(0, 7) })}</span>
        {:else if commitRevision}
          <History size={14} class="badge-icon" />
          <span>COMMIT: {commitRevision.slice(0, 7)}</span>
        {:else if baseRevision}
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
        <span class="diff-file-path">{compareWith ? `${compareWith} → ${filePath}` : filePath}</span>
      </div>

      {#if hasDiff && !isDocx}
        <div class="diff-counts">
          {#if diffStats.added > 0}
            <span class="stat-added">+{diffStats.added}</span>
          {/if}
          {#if diffStats.deleted > 0}
            <span class="stat-deleted">-{diffStats.deleted}</span>
          {/if}
          {#if isSavingLocal}
            <span class="diff-saving-indicator">{t('diffViewer.saving')}</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="diff-toolbar-right">
      {#if isVisualCandidate}
        <div class="view-mode-toggle" role="group" aria-label={t('diffViewer.modeGroupAria')}>
          <!-- 1. LADO A LADO -->
          <button
            type="button"
            class="toggle-btn"
            class:active={activeVisualTab === 'side_by_side'}
            onclick={() => chooseVisualTab('side_by_side')}
            title={isDocx ? t('docxDiff.panelsHint') : compareWith ? t('diffViewer.twoPanesTitle') : t('diffViewer.twoPanesGitTitle')}
          >
            <Columns size={13} />
            <span>{t('diffViewer.twoPanes')}</span>
          </button>

          <!-- Word: cambios de texto, tablas e imágenes -->
          {#if isDocx}
            <button
              type="button"
              class="toggle-btn"
              class:active={activeVisualTab === 'highlight'}
              onclick={() => chooseVisualTab('highlight')}
              title={t('docxDiff.tabHint')}
            >
              <GitCompare size={13} />
              <span>{t('docxDiff.tab')}</span>
            </button>
          {/if}

          <!-- 2. RESALTADO PERCEPTUAL (imágenes raster y dibujos de Excalidraw) -->
          {#if hasPixelDiff}
            <button
              type="button"
              class="toggle-btn"
              class:active={activeVisualTab === 'highlight'}
              onclick={() => chooseVisualTab('highlight')}
              title={t('diffViewer.differencesTitle')}
            >
              <Sparkles size={13} />
              <span>{t('diffViewer.differences')}</span>
            </button>
          {/if}

          <!-- 3. MODO TEXTO (Solo si NO es un binario puro como imagen o PDF) -->
          {#if !isPureBinary}
            <button
              type="button"
              class="toggle-btn"
              class:active={activeVisualTab === 'text'}
              onclick={() => chooseVisualTab('text')}
              title={t('diffViewer.textViewTitle')}
            >
              <FileCode size={13} />
              <span>{t('diffViewer.textView')}</span>
            </button>
          {/if}
        </div>
      {/if}

      <!-- SUBOPCIONES DE TEXTO (Solo si no es binario y está en pestaña texto) -->
      {#if !isPureBinary && (!isVisualCandidate || activeVisualTab === 'text')}
        <!-- MODO DE VISTA: SPLIT, UNIFIED O TEXTO PLANO -->
        <div class="view-mode-toggle" role="group" aria-label={t('diffViewer.textModeGroupAria')}>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showRawDiff && viewMode === 'split'}
            onclick={() => chooseTextMode('split')}
            title={t('diffViewer.splitTitle')}
          >
            <Columns size={13} />
            <span>{t('diffViewer.split')}</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showRawDiff && viewMode === 'unified'}
            onclick={() => chooseTextMode('unified')}
            title={t('diffViewer.unifiedTitle')}
          >
            <AlignJustify size={13} />
            <span>{t('diffViewer.unified')}</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showRawDiff}
            onclick={() => chooseTextMode('raw')}
            title={t('diffViewer.patchTitle')}
          >
            <FileCode size={13} />
            <span>{t('diffViewer.patch')}</span>
          </button>
        </div>

        {#if !showRawDiff && hasDiff}
          <!-- NAVEGACIÓN ENTRE CAMBIOS -->
          {#if chunkInfo.total > 0}
            <div class="chunk-nav" role="group" aria-label={t('diffViewer.chunkNavAria')}>
              <button
                type="button"
                class="chunk-nav-btn"
                onclick={() => cmDiffRef?.prevChunk()}
                title={t('diffViewer.prevChange')}
                aria-label={t('diffViewer.prevChange')}
              >
                <ChevronUp size={14} />
              </button>
              <span class="chunk-counter" title={t('diffViewer.changeChunksTitle')}>
                {chunkInfo.current > 0 ? `${chunkInfo.current}/${chunkInfo.total}` : `${chunkInfo.total}`}
              </span>
              <button
                type="button"
                class="chunk-nav-btn"
                onclick={() => cmDiffRef?.nextChunk()}
                title={t('diffViewer.nextChange')}
                aria-label={t('diffViewer.nextChange')}
              >
                <ChevronDown size={14} />
              </button>
            </div>
          {/if}

          <!-- BUSCAR EN EL DIFF -->
          <button
            type="button"
            class="action-btn"
            onclick={() => cmDiffRef?.openSearch()}
            title={t('diffViewer.searchTitle')}
            aria-label={t('diffViewer.searchAria')}
          >
            <Search size={14} />
          </button>

          <!-- EXPANDIR / COLAPSAR TODO -->
          <button
            type="button"
            class="action-btn"
            onclick={handleExpandAll}
            title={t('diffViewer.expandAll')}
          >
            <ChevronsUpDown size={14} />
          </button>
          <button
            type="button"
            class="action-btn"
            onclick={handleCollapseAll}
            title={t('diffViewer.collapseAll')}
          >
            <ChevronsDownUp size={14} />
          </button>
        {/if}

        <!-- AJUSTE DE LÍNEA (WRAP) -->
        <button
          type="button"
          class="action-btn"
          class:active={enableWrap}
          onclick={handleToggleWrap}
          title={enableWrap ? t('diffViewer.disableWrap') : t('diffViewer.enableWrap')}
        >
          <WrapText size={14} />
        </button>
      {/if}

      <!-- RECARGAR DIFF -->
      <button
        type="button"
        class="action-btn"
        disabled={loading}
        onclick={() => {
          reloadKey++;
          void fetchDiff();
        }}
        title={t('diffViewer.reloadTitle')}
      >
        <RotateCw size={14} class={loading ? 'spinning' : ''} />
      </button>
    </div>
  </header>

  {#if hasDiff && activeVisualTab === 'text'}
    <!-- Qué es cada lado del diff de texto (izquierda / «−» = antes, derecha / «+» = después) -->
    <div class="diff-sides-bar" class:unified={viewMode !== 'split' || showRawDiff}>
      {#if viewMode === 'split' && !showRawDiff}
        <span class="diff-side before" title={t('diffViewer.leftBeforeTitle')}>◀ {sides.beforeLabel}</span>
        <span class="diff-side after" title={t('diffViewer.rightAfterTitle')}>{sides.afterLabel} ▶</span>
      {:else}
        <span class="diff-side before" title={t('diffViewer.minusBeforeTitle')}>− {sides.beforeLabel}</span>
        <span class="diff-side after" title={t('diffViewer.plusAfterTitle')}>+ {sides.afterLabel}</span>
      {/if}
    </div>
  {/if}

  {#if diffData?.only_staged && !isStaged && !baseRevision && !compareWith && !commitRevision}
    <!-- Como `git diff`: local contra stage. Todo está en stage: los cambios se ven en el diff de stage -->
    <div class="diff-only-staged" role="status">
      <span>{t('diffViewer.onlyStagedPrefix')} <strong>{fileName}</strong> {t('diffViewer.onlyStagedSuffix')}</span>
      {#if onOpenStaged}
        <button type="button" class="diff-only-staged-btn" onclick={onOpenStaged}>{t('diffViewer.viewDiffStaged')}</button>
      {/if}
    </div>
  {/if}

  <!-- CONTENIDO PRINCIPAL -->
  <main class="diff-body">
    <!-- Cada vista se calcula la primera vez que se elige y queda en memoria (oculta) hasta cerrar la pestaña -->
    {#if isVisualCandidate && visited.side_by_side}
      <div class="diff-mode-pane" class:hidden={activeVisualTab !== 'side_by_side'}>
        {#if isDocx}
          <DocxPanels {sides} {reloadKey} />
        {:else}
          <VisualDiffViewer {filePath} {isStaged} {baseRevision} {compareWith} {commitRevision} {fromRevision} {renderToImage} />
        {/if}
      </div>
    {/if}
    {#if isDocx && isVisualCandidate && visited.highlight}
      <div class="diff-mode-pane" class:hidden={activeVisualTab !== 'highlight'}>
        <DocxDiffView {sides} {reloadKey} />
      </div>
    {/if}
    {#if hasPixelDiff && visited.highlight}
      <div class="diff-mode-pane" class:hidden={activeVisualTab !== 'highlight'}>
        <ImageHighlightDiffViewer {filePath} {isStaged} {baseRevision} {isDrawing} {isDiagram} {compareWith} {commitRevision} {fromRevision} {renderToImage} />
      </div>
    {/if}
    {#if visited.text}
    <div class="diff-mode-pane" class:hidden={activeVisualTab !== 'text'}>
    {#if loading}
      <div class="diff-state-container">
        <div class="spinner"></div>
        <p class="diff-state-text">{t('diffViewer.loadingDiff')}</p>
      </div>
    {:else if error}
      <div class="diff-state-container error">
        <div class="state-icon error-icon">
          <AlertCircle size={32} />
        </div>
        <h3 class="state-title">{t('diffViewer.errorTitle')}</h3>
        <p class="state-desc">{error}</p>
        <button type="button" class="retry-btn" onclick={fetchDiff}>
          {t('gitFileLog.retry')}
        </button>
      </div>
    {:else if !hasDiff}
      <div class="diff-state-container empty">
        <div class="state-icon success-icon">
          <CheckCircle2 size={32} />
        </div>
        <h3 class="state-title">{t('diffViewer.noDiffTitle')}</h3>
        <p class="state-desc">
          {t('diffViewer.noDiffPrefix')} <strong>{fileName}</strong> {compareWith ? t('diffViewer.noDiffCompareWith', { path: compareWith }) : commitRevision ? t('diffViewer.noDiffCommit', { rev: commitRevision.slice(0, 7) }) : baseRevision ? t('diffViewer.noDiffRev', { rev: baseRevision.slice(0, 7) }) : (isStaged ? t('diffViewer.noDiffStaged') : t('diffViewer.noDiffWorktree'))}.
        </p>
      </div>
    {:else if diffData}
      {#if showRawDiff}
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
          <CodeMirrorDiffView
            bind:this={cmDiffRef}
            oldContent={diffData.old_content ?? ''}
            newContent={diffData.new_content ?? ''}
            {fileName}
            mode={viewMode}
            wrap={enableWrap}
            {collapseUnchanged}
            theme={currentTheme}
            editable={isEditable}
            vimMode={effectiveVimMode}
            onContentChange={handleEditorContentChange}
            onChunksChange={(info) => (chunkInfo = info)}
          />
        </div>
      {/if}
    {/if}
    </div>
    {/if}
  </main>
</div>

<style>
  .diff-only-staged {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
    font-size: 12px;
    background: var(--git-staged-bg);
    color: var(--git-staged);
    border-bottom: 1px solid var(--git-staged-border);
  }
  .diff-only-staged-btn {
    padding: 2px 8px;
    border: 1px solid var(--git-staged-border);
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .diff-mode-pane {
    width: 100%;
    height: 100%;
    position: relative;
  }
  /* Oculto sin salir del layout: con display:none WebKit descarta las imágenes decodificadas y los
     lienzos quedan en 0×0, y al volver la vista se veía cargar otra vez */
  .diff-mode-pane.hidden {
    position: absolute;
    inset: 0;
    visibility: hidden;
    pointer-events: none;
  }

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
    background-color: var(--accent-bg);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .diff-file-badge.is-staged {
    background-color: var(--git-staged-bg);
    color: var(--git-staged);
    border-color: var(--git-staged-border);
  }

  .diff-file-badge.is-revision {
    background-color: var(--purple-bg);
    color: var(--purple);
    border-color: var(--purple-border);
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
    color: var(--success);
  }

  .stat-deleted {
    color: var(--danger);
  }

  .diff-saving-indicator {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    margin-left: 4px;
  }

  .chunk-nav {
    display: inline-flex;
    align-items: center;
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: 5px;
    height: 28px;
    padding: 0 2px;
    gap: 2px;
  }

  .chunk-nav-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0;
  }

  .chunk-nav-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-primary);
  }

  .chunk-counter {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 0 4px;
    font-variant-numeric: tabular-nums;
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
    box-shadow: var(--card-shadow);
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
    color: var(--danger);
  }

  .success-icon {
    color: var(--success);
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
    color: var(--on-accent);
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
    background-color: var(--success-bg);
    color: var(--success);
  }

  .diff-raw-line.del {
    background-color: var(--danger-bg);
    color: var(--danger);
  }

  .diff-raw-line.hunk {
    background-color: var(--accent-bg);
    color: var(--accent);
    font-weight: 600;
  }

  .diff-raw-line.header {
    color: var(--text-muted);
    font-weight: 500;
  }
  .diff-sides-bar {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 3px 10px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    font-size: 11px;
    font-weight: 600;
  }
  .diff-sides-bar.unified {
    display: flex;
    gap: 16px;
  }
  .diff-side.before {
    color: var(--danger, #f85149);
  }
  .diff-side.after {
    color: var(--success, #3fb950);
  }
  .diff-sides-bar:not(.unified) .diff-side.after {
    text-align: right;
  }
</style>
