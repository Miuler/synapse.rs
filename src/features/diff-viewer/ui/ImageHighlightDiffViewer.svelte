<script lang="ts">
  import {
    vaultRepository,
    type GitImageDiffResult,
  } from '@shared/repositories';
  import { AlertCircle, CheckCircle2, ZoomIn, ZoomOut, RotateCcw } from 'lucide-svelte';

  interface Props {
    filePath: string;
    isStaged?: boolean;
    baseRevision?: string;
  }

  let { filePath, isStaged = false, baseRevision }: Props = $props();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let diffResult = $state<GitImageDiffResult | null>(null);

  let scale = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);

  const fileName = $derived(filePath.split('/').pop() || filePath);

  async function loadDiff() {
    loading = true;
    error = null;
    try {
      diffResult = await vaultRepository.getGitImageDiff(filePath, isStaged, baseRevision);
    } catch (err: unknown) {
      console.error('Error calculando diff de imagen:', err);
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const _p = filePath;
    const _s = isStaged;
    const _r = baseRevision;
    loadDiff();
  });

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.15 : 0.85;
    scale = Math.max(0.1, Math.min(10, scale * zoomFactor));
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    isDragging = true;
    startX = e.clientX - panX;
    startY = e.clientY - panY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    panX = e.clientX - startX;
    panY = e.clientY - startY;
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function resetZoom() {
    scale = 1;
    panX = 0;
    panY = 0;
  }
</script>

<div class="image-highlight-root">
  {#if loading}
    <div class="state-container">
      <div class="spinner"></div>
      <p>Calculando diferencias píxel por píxel en memoria...</p>
    </div>
  {:else if error}
    <div class="state-container error">
      <AlertCircle size={32} class="error-icon" />
      <h3>Error en comparación de imagen</h3>
      <p>{error}</p>
    </div>
  {:else if !diffResult?.diff_data_url}
    <div class="state-container empty">
      <CheckCircle2 size={32} class="success-icon" />
      <h3>Sin imagen de comparación</h3>
      <p>No se pudo generar el mapa de diferencias (posiblemente un archivo nuevo o eliminado).</p>
    </div>
  {:else}
    <!-- BARRA DE INFORMACIÓN DE DIFERENCIAS -->
    <div class="diff-stats-bar">
      <div class="stat-pill" class:has-diff={diffResult.diff_count > 0}>
        <span class="stat-dot"></span>
        <span>
          {diffResult.diff_count.toLocaleString()} píxeles distintos ({diffResult.diff_percentage.toFixed(2)}%)
        </span>
      </div>

      <div class="dim-badge">
        <span>{diffResult.width} × {diffResult.height} px</span>
      </div>

      <div class="color-legend">
        <span class="legend-swatch magenta"></span>
        <span class="legend-label">Píxeles modificados</span>
      </div>

      <div class="zoom-controls">
        <button type="button" class="ctrl-btn" onclick={() => (scale = Math.min(10, scale * 1.2))} title="Acercar">
          <ZoomIn size={14} />
        </button>
        <span class="zoom-value">{Math.round(scale * 100)}%</span>
        <button type="button" class="ctrl-btn" onclick={() => (scale = Math.max(0.1, scale * 0.8))} title="Alejar">
          <ZoomOut size={14} />
        </button>
        <button type="button" class="ctrl-btn" onclick={resetZoom} title="Restablecer vista">
          <RotateCcw size={13} />
        </button>
      </div>
    </div>

    <!-- ÁREA DE VISUALIZACIÓN INTERACTIVA -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="canvas-viewport"
      onwheel={handleWheel}
      onmousedown={handleMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
      role="region"
      aria-label="Lienzo de imagen diferencial"
    >
      <div
        class="image-transform-wrapper"
        style="transform: translate({panX}px, {panY}px) scale({scale});"
      >
        <img
          src={diffResult.diff_data_url}
          alt="Diff perceptual de {fileName}"
          class="diff-img"
          draggable="false"
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .image-highlight-root {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
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

  .diff-stats-bar {
    height: 36px;
    min-height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    background-color: var(--bg-secondary, #161b22);
    border-bottom: 1px solid var(--border-primary, rgba(255, 255, 255, 0.08));
    font-size: 11px;
    user-select: none;
    z-index: 10;
  }

  .stat-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
    background-color: rgba(46, 160, 67, 0.15);
    color: #3fb950;
    border: 1px solid rgba(46, 160, 67, 0.3);
  }

  .stat-pill.has-diff {
    background-color: rgba(248, 81, 73, 0.15);
    color: #ff3366;
    border-color: rgba(248, 81, 73, 0.35);
  }

  .stat-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: currentColor;
  }

  .dim-badge {
    color: var(--text-secondary, #8b949e);
    font-family: monospace;
    font-size: 11px;
  }

  .color-legend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary, #8b949e);
  }

  .legend-swatch.magenta {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    background-color: #ff0077;
    border: 1px solid rgba(255, 255, 255, 0.4);
  }

  .zoom-controls {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background-color: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-secondary, rgba(255, 255, 255, 0.1));
    border-radius: 4px;
    padding: 2px 4px;
  }

  .ctrl-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: transparent;
    border: none;
    color: var(--text-secondary, #8b949e);
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ctrl-btn:hover {
    background-color: var(--bg-primary, #0f1115);
    color: var(--text-primary, #f0f6fc);
  }

  .zoom-value {
    font-size: 11px;
    font-family: monospace;
    min-width: 36px;
    text-align: center;
    color: var(--text-primary, #f0f6fc);
  }

  .canvas-viewport {
    flex: 1;
    overflow: hidden;
    position: relative;
    cursor: grab;
    display: flex;
    align-items: center;
    justify-content: center;
    background-image:
      linear-gradient(45deg, rgba(255, 255, 255, 0.03) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(255, 255, 255, 0.03) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, rgba(255, 255, 255, 0.03) 75%),
      linear-gradient(-45deg, transparent 75%, rgba(255, 255, 255, 0.03) 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0px;
  }

  .canvas-viewport:active {
    cursor: grabbing;
  }

  .image-transform-wrapper {
    transform-origin: center center;
    will-change: transform;
    transition: transform 0.05s ease-out;
  }

  .diff-img {
    display: block;
    max-width: none;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    border-radius: 2px;
    border: 1px solid var(--border-primary, rgba(255, 255, 255, 0.1));
  }
</style>
