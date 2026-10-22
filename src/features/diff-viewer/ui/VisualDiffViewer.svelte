<script lang="ts">
  import { onMount } from 'svelte';
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
  } from '@entities/file-type';
  import { ImageViewer } from '@features/image-viewer';
  import { PdfViewer } from '@features/pdf-viewer';
  import { ExcalidrawViewer } from '@features/excalidraw-editor';
  import { MermaidViewer } from '@features/mermaid-editor';
  import { MermanViewer } from '@features/merman-editor';
  import { AlertCircle, HardDrive, GitCommit, Split } from 'lucide-svelte';

  interface Props {
    filePath: string;
    isStaged?: boolean;
    baseRevision?: string;
  }

  let { filePath, isStaged = false, baseRevision }: Props = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let gitRevision = $state<GitFileRevisionResult | null>(null);
  let localContent = $state<string>('');
  let localDataUrl = $state<string | null>(null);

  let splitRatio = $state(50);
  let isResizing = $state(false);
  let containerRef = $state<HTMLDivElement | null>(null);

  let saveTimeout: ReturnType<typeof setTimeout> | null = null;
  let isSavingLocal = $state(false);
  const fileName = $derived(filePath.split('/').pop() || filePath);

  function persistLocalChanges(newContent: string) {
    localContent = newContent;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      try {
        isSavingLocal = true;
        await vaultRepository.saveNote({
          relativePath: filePath,
          title: fileName,
          content: newContent,
          encoding: 'UTF-8',
        });
      } catch (err) {
        console.error('Error al guardar cambios locales desde visual diff:', err);
      } finally {
        isSavingLocal = false;
      }
    }, 600);
  }
  const isDrawing = $derived(isDrawingFile(filePath));
  const isDiagram = $derived(isDiagramFile(filePath));
  const isPdf = $derived(isPdfFile(filePath));
  const isImage = $derived(isImageFile(filePath) && !isDrawing);

  async function loadData() {
    loading = true;
    error = null;
    try {
      // 1. Obtener revisión de Git
      const rev = await vaultRepository.getGitFileRevision(filePath, isStaged, baseRevision);
      gitRevision = rev;

      // 2. Si es dibujo o diagrama, leer contenido de texto de disco
      if (isDrawing || isDiagram) {
        const noteData = await vaultRepository.readNote(filePath);
        localContent = noteData?.content || '';
      } else if (isImage || isPdf) {
        // Para imágenes o PDF, obtener dataUrl de disco
        const url = await vaultRepository.readAssetDataUrl(filePath);
        localDataUrl = url;
      }
    } catch (err: unknown) {
      console.error('Error cargando diff visual:', err);
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const _p = filePath;
    const _s = isStaged;
    const _r = baseRevision;
    loadData();
  });

  function handleResizeStart(e: MouseEvent | PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    isResizing = true;
    document.body.classList.add('is-resizing-col');

    const handlePointerMove = (ev: MouseEvent | PointerEvent) => {
      if (!containerRef) return;
      const rect = containerRef.getBoundingClientRect();
      const relativeX = ev.clientX - rect.left;
      const percentage = (relativeX / rect.width) * 100;
      splitRatio = Math.max(15, Math.min(85, percentage));
    };

    const handlePointerUp = () => {
      isResizing = false;
      document.body.classList.remove('is-resizing-col');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
  }
</script>

<div class="visual-diff-root" bind:this={containerRef}>
  {#if loading}
    <div class="state-container">
      <div class="spinner"></div>
      <p>Cargando visualización de ambos paneles (Disco vs Git)...</p>
    </div>
  {:else if error}
    <div class="state-container error">
      <AlertCircle size={32} class="error-icon" />
      <h3>Error al preparar diff visual</h3>
      <p>{error}</p>
    </div>
  {:else}
    <!-- PANEL IZQUIERDO: DISCO DURO (WORKING TREE) -->
    <div class="pane left-pane" style="width: {splitRatio}%">
      <div class="pane-header">
        <div class="pane-tag local">
          <HardDrive size={13} />
          <span>Disco Duro (Local{#if isDrawing || isDiagram} — Editable{/if})</span>
        </div>
        <div class="pane-header-right">
          {#if isSavingLocal}
            <span class="saving-indicator">Guardando...</span>
          {/if}
          <span class="pane-filename" title={filePath}>{fileName}</span>
        </div>
      </div>

      <div class="pane-content">
        {#if isDrawing}
          <ExcalidrawViewer
            content={localContent}
            readOnly={false}
            onChange={(updated) => persistLocalChanges(updated)}
            onSave={(updated) => updated && persistLocalChanges(updated)}
          />
        {:else if isDiagram}
          {#if appSettings.mermaidRenderer === 'mermaidjs'}
            <MermaidViewer
              content={localContent}
              readOnly={false}
              onChange={(updated) => persistLocalChanges(updated)}
            />
          {:else}
            <MermanViewer
              content={localContent}
              readOnly={false}
              onChange={(updated) => persistLocalChanges(updated)}
            />
          {/if}
        {:else if isPdf}
          <PdfViewer src={localDataUrl || filePath} />
        {:else if isImage}
          <ImageViewer src={localDataUrl || filePath} alt="Local: {fileName}" />
        {/if}
      </div>
    </div>

    <!-- SEPARADOR / RESIZER -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="resizer"
      class:resizing={isResizing}
      onpointerdown={handleResizeStart}
      role="separator"
      aria-orientation="vertical"
      aria-valuenow={splitRatio}
      tabindex="0"
      title="Arrastra para ajustar proporción"
    >
      <div class="resizer-handle">
        <Split size={12} />
      </div>
    </div>

    <!-- PANEL DERECHO: GIT (HEAD, STAGED O REVISIÓN) -->
    <div class="pane right-pane" style="width: {100 - splitRatio}%">
      <div class="pane-header">
        <div class="pane-tag git" class:is-staged={isStaged && !baseRevision} class:is-revision={Boolean(baseRevision)}>
          <GitCommit size={13} />
          <span>{baseRevision ? `REV: ${baseRevision.slice(0, 7)}` : (isStaged ? 'Stage / Index' : 'HEAD (Último commit)')}</span>
        </div>
        {#if !gitRevision?.exists}
          <span class="not-in-git-badge">No existe en esta revisión</span>
        {:else}
          <span class="pane-rev-info">{baseRevision ? `Revisión ${baseRevision.slice(0, 7)}` : (isStaged ? 'Revisión en Stage' : 'Última versión commiteada')}</span>
        {/if}
      </div>

      <div class="pane-content">
        {#if !gitRevision?.exists}
          <div class="empty-git-state">
            <p>Este archivo no existe en {baseRevision ? `la revisión ${baseRevision.slice(0, 7)}` : (isStaged ? 'el Stage' : 'el último commit (HEAD)')}.</p>
          </div>
        {:else if isDrawing}
          <ExcalidrawViewer content={gitRevision.content || ''} readOnly={true} />
        {:else if isDiagram}
          {#if appSettings.mermaidRenderer === 'mermaidjs'}
            <MermaidViewer content={gitRevision.content || ''} readOnly={true} />
          {:else}
            <MermanViewer content={gitRevision.content || ''} readOnly={true} />
          {/if}
        {:else if isPdf}
          <PdfViewer src={gitRevision.data_url || ''} />
        {:else if isImage}
          <ImageViewer src={gitRevision.data_url || ''} alt="Git: {fileName}" />
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .visual-diff-root {
    display: flex;
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
    background-color: var(--bg-primary, #0f1115);
  }

  .state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    gap: 12px;
    color: var(--text-muted, #8b949e);
  }

  .state-container.error {
    color: #f85149;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-primary, rgba(255, 255, 255, 0.1));
    border-top-color: var(--accent, #38bdf8);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    position: relative;
  }

  .left-pane {
    border-right: 1px solid var(--border-primary, rgba(255, 255, 255, 0.08));
  }

  .right-pane {
    border-left: 1px solid var(--border-primary, rgba(255, 255, 255, 0.08));
  }

  .pane-header {
    height: 34px;
    min-height: 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    background-color: var(--bg-secondary, #161b22);
    border-bottom: 1px solid var(--border-primary, rgba(255, 255, 255, 0.08));
    font-size: 11px;
    user-select: none;
  }

  .pane-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 7px;
    border-radius: 4px;
  }

  .pane-tag.local {
    background-color: rgba(56, 189, 248, 0.12);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.25);
  }

  .pane-tag.git {
    background-color: rgba(245, 158, 11, 0.12);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.25);
  }

  .pane-tag.git.is-staged {
    background-color: rgba(46, 160, 67, 0.15);
    color: #3fb950;
    border: 1px solid rgba(46, 160, 67, 0.3);
  }

  .pane-tag.git.is-revision {
    background-color: rgba(147, 51, 234, 0.15);
    color: #a855f7;
    border: 1px solid rgba(147, 51, 234, 0.3);
  }

  .pane-header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 60%;
  }

  .saving-indicator {
    color: var(--accent, #38bdf8);
    font-size: 10px;
    font-weight: 500;
  }

  .pane-filename,
  .pane-rev-info {
    color: var(--text-secondary, #8b949e);
    font-size: 11px;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .not-in-git-badge {
    background-color: rgba(248, 81, 73, 0.15);
    color: #f85149;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 500;
  }

  .pane-content {
    flex: 1;
    height: calc(100% - 34px);
    overflow: hidden;
    position: relative;
    background-color: var(--bg-primary, #0f1115);
  }

  .empty-git-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 24px;
    text-align: center;
    color: var(--text-muted, #8b949e);
    font-size: 13px;
  }

  .resizer {
    width: 6px;
    margin: 0 -3px;
    cursor: col-resize;
    z-index: 20;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.15s ease;
  }

  .resizer:hover,
  .resizer.resizing {
    background-color: var(--accent, #38bdf8);
  }

  .resizer-handle {
    opacity: 0;
    pointer-events: none;
    background-color: var(--accent, #38bdf8);
    color: #0b0d10;
    border-radius: 3px;
    padding: 4px 1px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.15s;
  }

  .resizer:hover .resizer-handle,
  .resizer.resizing .resizer-handle {
    opacity: 1;
  }
</style>
