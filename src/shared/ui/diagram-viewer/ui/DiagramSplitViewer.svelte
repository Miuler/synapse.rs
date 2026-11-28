<script lang="ts">
  import { tick } from 'svelte';
  import { Toolbar, Separator } from 'bits-ui';
  import { Search, Save, AlertCircle } from 'lucide-svelte';
  import { CodeEditor, type EditorDiagnostic } from '@shared/ui/code-editor';
  import { diagramLayoutApi } from '@shared/api';
  import { SplitResizer } from '@shared/ui/split-resizer';
  import { LaserCanvas, LaserToggle, laserShortcuts, LASER_CURSOR } from '@shared/ui/laser-pointer';
  import { t } from '@shared/lib/i18n';

  interface Props {
    content: string;
    engineName: string;
    engineTooltip?: string;
    mode: 'd2' | 'mermaid' | 'plantuml';
    /** Identifica el diagrama para recordar la proporción del panel de código */
    filePath?: string;
    svgContent: string;
    isReady: boolean;
    error?: string;
    diagnostics?: EditorDiagnostic[];
    readOnly?: boolean;
    showEditor?: boolean;
    vimMode?: boolean;
    /** Pestaña activa: al volver a ella, el editor recupera el foco y su cursor. */
    isActive?: boolean;
    /** Ctrl+Enter en el editor: fuerza el redibujado del diagrama. */
    onRun?: () => void;
    loadingMessage?: string;
    onChange?: (content: string) => void;
    onSelectionChange?: (info: any) => void;
  }

  let {
    content = '',
    engineName,
    engineTooltip = '',
    mode,
    filePath,
    svgContent = '',
    isReady = true,
    error = '',
    diagnostics = [],
    readOnly = false,
    showEditor = false,
    vimMode = false,
    isActive = false,
    onRun,
    loadingMessage,
    onChange,
    onSelectionChange,
  }: Props = $props();
  const effectiveLoadingMessage = $derived(loadingMessage ?? t('diagramViewer.loadingEngine'));

  const isEditorVisible = $derived(!readOnly || showEditor);

  // Ancho del panel de código (%). null = valor por defecto de CSS; solo se fija si el usuario lo manipula
  let editorWidthPct = $state<number | null>(null);
  let isResizing = $state(false);
  let viewerContainerRef = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const path = filePath;
    editorWidthPct = null;
    if (!path) return;
    let isCurrent = true;
    diagramLayoutApi.getSplitRatio(path).then((ratio) => {
      if (isCurrent && ratio !== null) editorWidthPct = ratio;
    });
    return () => {
      isCurrent = false;
    };
  });

  let codeEditorRef = $state<any>(null);

  let scale = $state(1);
  /** Desplazamiento del centro de la vista, en unidades del viewBox del diagrama. */
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);
  let startPanX = 0;
  let startPanY = 0;

  // Modo Puntero Láser
  let isLaserMode = $state(false);
  let laser = $state<ReturnType<typeof LaserCanvas> | null>(null);

  let containerRef = $state<HTMLDivElement | null>(null);
  let baseViewBox = $state<{ x: number; y: number; w: number; h: number } | null>(null);

  // Al cambiar el SVG renderizado, calcular su base viewBox
  $effect(() => {
    const svgStr = svgContent;
    if (svgStr) {
      scale = 1;
      panX = 0;
      panY = 0;

      const match = svgStr.match(/viewBox=["']([^"']+)["']/i);
      let parsedVb: { x: number; y: number; w: number; h: number } | null = null;

      if (match && match[1]) {
        const parts = match[1].trim().split(/[\s,]+/).map(Number);
        if (parts.length === 4 && !parts.some(isNaN)) {
          parsedVb = { x: parts[0], y: parts[1], w: parts[2], h: parts[3] };
        }
      }

      if (!parsedVb) {
        const widthMatch = svgStr.match(/width=["']([0-9.]+)(?:px)?["']/i);
        const heightMatch = svgStr.match(/height=["']([0-9.]+)(?:px)?["']/i);
        const w = widthMatch ? parseFloat(widthMatch[1]) : 800;
        const h = heightMatch ? parseFloat(heightMatch[1]) : 600;
        parsedVb = { x: 0, y: 0, w: Math.max(w, 100), h: Math.max(h, 100) };
      }

      baseViewBox = parsedVb;
    } else {
      baseViewBox = null;
    }
  });

  // Asegurar que el elemento SVG tome el 100% del contenedor
  $effect(() => {
    if (svgContent && containerRef) {
      tick().then(() => {
        const svg = containerRef?.querySelector('svg');
        if (svg) {
          svg.style.width = '100%';
          svg.style.height = '100%';
          svg.style.maxWidth = 'none';
          svg.style.maxHeight = 'none';
        }
      });
    }
  });

  // Transformación interactiva del ViewBox con Pan & Zoom
  $effect(() => {
    const s = scale;
    const px = panX;
    const py = panY;
    const bVb = baseViewBox;

    if (containerRef && bVb) {
      const svg = containerRef.querySelector('svg');
      if (svg) {
        const { x, y, w, h } = bVb;
        const curScale = Math.max(0.1, s);
        const newW = w / curScale;
        const newH = h / curScale;

        const newX = x + (w - newW) / 2 + px;
        const newY = y + (h - newH) / 2 + py;

        svg.setAttribute('viewBox', `${newX} ${newY} ${newW} ${newH}`);
      }
    }
  });

  /**
   * Unidades del viewBox por píxel de pantalla con el zoom `s`. El SVG se ajusta al contenedor
   * conservando la proporción (como `xMidYMid meet`): manda el lado que más se ajusta.
   */
  function unitsPerPx(s: number): number {
    if (!containerRef || !baseViewBox) return 1;
    const rect = containerRef.getBoundingClientRect();
    if (!rect.width || !rect.height) return 1;
    const curScale = Math.max(0.1, s);
    return Math.max(baseViewBox.w / curScale / rect.width, baseViewBox.h / curScale / rect.height);
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomSensitivity = 0.002;
    const delta = -e.deltaY * zoomSensitivity;
    const next = Math.max(0.1, Math.min(scale * Math.exp(delta), 10));
    // El punto del diagrama bajo el cursor se queda quieto
    if (containerRef) {
      const rect = containerRef.getBoundingClientRect();
      const mx = e.clientX - (rect.left + rect.width / 2);
      const my = e.clientY - (rect.top + rect.height / 2);
      const before = unitsPerPx(scale);
      const after = unitsPerPx(next);
      panX += mx * (before - after);
      panY += my * (before - after);
    }
    scale = next;
  }

  /** El arrastre en curso dibuja con el láser (clic izquierdo en modo láser) en vez de mover. */
  let drawing = $state(false);

  // Clic izquierdo: mueve el diagrama (o dibuja en modo láser). Clic derecho mantenido: siempre mueve.
  function handlePointerDown(e: PointerEvent) {
    if (e.button > 2) return;
    if (e.button === 1) e.preventDefault(); // sin autodesplazamiento ni pegado del botón del medio
    isDragging = true;
    drawing = isLaserMode && e.button === 0;
    if (drawing) {
      laser?.start(e.clientX, e.clientY);
    } else {
      startX = e.clientX;
      startY = e.clientY;
      startPanX = panX;
      startPanY = panY;
    }
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (isDragging) {
      if (drawing) {
        laser?.move(e.clientX, e.clientY);
      } else {
        // El diagrama sigue al mouse 1 a 1, con cualquier zoom
        const upp = unitsPerPx(scale);
        panX = startPanX - (e.clientX - startX) * upp;
        panY = startPanY - (e.clientY - startY) * upp;
      }
    }
  }

  function handlePointerUp(e: PointerEvent) {
    isDragging = false;
    drawing = false;
    laser?.end();
    const el = e.target as HTMLElement;
    if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId);
  }
</script>

<div
  class="diagram-viewer-container"
  class:split-mode={isEditorVisible}
  class:is-resizing={isResizing}
  bind:this={viewerContainerRef}
>
  {#if isEditorVisible}
    <div class="editor-pane" style:width={editorWidthPct !== null ? `${editorWidthPct}%` : undefined}>
      <div class="editor-header-bar">
        <div class="editor-title-group">
          <span>{t('diagramViewer.codeHeading', { engine: engineName })}</span>
          <span class="engine-badge" title={engineTooltip || t('diagramViewer.viewerTooltip', { engine: engineName })}>{engineName}</span>
          {#if readOnly}
            <span class="readonly-badge" title={t('diagramViewer.readOnlyTitle')}>{t('diagramViewer.readOnlyBadge')}</span>
          {/if}
        </div>
        <div class="editor-header-actions">
          {#if error}
            <span class="syntax-error-badge">{t('diagramViewer.syntaxError')}</span>
          {/if}
          <button
            type="button"
            class="editor-header-btn"
            onclick={() => {
              if (codeEditorRef) codeEditorRef.triggerSearch();
            }}
            title={t('diagramViewer.searchTitle')}
          >
            <Search size={12} />
            <span>{t('diagramViewer.search')}</span>
          </button>

          {#if !readOnly}
            <button
              type="button"
              class="editor-save-btn"
              onclick={() => {
                if (onChange) onChange(content);
              }}
              title={t('diagramViewer.saveTitle')}
            >
              <Save size={12} />
              <span>{t('diagramViewer.save')}</span>
            </button>
          {/if}
        </div>
      </div>

      <div class="editor-body">
        <CodeEditor
          bind:this={codeEditorRef}
          {content}
          {readOnly}
          {vimMode}
          {isActive}
          {onRun}
          {mode}
          {diagnostics}
          onChange={(newVal) => {
            if (onChange) onChange(newVal);
          }}
          onSelectionChange={(info) => {
            if (onSelectionChange) onSelectionChange(info);
          }}
        />
      </div>

      {#if error}
        <div class="editor-error-footer">
          <AlertCircle size={14} />
          <pre>{error}</pre>
        </div>
      {/if}
    </div>
    <SplitResizer
      container={viewerContainerRef}
      label={t('diagramViewer.resizeCodeAria')}
      bind:active={isResizing}
      onResize={(pct) => (editorWidthPct = pct)}
      onResizeEnd={(pct) => {
        if (filePath) diagramLayoutApi.setSplitRatio(filePath, pct);
      }}
    />
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="preview-pane"
    class:laser-cursor={isLaserMode && !(isDragging && !drawing)}
    style:--laser-cursor={LASER_CURSOR}
    bind:this={containerRef}
    onwheel={handleWheel}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
    oncontextmenu={(e) => e.preventDefault()}
    use:laserShortcuts={{ isActive: () => isLaserMode, setActive: (v) => (isLaserMode = v) }}
  >
    {#if error}
      <div class="error-msg">
        <div class="error-title">{t('diagramViewer.renderErrorTitle', { engine: engineName })}</div>
        <pre>{error}</pre>
      </div>
    {/if}
    {#if !isReady}
      <div class="loading">{effectiveLoadingMessage}</div>
    {/if}
    {#if svgContent && !error}
      <div class="svg-container">
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html svgContent}
      </div>
    {/if}

    <!-- Capa de lienzo transparente para el Puntero Láser -->
    <LaserCanvas bind:this={laser} active={isLaserMode} />
  </div>

  <Toolbar.Root class="zoom-controls" aria-label={t('diagramViewer.zoomControlsAria')}>
    <LaserToggle bind:active={isLaserMode} />
    <Separator.Root class="separator" orientation="vertical" />
    <Toolbar.Button onclick={() => (scale = Math.max(0.1, scale / 1.2))} aria-label={t('diagramViewer.zoomOutAria')}>-</Toolbar.Button>
    <span>{Math.round(scale * 100)}%</span>
    <Toolbar.Button onclick={() => (scale = Math.min(10, scale * 1.2))} aria-label={t('diagramViewer.zoomInAria')}>+</Toolbar.Button>
    <Toolbar.Button
      class="reset-btn"
      onclick={() => {
        scale = 1;
        panX = 0;
        panY = 0;
      }}
      aria-label={t('diagramViewer.resetZoomAria')}>{t('diagramViewer.resetZoom')}</Toolbar.Button
    >
  </Toolbar.Root>
</div>

<style>
  .diagram-viewer-container {
    display: flex;
    flex-direction: row;
    height: 100%;
    width: 100%;
    box-sizing: border-box;
    position: relative;
    overflow: hidden;
    background: var(--bg-primary, #ffffff);
  }

  .editor-pane {
    width: 40%;
    min-width: 160px;
    height: 100%;
    border-right: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .is-resizing {
    user-select: none;
    cursor: col-resize;
  }

  .editor-header-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-primary, #ffffff);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
  }

  .editor-title-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .engine-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent, #0969da);
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    padding: 1px 6px;
    border-radius: 4px;
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    letter-spacing: 0.3px;
  }

  .readonly-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--badge-bg);
    color: var(--text-secondary, #656d76);
    border: 1px solid var(--border-primary, #d0d7de);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .editor-header-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    font-size: 11px;
    font-weight: 500;
    background: transparent;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 4px;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .editor-header-btn:hover {
    background: var(--hover-bg);
    color: var(--text-primary, #1f2328);
  }

  .editor-header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .editor-save-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.3));
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent, #0969da);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .editor-save-btn:hover {
    background: var(--accent, #0969da);
    color: var(--on-accent);
  }

  .syntax-error-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--danger);
    background: var(--error-bg);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--danger-border);
    text-transform: none;
  }

  .editor-body {
    flex: 1;
    display: flex;
    position: relative;
    overflow: hidden;
    background: var(--bg-primary, #ffffff);
  }

  .editor-error-footer {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    background: var(--error-bg);
    border-top: 1px solid var(--danger-border);
    color: var(--danger);
    font-size: 12px;
    font-family: var(--code-font, monospace);
    max-height: 120px;
    overflow-y: auto;
  }

  .editor-error-footer pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
  }

  .editor-error-footer :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }

  .preview-pane {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-width: 0;
    cursor: grab;
    touch-action: none;
    position: relative;
  }

  .preview-pane:active {
    cursor: grabbing;
  }

  .preview-pane.laser-cursor,
  .preview-pane.laser-cursor:active {
    cursor: var(--laser-cursor, crosshair);
  }

  .svg-container {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .svg-container :global(svg) {
    width: 100%;
    height: 100%;
    max-width: none;
    pointer-events: none;
    user-select: none;
  }

  .error-msg {
    color: var(--danger);
    background: var(--error-bg);
    padding: 14px;
    border-radius: 8px;
    font-family: var(--code-font, monospace);
    font-size: 13px;
    width: 85%;
    border: 1px solid var(--danger-border);
    z-index: 10;
    box-shadow: 0 4px 12px color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .error-title {
    font-weight: 600;
    margin-bottom: 8px;
    font-size: 14px;
  }

  .error-msg pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .loading {
    color: var(--text-secondary);
    font-size: 14px;
    padding: 20px;
  }

  :global(.zoom-controls) {
    position: absolute;
    bottom: 24px;
    right: 24px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-secondary, #f6f8fa);
    padding: 8px 16px;
    border-radius: 8px;
    border: 1px solid var(--border-primary, #d0d7de);
    box-shadow: var(--card-shadow-hover);
    z-index: 10;
  }

  :global(.zoom-controls .separator) {
    width: 1px;
    height: 16px;
    background-color: var(--border-primary, #d0d7de);
    margin: 0 4px;
  }

  :global(.zoom-controls button) {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-primary);
    padding: 4px 8px;
    border-radius: 4px;
    font-weight: 500;
  }

  :global(.zoom-controls button:hover) {
    background: var(--hover-bg);
  }

  :global(.zoom-controls span) {
    font-size: 13px;
    min-width: 45px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  :global(.zoom-controls .reset-btn) {
    font-size: 12px;
    margin-left: 4px;
    border-left: 1px solid var(--border-primary, #d0d7de) !important;
    border-radius: 0 4px 4px 0 !important;
    padding-left: 8px !important;
  }
</style>
