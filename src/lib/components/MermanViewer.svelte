<script lang="ts">
  import { initMerman, renderSvg } from '@mermanjs/web';
  import { onMount } from 'svelte';

  interface Props {
    content: string;
    // Ignoramos readOnly porque el componente será siempre de sólo lectura
    readOnly?: boolean;
    onChange?: (content: string) => void;
  }

  let { content = '' }: Props = $props();

  let svgContent = $state('');
  let isReady = $state(false);
  let error = $state('');
  
  let scale = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);

  onMount(async () => {
    try {
      await initMerman();
      isReady = true;
    } catch (e) {
      error = 'Error al cargar merman: ' + String(e);
    }
  });

  $effect(() => {
    if (isReady && content) {
      try {
        svgContent = renderSvg(content);
        error = '';
        scale = 1;
        panX = 0;
        panY = 0;
      } catch (e) {
        error = 'Error de sintaxis Mermaid:\n' + String(e);
      }
    } else if (isReady && !content) {
      svgContent = '';
      error = '';
    }
  });

  function handleWheel(e: WheelEvent) {
    // Si queremos que el zoom funcione siempre (incluso sin Ctrl), quitamos la condición
    e.preventDefault();
    const zoomSensitivity = 0.002;
    const delta = -e.deltaY * zoomSensitivity;
    scale = Math.max(0.1, Math.min(scale * Math.exp(delta), 10));
  }

  function handlePointerDown(e: PointerEvent) {
    isDragging = true;
    startX = e.clientX - panX;
    startY = e.clientY - panY;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (isDragging) {
      panX = e.clientX - startX;
      panY = e.clientY - startY;
    }
  }

  function handlePointerUp(e: PointerEvent) {
    isDragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="merman-container">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div 
    class="preview-pane"
    onwheel={handleWheel}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  >
    {#if error}
      <div class="error-msg">{error}</div>
    {/if}
    {#if !isReady}
      <div class="loading">Cargando Merman...</div>
    {/if}
    {#if svgContent && !error}
      <div class="svg-container" style="transform: translate({panX}px, {panY}px) scale({scale})">
        {@html svgContent}
      </div>
    {/if}
  </div>
  
  <div class="zoom-controls">
    <button onclick={() => scale = Math.max(0.1, scale / 1.2)}>-</button>
    <span>{Math.round(scale * 100)}%</span>
    <button onclick={() => scale = Math.min(10, scale * 1.2)}>+</button>
    <button class="reset-btn" onclick={() => { scale = 1; panX = 0; panY = 0; }}>Reset</button>
  </div>
</div>

<style>
  .merman-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    box-sizing: border-box;
    position: relative;
    overflow: hidden;
    background: var(--bg-primary, #ffffff);
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
  }
  
  .preview-pane:active {
    cursor: grabbing;
  }

  .svg-container {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    transform-origin: center center;
    transition: transform 0.05s ease-out;
    will-change: transform;
  }
  
  .svg-container :global(svg) {
    width: 100%;
    height: 100%;
    max-width: none;
    pointer-events: none; /* Para que el contenedor principal reciba los eventos de puntero */
    user-select: none;
  }

  .error-msg {
    color: #cf222e;
    background: #ffebe9;
    padding: 12px;
    border-radius: 6px;
    font-family: var(--code-font, monospace);
    font-size: 13px;
    white-space: pre-wrap;
    width: 80%;
    border: 1px solid rgba(207, 34, 46, 0.2);
    z-index: 10;
  }

  .loading {
    color: var(--text-secondary);
    font-size: 14px;
    padding: 20px;
  }
  
  .zoom-controls {
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
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    z-index: 10;
  }
  
  .zoom-controls button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-primary);
    padding: 4px 8px;
    border-radius: 4px;
    font-weight: 500;
  }
  
  .zoom-controls button:hover {
    background: rgba(0,0,0,0.05);
  }
  
  .zoom-controls span {
    font-size: 13px;
    min-width: 45px;
    text-align: center;
    font-family: var(--code-font, monospace);
    color: var(--text-primary);
  }
  
  .reset-btn {
    font-size: 13px !important;
    margin-left: 8px;
    border-left: 1px solid var(--border-primary, #d0d7de) !important;
    border-radius: 0 !important;
    padding-left: 12px !important;
  }
</style>
