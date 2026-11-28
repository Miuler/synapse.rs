<script lang="ts">
  import {
    vaultRepository,
    type GitFileRevisionResult,
  } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
  import {
    isImageFile,
    isPdfFile,
    isDrawingFile,
    isDiagramFile,
    isD2File,
    isPlantUmlFile,
    isMarkdownFile,
  } from '@entities/file-type';
  import { ImageViewer } from '@features/image-viewer';
  import { PdfViewer } from '@features/pdf-viewer';
  import { ExcalidrawViewer } from '@features/excalidraw-editor';
  import { MermaidViewer } from '@features/mermaid-editor';
  import { D2Viewer } from '@features/d2-editor';
  import { PlantUmlViewer } from '@features/plantuml-editor';
  import { MermanViewer } from '@features/merman-editor';
  import { MarkdownViewer } from '@features/markdown-editor';
  import { OpenApiViewer, isOpenApiContent } from '@features/openapi-viewer';
  import { t } from '@shared/lib/i18n';
  import {
    History,
    Shield,
    RotateCw,
    X,
    FileText,
    AlertCircle,
    Eye,
    FileCode,
    Columns,
    Copy,
    Check,
    WrapText,
  } from 'lucide-svelte';

  interface Props {
    filePath: string;
    revision: string;
    onClose?: () => void;
  }

  let { filePath, revision, onClose }: Props = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let revData = $state<GitFileRevisionResult | null>(null);

  // Estados de vista
  let showDrawingRaw = $state(false);
  let showMermaidEditor = $state(true); // Modo edición (código visible) por defecto para Mermaid
  let showMarkdownRaw = $state(false);
  let showOpenApiRaw = $state(false);
  let wrapText = $state(false);
  let copiedRaw = $state(false);

  const fileName = $derived(filePath.split('/').pop() || filePath);
  const isDrawing = $derived(isDrawingFile(filePath));
  const isDiagram = $derived(isDiagramFile(filePath));
  const isPdf = $derived(isPdfFile(filePath));
  const isImage = $derived(isImageFile(filePath) && !isDrawing);
  const isMarkdown = $derived(isMarkdownFile(filePath));
  const isTextFile = $derived(!isImage && !isPdf);
  const isOpenApi = $derived(!isDrawing && !isDiagram && !isMarkdown && isOpenApiContent(filePath, revData?.content));
  const isCurrentlyInRaw = $derived(
    (isDrawing && showDrawingRaw) ||
    (isMarkdown && showMarkdownRaw) ||
    (isOpenApi && showOpenApiRaw) ||
    (!isDrawing && !isDiagram && !isImage && !isPdf && !isMarkdown && !isOpenApi)
  );

  // Los `$ref` a otros archivos se leen de la misma revisión
  async function readAtRevision(path: string): Promise<string | null> {
    const r = await vaultRepository.getGitFileRevision(path, false, revision).catch(() => null);
    return r?.exists ? (r.content ?? null) : null;
  }

  async function loadRevision() {
    loading = true;
    error = null;
    try {
      const data = await vaultRepository.getGitFileRevision(filePath, false, revision);
      if (!data.exists) {
        error = t('revisionViewer.missingInRev', { rev: revision.slice(0, 7) });
      } else {
        revData = data;
      }
    } catch (err: unknown) {
      console.error('Error cargando revisión histórica de archivo:', err);
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const _p = filePath;
    const _r = revision;
    loadRevision();
  });

  async function copyRawContent() {
    if (!revData?.content) return;
    try {
      await navigator.clipboard.writeText(revData.content);
      copiedRaw = true;
      setTimeout(() => {
        copiedRaw = false;
      }, 2000);
    } catch (err) {
      console.error('Error al copiar contenido al portapapeles:', err);
    }
  }
</script>

<div class="revision-viewer-wrapper">
  <!-- BARRA DE HERRAMIENTAS / CABECERA -->
  <header class="revision-toolbar">
    <div class="revision-toolbar-left">
      <div class="revision-badge rev" title={t('revisionViewer.gitRevTitle', { rev: revision })}>
        <History size={13} class="badge-icon" />
        <span>REV: {revision.slice(0, 7)}</span>
      </div>

      <div class="revision-badge readonly" title={t('revisionViewer.readOnlyTitle')}>
        <Shield size={12} class="badge-icon" />
        <span>{t('revisionViewer.readOnlyBadge')}</span>
      </div>

      <div class="revision-file-info" title={filePath}>
        <span class="revision-file-name">{fileName}</span>
        <span class="revision-file-path">{filePath}</span>
      </div>
    </div>

    <div class="revision-toolbar-right">
      <!-- 1. TOGGLES DE MODO DE VISTA -->
      {#if isDrawing}
        <div class="view-mode-toggle" role="group" aria-label={t('revisionViewer.excalidrawModeAria')}>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showDrawingRaw}
            onclick={() => (showDrawingRaw = false)}
            title={t('revisionViewer.visualTitle')}
          >
            <Eye size={12} />
            <span>{t('revisionViewer.visual')}</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showDrawingRaw}
            onclick={() => (showDrawingRaw = true)}
            title={t('revisionViewer.rawTextTitle')}
          >
            <FileCode size={12} />
            <span>{t('revisionViewer.rawText')}</span>
          </button>
        </div>
      {:else if isDiagram}
        <div class="view-mode-toggle" role="group" aria-label={t('revisionViewer.diagramModeAria')}>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showMermaidEditor}
            onclick={() => (showMermaidEditor = false)}
            title={t('revisionViewer.diagramOnlyTitle')}
          >
            <Eye size={12} />
            <span>{t('revisionViewer.diagramOnly')}</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showMermaidEditor}
            onclick={() => (showMermaidEditor = true)}
            title={t('revisionViewer.diagramCodeTitle')}
          >
            <Columns size={12} />
            <span>{t('revisionViewer.diagramCode')}</span>
          </button>
        </div>
      {:else if isMarkdown}
        <div class="view-mode-toggle" role="group" aria-label={t('revisionViewer.markdownModeAria')}>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showMarkdownRaw}
            onclick={() => (showMarkdownRaw = false)}
            title={t('revisionViewer.markdownReadingTitle')}
          >
            <Eye size={12} />
            <span>{t('revisionViewer.markdownReading')}</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showMarkdownRaw}
            onclick={() => (showMarkdownRaw = true)}
            title={t('revisionViewer.markdownRawTitle')}
          >
            <FileCode size={12} />
            <span>Raw</span>
          </button>
        </div>
      {:else if isOpenApi}
        <div class="view-mode-toggle" role="group" aria-label={t('revisionViewer.openapiModeAria')}>
          <button
            type="button"
            class="toggle-btn"
            class:active={!showOpenApiRaw}
            onclick={() => (showOpenApiRaw = false)}
            title={t('revisionViewer.openapiViewTitle')}
          >
            <Eye size={12} />
            <span>API</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showOpenApiRaw}
            onclick={() => (showOpenApiRaw = true)}
            title={t('revisionViewer.openapiRawTitle')}
          >
            <FileCode size={12} />
            <span>Raw</span>
          </button>
        </div>
      {/if}

      <!-- 2. ACCIONES CUANDO ESTÁ EN MODO RAW O TEXTO -->
      {#if isCurrentlyInRaw && revData?.content}
        <button
          type="button"
          class="rev-btn icon-btn"
          class:active={wrapText}
          onclick={() => (wrapText = !wrapText)}
          title={wrapText ? t('revisionViewer.disableWrap') : t('revisionViewer.enableWrap')}
        >
          <WrapText size={14} />
        </button>

        <button
          type="button"
          class="rev-btn icon-btn"
          onclick={copyRawContent}
          title={t('revisionViewer.copyTextTitle')}
        >
          {#if copiedRaw}
            <Check size={14} style="color: var(--success);" />
          {:else}
            <Copy size={14} />
          {/if}
        </button>
      {/if}

      <!-- 3. BOTÓN RECARGAR -->
      <button
        type="button"
        class="rev-btn icon-btn"
        onclick={loadRevision}
        title={t('revisionViewer.reloadTitle')}
      >
        <RotateCw size={14} class={loading ? 'spinning' : ''} />
      </button>

      <!-- 4. BOTÓN CERRAR -->
      {#if onClose}
        <button
          type="button"
          class="rev-btn icon-btn close-btn"
          onclick={onClose}
          title={t('revisionViewer.closeTabTitle')}
        >
          <X size={15} />
        </button>
      {/if}
    </div>
  </header>

  <!-- CONTENIDO PRINCIPAL -->
  <main class="revision-body">
    {#if loading}
      <div class="revision-state-container">
        <div class="spinner"></div>
        <p class="revision-state-text">{t('revisionViewer.loading', { rev: revision.slice(0, 7) })}</p>
      </div>
    {:else if error}
      <div class="revision-state-container error">
        <div class="state-icon error-icon">
          <AlertCircle size={32} />
        </div>
        <h3 class="state-title">{t('revisionViewer.errorTitle')}</h3>
        <p class="state-desc">{error}</p>
        <button type="button" class="retry-btn" onclick={loadRevision}>
          {t('gitFileLog.retry')}
        </button>
      </div>
    {:else if revData}
      <div class="revision-content-wrapper">
        {#if isDrawing}
          {#if showDrawingRaw}
            <div class="revision-code-container">
              <pre class="revision-code-pre" class:wrap={wrapText}><code>{revData.content || ''}</code></pre>
            </div>
          {:else}
            <ExcalidrawViewer
              content={revData.content || ''}
              filePath={filePath}
              readOnly={true}
            />
          {/if}
        {:else if isDiagram}
          {#if isD2File(filePath)}
            <D2Viewer
              content={revData.content || ''}
              readOnly={true}
              showEditor={showMermaidEditor}
            />
          {:else if isPlantUmlFile(filePath)}
            <PlantUmlViewer
              content={revData.content || ''}
              readOnly={true}
              showEditor={showMermaidEditor}
            />
          {:else if appSettings.mermaidRenderer === 'mermaidjs'}
            <MermaidViewer
              content={revData.content || ''}
              readOnly={true}
              showEditor={showMermaidEditor}
            />
          {:else}
            <MermanViewer
              content={revData.content || ''}
              readOnly={true}
              showEditor={showMermaidEditor}
            />
          {/if}
        {:else if isImage}
          <ImageViewer
            src={revData.data_url || ''}
            alt={fileName}
          />
        {:else if isPdf}
          <PdfViewer
            src={revData.data_url || ''}
          />
        {:else if isMarkdown}
          {#if showMarkdownRaw}
            <div class="revision-code-container">
              <pre class="revision-code-pre" class:wrap={wrapText}><code>{revData.content || ''}</code></pre>
            </div>
          {:else}
            <div class="revision-markdown-container">
              <MarkdownViewer
                content={revData.content || ''}
                filePath={filePath}
                readOnly={true}
                viewMode="reading"
                isMarkdown={true}
              />
            </div>
          {/if}
        {:else if isOpenApi && !showOpenApiRaw}
          <div class="revision-markdown-container">
            <OpenApiViewer content={revData.content || ''} path={filePath} readFile={readAtRevision} />
          </div>
        {:else}
          <div class="revision-code-container">
            <pre class="revision-code-pre" class:wrap={wrapText}><code>{revData.content || ''}</code></pre>
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<style>
  .revision-viewer-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    overflow: hidden;
  }

  .revision-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 14px;
    height: 38px;
    background: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-secondary, #e1e4e8);
    user-select: none;
    flex-shrink: 0;
  }

  .revision-toolbar-left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    overflow: hidden;
  }

  .revision-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: monospace;
    letter-spacing: 0.4px;
    flex-shrink: 0;
  }

  .revision-badge.rev {
    background: var(--accent-bg);
    color: var(--accent, #0969da);
    border: 1px solid var(--accent-border);
  }

  .revision-badge.readonly {
    background: var(--badge-bg);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-primary, #d0d7de);
  }

  .revision-file-info {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .revision-file-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .revision-file-path {
    font-size: 11.5px;
    color: var(--text-muted, #8c959f);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .revision-toolbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .view-mode-toggle {
    display: inline-flex;
    align-items: center;
    background-color: var(--bg-tertiary, #eaeef2);
    border: 1px solid var(--border-secondary, #e1e4e8);
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
    color: var(--text-secondary, #656d76);
    background: transparent;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    color: var(--text-primary, #1f2328);
  }

  .toggle-btn.active {
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    box-shadow: var(--card-shadow);
  }

  .rev-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary, #656d76);
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .rev-btn.icon-btn {
    width: 28px;
    height: 28px;
    padding: 0;
  }

  .rev-btn:hover {
    background: var(--bg-tertiary, #eaeef2);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #d0d7de);
  }

  .rev-btn.active {
    background: var(--bg-tertiary, #eaeef2);
    color: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
  }

  .rev-btn.close-btn:hover {
    color: var(--danger);
  }

  .revision-body {
    flex: 1;
    height: calc(100% - 38px);
    overflow: hidden;
    position: relative;
    background: var(--bg-primary, #ffffff);
  }

  .revision-content-wrapper {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .revision-markdown-container {
    width: 100%;
    height: 100%;
    overflow-y: auto;
  }

  .revision-code-container {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 16px;
    background: var(--bg-primary, #ffffff);
    box-sizing: border-box;
  }

  .revision-code-pre {
    margin: 0;
    font-family: var(--code-font, monospace);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-primary, #1f2328);
    white-space: pre;
    tab-size: 2;
  }

  /* El estilo global de `code` (inline: borde, relleno, fondo) dibujaba una caja en cada línea */
  .revision-code-pre code {
    font: inherit;
    color: inherit;
    background: none;
    border: none;
    border-radius: 0;
    padding: 0;
  }

  .revision-code-pre.wrap {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .revision-state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    padding: 32px;
    text-align: center;
  }

  .revision-state-text {
    font-size: 13px;
    color: var(--text-secondary, #656d76);
  }

  .state-icon.error-icon {
    color: var(--danger);
  }

  .state-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary, #1f2328);
  }

  .state-desc {
    font-size: 12.5px;
    color: var(--text-secondary, #656d76);
    max-width: 450px;
    margin: 0;
  }

  .retry-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
    margin-top: 6px;
  }

  .retry-btn:hover {
    background: var(--bg-tertiary, #eaeef2);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-primary, #d0d7de);
    border-top-color: var(--accent, #0969da);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  :global(.spinning) {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
