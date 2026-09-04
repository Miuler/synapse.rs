<script lang="ts">
  interface Props {
    src: string;
    alt?: string;
  }

  let { src, alt = 'Imagen' }: Props = $props();

  console.debug('ImageViewer props:', { src, alt });

  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let startPanX = 0;
  let startPanY = 0;

  let containerRef = $state<HTMLDivElement | null>(null);

  function zoomIn() {
    zoom = Math.min(zoom * 1.25, 5);
  }

  function zoomOut() {
    zoom = Math.max(zoom / 1.25, 0.2);
  }

  function resetZoom() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      if (e.deltaY < 0) {
        zoomIn();
      } else {
        zoomOut();
      }
    }
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    isDragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    startPanX = panX;
    startPanY = panY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    panX = startPanX + (e.clientX - dragStartX);
    panY = startPanY + (e.clientY - dragStartY);
  }

  function handleMouseUp() {
    isDragging = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="image-viewer-container"
  bind:this={containerRef}
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  role="region"
  aria-label="Visor de imagen"
>
  <div
    class="image-stage"
    style="transform: translate({panX}px, {panY}px) scale({zoom}); cursor: {isDragging ? 'grabbing' : zoom > 1 ? 'grab' : 'default'};"
  >
    <img {src} {alt} class="viewer-img" draggable="false" />
  </div>

  <!-- Barra de control flotante para zoom -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="image-controls" onmousedown={(e) => e.stopPropagation()}>
    <button type="button" class="ctrl-btn" onclick={zoomOut} title="Alejar (-)">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12"/>
      </svg>
    </button>
    <button type="button" class="ctrl-btn zoom-indicator" onclick={resetZoom} title="Restablecer zoom (100%)">
      {Math.round(zoom * 100)}%
    </button>
    <button type="button" class="ctrl-btn" onclick={zoomIn} title="Acercar (+)">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19"/>
        <line x1="5" y1="12" x2="19" y2="12"/>
      </svg>
    </button>
    <button type="button" class="ctrl-btn" onclick={resetZoom} title="Ajustar al centro">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .image-viewer-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--bg-secondary, #f6f8fa);
    background-image: 
      linear-gradient(45deg, rgba(0, 0, 0, 0.02) 25%, transparent 25%), 
      linear-gradient(-45deg, rgba(0, 0, 0, 0.02) 25%, transparent 25%), 
      linear-gradient(45deg, transparent 75%, rgba(0, 0, 0, 0.02) 75%), 
      linear-gradient(-45deg, transparent 75%, rgba(0, 0, 0, 0.02) 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0px;
    display: flex;
    align-items: center;
    justify-content: center;
    user-select: none;
  }

  .image-stage {
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: 90%;
    max-height: 90%;
    transition: transform 0.08s ease-out;
    transform-origin: center center;
  }

  .viewer-img {
    max-width: 100%;
    max-height: 80vh;
    width: auto;
    height: auto;
    object-fit: contain;
    border-radius: 6px;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.12), 0 1px 3px rgba(0, 0, 0, 0.08);
    background-color: #ffffff;
  }

  .image-controls {
    position: absolute;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 2px;
    background: rgba(255, 255, 255, 0.92);
    backdrop-filter: blur(8px);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 20px;
    padding: 3px 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    z-index: 10;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-primary, #1f2328);
    padding: 6px 8px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    transition: background 0.15s ease;
  }

  .ctrl-btn:hover {
    background: rgba(0, 0, 0, 0.06);
  }

  .zoom-indicator {
    min-width: 44px;
    font-variant-numeric: tabular-nums;
  }
</style>
