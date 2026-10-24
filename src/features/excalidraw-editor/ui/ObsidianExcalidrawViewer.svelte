<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import {
    parseExcalidrawContent,
    serializeExcalidrawMarkdown,
    isExcalidrawMarkdown,
    extractWikilinksFromElements,
    type ExcalidrawScene,
    type SynapseApp,
    ExcalidrawAutomate,
  } from '@shared/obsidian-api';
  import { vaultRepository } from '@shared/repositories';
  import { appSettings } from '@entities/settings';
  import {
    Eye,
    Code,
    Download,
    ZoomIn,
    ZoomOut,
    RotateCcw,
    Sparkles,
    FileCode,
    Link,
    Check,
    PenTool,
    Settings,
  } from 'lucide-svelte';

  interface Props {
    content: string;
    filePath?: string;
    readOnly?: boolean;
    synapseApp?: SynapseApp | null;
    onChange?: (content: string) => void;
    onSave?: (content?: string) => void;
    onNavigate?: (path: string) => void;
    onSwitchToNative?: () => void;
  }

  let {
    content = '',
    filePath = '',
    readOnly = false,
    synapseApp = null,
    onChange,
    onSave,
    onNavigate,
    onSwitchToNative,
  }: Props = $props();

  let viewMode = $state<'visual' | 'text'>('visual');
  let rawText = $state(untrack(() => content));
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let copiedNotice = $state(false);
  let resolvedImagesMap = $state<Record<string, any>>({});

  // Parsear la escena y extraer metadatos
  let parsedDoc = $derived.by(() => {
    try {
      return parseExcalidrawContent(rawText);
    } catch (e) {
      console.warn('[ObsidianExcalidrawViewer] Error al parsear contenido:', e);
      return {
        scene: {
          type: 'excalidraw' as const,
          version: 2,
          source: 'synapse-obsidian',
          elements: [],
          appState: {},
          files: {},
        },
        textElements: [],
        wikilinks: [],
        isMarkdown: false,
        frontmatter: {},
        embeddedFiles: undefined,
      };
    }
  });

  // Cargar imágenes incrustadas (## Embedded Files)
  $effect(() => {
    const embedded = parsedDoc.embeddedFiles;
    if (!embedded || Object.keys(embedded).length === 0) return;

    for (const [fileId, targetPath] of Object.entries(embedded)) {
      if (resolvedImagesMap[fileId] || typeof targetPath !== 'string') continue;

      vaultRepository.readAssetDataUrl(targetPath, filePath).then((dataUrl) => {
        if (dataUrl) {
          resolvedImagesMap = {
            ...resolvedImagesMap,
            [fileId]: { id: fileId, dataURL: dataUrl },
          };
        }
      }).catch((err) => {
        console.warn('[ObsidianExcalidrawViewer] No se pudo resolver imagen incrustada:', { fileId, targetPath, err });
      });
    }
  });

  let scene = $derived(parsedDoc.scene);
  let isMarkdownFormat = $derived(parsedDoc.isMarkdown || isExcalidrawMarkdown(rawText) || filePath.toLowerCase().endsWith('.excalidraw.md'));
  let wikilinks = $derived(extractWikilinksFromElements(scene.elements || []));
  let activeElementCount = $derived((scene.elements || []).filter((el) => !el.isDeleted).length);

  // Generar SVG con ExcalidrawAutomate
  let svgContent = $derived.by(() => {
    try {
      const ea = new ExcalidrawAutomate(synapseApp || undefined);
      const combinedFiles = {
        ...(scene.files || {}),
        ...resolvedImagesMap,
      };
      return ea.toSVG(scene.elements || [], combinedFiles);
    } catch (e) {
      console.warn('[ObsidianExcalidrawViewer] Error al generar SVG:', e);
      return `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><text x="10" y="50" fill="gray">No se pudo renderizar el lienzo</text></svg>`;
    }
  });

  // Reaccionar ante cambios externos del contenido
  $effect(() => {
    if (content !== untrack(() => rawText)) {
      rawText = content;
    }
  });

  function handleTextChange(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    rawText = target.value;
    if (onChange) {
      onChange(rawText);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      if (onSave) {
        onSave(getCurrentSerializedScene());
      }
    }
  }

  export function getCurrentSerializedScene(): string {
    return rawText;
  }

  function toggleViewMode() {
    if (viewMode === 'visual') {
      viewMode = 'text';
    } else {
      // Al volver a modo visual, sincronizamos y formateamos si fuera necesario
      viewMode = 'visual';
    }
  }

  function handleZoomIn() {
    zoom = Math.min(zoom + 0.2, 3);
  }

  function handleZoomOut() {
    zoom = Math.max(zoom - 0.2, 0.3);
  }

  function handleResetView() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function handlePointerDown(e: PointerEvent) {
    if (e.button === 0 || e.button === 1) { // Click primario o botón central
      isDragging = true;
      dragStartX = e.clientX - panX;
      dragStartY = e.clientY - panY;
      (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    panX = e.clientX - dragStartX;
    panY = e.clientY - dragStartY;
  }

  function handlePointerUp(e: PointerEvent) {
    if (isDragging) {
      isDragging = false;
      try {
        (e.currentTarget as HTMLElement)?.releasePointerCapture(e.pointerId);
      } catch {
        // Ignorar si ya se liberó
      }
    }
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const delta = e.deltaY < 0 ? 0.1 : -0.1;
      zoom = Math.min(Math.max(zoom + delta, 0.3), 3);
    }
  }

  async function handleExportSvg() {
    try {
      const blob = new Blob([svgContent], { type: 'image/svg+xml;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      const filename = filePath.split('/').pop()?.replace(/\.(excalidraw\.md|excalidraw|md)$/i, '') || 'dibujo';
      a.href = url;
      a.download = `${filename}.svg`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.warn('Error al exportar SVG:', e);
    }
  }

  async function handleExportPng() {
    try {
      const ea = new ExcalidrawAutomate(synapseApp || undefined);
      const buffer = await ea.toPNG(scene.elements || []);
      const blob = new Blob([buffer], { type: 'image/png' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      const filename = filePath.split('/').pop()?.replace(/\.(excalidraw\.md|excalidraw|md)$/i, '') || 'dibujo';
      a.href = url;
      a.download = `${filename}.png`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.warn('Error al exportar PNG:', e);
    }
  }

  function handleToggleDefaultViewer() {
    appSettings.toggleDefaultExcalidrawViewer();
  }
</script>

<!-- Contenedor Principal del Visor Obsidian Excalidraw -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="obsidian-excalidraw-container" onkeydown={handleKeyDown} tabindex="0" role="region" aria-label="Visor Obsidian Excalidraw">
  <!-- Cabecera de Acciones y Modos -->
  <header class="obsidian-viewer-header">
    <div class="header-left">
      <div class="plugin-badge" title="Visualizado mediante el motor del plugin de Obsidian Excalidraw">
        <Sparkles size={13} class="badge-icon" />
        <span>Obsidian Excalidraw</span>
      </div>

      <div class="mode-switch-group" role="group" aria-label="Alternar modo de vista">
        <button
          type="button"
          class="mode-btn"
          class:active={viewMode === 'visual'}
          onclick={() => (viewMode = 'visual')}
          title="Vista gráfica vectorial del dibujo"
        >
          <Eye size={13} />
          <span>Lienzo</span>
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={viewMode === 'text'}
          onclick={() => (viewMode = 'text')}
          title="Vista de código fuente (Markdown / JSON)"
        >
          <Code size={13} />
          <span>Código</span>
        </button>
      </div>
    </div>

    <div class="header-right">
      {#if viewMode === 'visual'}
        <button
          type="button"
          class="action-btn"
          onclick={handleExportSvg}
          title="Descargar diagrama en formato vectorial SVG"
        >
          <Download size={13} />
          <span>SVG</span>
        </button>

        <button
          type="button"
          class="action-btn"
          onclick={handleExportPng}
          title="Descargar diagrama como imagen PNG transparente"
        >
          <Download size={13} />
          <span>PNG</span>
        </button>
      {/if}

      {#if onSwitchToNative}
        <button
          type="button"
          class="action-btn switch-btn"
          onclick={onSwitchToNative}
          title="Abrir con el editor nativo de Excalidraw en esta pestaña"
        >
          <PenTool size={13} />
          <span>Abrir con Excalidraw nativo</span>
        </button>
      {/if}

      <button
        type="button"
        class="action-btn config-btn"
        class:is-default={appSettings.defaultExcalidrawViewer === 'obsidian'}
        onclick={handleToggleDefaultViewer}
        title="Alternar si los archivos de dibujo se abren por defecto con Excalidraw nativo o con el plugin de Obsidian"
      >
        <Settings size={13} />
        <span>
          Por defecto: {appSettings.defaultExcalidrawViewer === 'obsidian' ? 'Obsidian' : 'Nativo'}
        </span>
      </button>
    </div>
  </header>

  <!-- Área Central de Contenido -->
  <main class="viewer-body">
    {#if viewMode === 'visual'}
      <!-- Vista de Lienzo SVG Interactiva -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="stage-viewport"
        class:is-dragging={isDragging}
        onpointerdown={handlePointerDown}
        onpointermove={handlePointerMove}
        onpointerup={handlePointerUp}
        onwheel={handleWheel}
      >
        <!-- Envoltorio escalable y desplazable -->
        <div
          class="svg-transform-wrapper"
          style="transform: translate({panX}px, {panY}px) scale({zoom}); transform-origin: center center;"
        >
          <!-- Contenido SVG inyectado -->
          <div class="svg-stage">
            {@html svgContent}
          </div>
        </div>

        <!-- Controles flotantes de Zoom y Posición -->
        <div class="zoom-controls">
          <button type="button" class="zoom-btn" onclick={handleZoomIn} title="Acercar (Ctrl + Rueda arriba)">
            <ZoomIn size={14} />
          </button>
          <button type="button" class="zoom-btn" onclick={handleResetView} title="Restablecer escala al 100%">
            <span class="zoom-label">{Math.round(zoom * 100)}%</span>
          </button>
          <button type="button" class="zoom-btn" onclick={handleZoomOut} title="Alejar (Ctrl + Rueda abajo)">
            <ZoomOut size={14} />
          </button>
          <button type="button" class="zoom-btn" onclick={handleResetView} title="Centrar dibujo">
            <RotateCcw size={13} />
          </button>
        </div>

        <!-- Barra flotante de Wikilinks embebidos si existen -->
        {#if wikilinks.length > 0}
          <div class="wikilinks-overlay">
            <span class="wikilinks-title">
              <Link size={12} />
              <span>Enlaces detectados:</span>
            </span>
            <div class="wikilinks-tags">
              {#each wikilinks as link}
                <button
                  type="button"
                  class="wikilink-pill"
                  onclick={() => onNavigate && onNavigate(link)}
                  title="Abrir nota vinculada [[{link}]]"
                >
                  [[{link}]]
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <!-- Modo Editor de Texto / Markdown Crudo -->
      <div class="text-editor-container">
        <textarea
          class="raw-textarea"
          value={rawText}
          readonly={readOnly}
          oninput={handleTextChange}
          placeholder="Contenido de Excalidraw en formato Markdown u Obsidian JSON..."
          spellcheck="false"
        ></textarea>
      </div>
    {/if}
  </main>

  <!-- Barra de Estado Inferior -->
  <footer class="obsidian-viewer-footer">
    <div class="footer-left">
      <span class="footer-item">
        <FileCode size={12} />
        <span>Formato: {isMarkdownFormat ? 'Obsidian Markdown (.excalidraw.md)' : 'Excalidraw JSON (.excalidraw)'}</span>
      </span>
      <span class="footer-separator">•</span>
      <span class="footer-item">
        <span>Elementos: {activeElementCount}</span>
      </span>
      {#if wikilinks.length > 0}
        <span class="footer-separator">•</span>
        <span class="footer-item">
          <span>Wikilinks: {wikilinks.length}</span>
        </span>
      {/if}
    </div>

    <div class="footer-right">
      <span class="footer-item mode-indicator">
        Modo actual: <strong>{viewMode === 'visual' ? 'Lienzo SVG' : 'Código Fuente'}</strong>
      </span>
    </div>
  </footer>
</div>

<style>
  .obsidian-excalidraw-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    position: relative;
    overflow: hidden;
    outline: none;
    font-family: inherit;
  }

  /* Cabecera superior */
  .obsidian-viewer-header {
    height: 40px;
    padding: 0 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--bg-secondary, #f6f8fa);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    flex-shrink: 0;
    gap: 8px;
    user-select: none;
  }

  .header-left,
  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .plugin-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    border-radius: 12px;
    background-color: color-mix(in srgb, var(--accent, #6366f1) 14%, transparent);
    color: var(--accent, #6366f1);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.2px;
  }

  :global(.badge-icon) {
    color: var(--accent, #6366f1);
  }

  /* Selector de modo (Lienzo / Código) */
  .mode-switch-group {
    display: flex;
    align-items: center;
    background-color: var(--bg-tertiary, #eaeef2);
    border-radius: 6px;
    padding: 2px;
    border: 1px solid var(--border-secondary, #e1e4e8);
  }

  .mode-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 9px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary, #656d76);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    color: var(--text-primary, #1f2328);
  }

  .mode-btn.active {
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1f2328);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }

  /* Botones de acción */
  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-secondary, #d0d7de);
    background-color: var(--bg-primary, #ffffff);
    color: var(--text-secondary, #656d76);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background-color: var(--bg-tertiary, #f3f4f6);
    color: var(--text-primary, #1f2328);
    border-color: var(--border-primary, #babbbe);
  }

  .action-btn.switch-btn {
    border-color: color-mix(in srgb, var(--accent, #6366f1) 40%, transparent);
    color: var(--accent, #6366f1);
  }

  .action-btn.switch-btn:hover {
    background-color: color-mix(in srgb, var(--accent, #6366f1) 10%, transparent);
  }

  .action-btn.config-btn.is-default {
    background-color: color-mix(in srgb, var(--accent, #6366f1) 12%, transparent);
    color: var(--accent, #6366f1);
    border-color: var(--accent, #6366f1);
  }

  /* Cuerpo del visor */
  .viewer-body {
    flex: 1;
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .stage-viewport {
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
    cursor: grab;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-primary, #ffffff);
    background-image: radial-gradient(
      var(--border-secondary, #e1e4e8) 1px,
      transparent 1px
    );
    background-size: 20px 20px;
  }

  .stage-viewport.is-dragging {
    cursor: grabbing;
    user-select: none;
  }

  .svg-transform-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.05s ease-out;
    pointer-events: none;
  }

  .svg-stage {
    display: inline-block;
    filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.06));
    max-width: 90vw;
    max-height: 85vh;
  }

  .svg-stage :global(svg) {
    display: block;
    max-width: 100%;
    max-height: 100%;
    height: auto;
  }

  /* Controles de Zoom */
  .zoom-controls {
    position: absolute;
    bottom: 16px;
    right: 16px;
    display: flex;
    align-items: center;
    background-color: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 10;
    overflow: hidden;
  }

  .zoom-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 6px 9px;
    border: none;
    background: transparent;
    color: var(--text-secondary, #656d76);
    cursor: pointer;
    font-size: 11px;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .zoom-btn:hover {
    background-color: var(--bg-tertiary, #e1e4e8);
    color: var(--text-primary, #1f2328);
  }

  .zoom-label {
    font-family: monospace;
    font-size: 11px;
    min-width: 38px;
    text-align: center;
  }

  /* Overlay de Wikilinks */
  .wikilinks-overlay {
    position: absolute;
    top: 14px;
    left: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    background-color: color-mix(in srgb, var(--bg-secondary, #f6f8fa) 92%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 6px 10px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    z-index: 10;
    max-width: 70%;
    flex-wrap: wrap;
  }

  .wikilinks-title {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    font-weight: 500;
  }

  .wikilinks-tags {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
  }

  .wikilink-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 7px;
    background-color: color-mix(in srgb, var(--accent, #6366f1) 15%, transparent);
    color: var(--accent, #6366f1);
    border: 1px solid color-mix(in srgb, var(--accent, #6366f1) 30%, transparent);
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .wikilink-pill:hover {
    background-color: var(--accent, #6366f1);
    color: #ffffff;
  }

  /* Modo Editor de Texto / Markdown */
  .text-editor-container {
    width: 100%;
    height: 100%;
    padding: 12px;
    box-sizing: border-box;
    background-color: var(--bg-primary, #ffffff);
  }

  .raw-textarea {
    width: 100%;
    height: 100%;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    padding: 12px;
    box-sizing: border-box;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
    line-height: 1.5;
    background-color: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    resize: none;
    outline: none;
    tab-size: 2;
  }

  .raw-textarea:focus {
    border-color: var(--accent, #6366f1);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #6366f1) 20%, transparent);
  }

  /* Pie de barra de estado inferior */
  .obsidian-viewer-footer {
    height: 26px;
    padding: 0 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--bg-secondary, #f6f8fa);
    border-top: 1px solid var(--border-primary, #d0d7de);
    font-size: 11px;
    color: var(--text-secondary, #656d76);
    user-select: none;
    flex-shrink: 0;
  }

  .footer-left,
  .footer-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .footer-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .footer-separator {
    color: var(--border-primary, #d0d7de);
  }

  .mode-indicator strong {
    color: var(--text-primary, #1f2328);
  }
</style>
