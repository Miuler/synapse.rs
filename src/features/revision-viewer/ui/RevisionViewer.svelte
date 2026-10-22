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
    isMarkdownFile,
  } from '@entities/file-type';
  import { ImageViewer } from '@features/image-viewer';
  import { PdfViewer } from '@features/pdf-viewer';
  import { ExcalidrawViewer } from '@features/excalidraw-editor';
  import { MermaidViewer } from '@features/mermaid-editor';
  import { MermanViewer } from '@features/merman-editor';
  import { MarkdownViewer } from '@features/markdown-editor';
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
  let wrapText = $state(false);
  let copiedRaw = $state(false);

  const fileName = $derived(filePath.split('/').pop() || filePath);
  const isDrawing = $derived(isDrawingFile(filePath));
  const isDiagram = $derived(isDiagramFile(filePath));
  const isPdf = $derived(isPdfFile(filePath));
  const isImage = $derived(isImageFile(filePath) && !isDrawing);
  const isMarkdown = $derived(isMarkdownFile(filePath));
  const isTextFile = $derived(!isImage && !isPdf);
  const isCurrentlyInRaw = $derived(
    (isDrawing && showDrawingRaw) ||
    (isMarkdown && showMarkdownRaw) ||
    (!isDrawing && !isDiagram && !isImage && !isPdf && !isMarkdown)
  );

  async function loadRevision() {
    loading = true;
    error = null;
    try {
      const data = await vaultRepository.getGitFileRevision(filePath, false, revision);
      if (!data.exists) {
        error = `El archivo no existía en la revisión ${revision.slice(0, 7)}.`;
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
      <div class="revision-badge rev" title="Revisión Git: {revision}">
        <History size={13} class="badge-icon" />
        <span>REV: {revision.slice(0, 7)}</span>
      </div>

      <div class="revision-badge readonly" title="Cargado directamente en memoria. Sin modificaciones en disco.">
        <Shield size={12} class="badge-icon" />
        <span>SOLO LECTURA</span>
      </div>

      <div class="revision-file-info" title={filePath}>
        <span class="revision-file-name">{fileName}</span>
        <span class="revision-file-path">{filePath}</span>
      </div>
    </div>

    <div class="revision-toolbar-right">
      <!-- 1. TOGGLES DE MODO DE VISTA -->
      {#if isDrawing}
        <div class="view-mode-toggle" role="group" aria-label="Modo de visualización Excalidraw">
          <button
            type="button"
            class="toggle-btn"
            class:active={!showDrawingRaw}
            onclick={() => (showDrawingRaw = false)}
            title="Vista visual interactiva (Excalidraw)"
          >
            <Eye size={12} />
            <span>Visual</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showDrawingRaw}
            onclick={() => (showDrawingRaw = true)}
            title="Ver contenido Raw como texto JSON"
          >
            <FileCode size={12} />
            <span>Raw (Texto)</span>
          </button>
        </div>
      {:else if isDiagram}
        <div class="view-mode-toggle" role="group" aria-label="Modo de visualización Mermaid">
          <button
            type="button"
            class="toggle-btn"
            class:active={!showMermaidEditor}
            onclick={() => (showMermaidEditor = false)}
            title="Ver únicamente el diagrama renderizado"
          >
            <Eye size={12} />
            <span>Solo Diagrama</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showMermaidEditor}
            onclick={() => (showMermaidEditor = true)}
            title="Modo edición: ver el código Mermaid al lado del diagrama en solo lectura"
          >
            <Columns size={12} />
            <span>Modo Edición (Código)</span>
          </button>
        </div>
      {:else if isMarkdown}
        <div class="view-mode-toggle" role="group" aria-label="Modo de visualización Markdown">
          <button
            type="button"
            class="toggle-btn"
            class:active={!showMarkdownRaw}
            onclick={() => (showMarkdownRaw = false)}
            title="Vista previa formateada (Markdown)"
          >
            <Eye size={12} />
            <span>Lectura</span>
          </button>
          <button
            type="button"
            class="toggle-btn"
            class:active={showMarkdownRaw}
            onclick={() => (showMarkdownRaw = true)}
            title="Ver texto Markdown en bruto (Raw)"
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
          title={wrapText ? 'Desactivar ajuste de línea' : 'Activar ajuste de línea (Wrap)'}
        >
          <WrapText size={14} />
        </button>

        <button
          type="button"
          class="rev-btn icon-btn"
          onclick={copyRawContent}
          title="Copiar texto al portapapeles"
        >
          {#if copiedRaw}
            <Check size={14} style="color: #1a7f37;" />
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
        title="Recargar revisión histórica"
      >
        <RotateCw size={14} class={loading ? 'spinning' : ''} />
      </button>

      <!-- 4. BOTÓN CERRAR -->
      {#if onClose}
        <button
          type="button"
          class="rev-btn icon-btn close-btn"
          onclick={onClose}
          title="Cerrar pestaña"
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
        <p class="revision-state-text">Cargando revisión histórica ({revision.slice(0, 7)}) desde Git en memoria...</p>
      </div>
    {:else if error}
      <div class="revision-state-container error">
        <div class="state-icon error-icon">
          <AlertCircle size={32} />
        </div>
        <h3 class="state-title">Error al cargar revisión</h3>
        <p class="state-desc">{error}</p>
        <button type="button" class="retry-btn" onclick={loadRevision}>
          Reintentar
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
              readOnly={true}
            />
          {/if}
        {:else if isDiagram}
          {#if appSettings.mermaidRenderer === 'mermaidjs'}
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
    background: rgba(9, 105, 218, 0.12);
    color: var(--accent, #0969da);
    border: 1px solid rgba(9, 105, 218, 0.25);
  }

  .revision-badge.readonly {
    background: rgba(101, 109, 118, 0.1);
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
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
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
    color: #cf222e;
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
    color: #cf222e;
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
