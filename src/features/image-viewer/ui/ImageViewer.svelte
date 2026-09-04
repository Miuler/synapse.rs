<script lang="ts">
  interface Props {
    src: string;
    alt?: string;
  }

  let { src, alt = 'Imagen' }: Props = $props();

  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;

  function zoomIn() {
    zoom = Math.min(zoom * 1.25, 5);
  }

  function zoomOut() {
    zoom = Math.max(zoom / 1.25, 0.1);
  }

  function resetZoom() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey) {
      e.preventDefault();
      if (e.deltaY < 0) {
        zoomIn();
      } else {
        zoomOut();
      }
    }
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button === 0) {
      isDragging = true;
      dragStartX = e.clientX - panX;
      dragStartY = e.clientY - panY;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      panX = e.clientX - dragStartX;
      panY = e.clientY - dragStartY;
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="image-viewer-container"
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  role="region"
  aria-label="Visor de imagen"
>
  <div
    class="image-wrapper"
    style="transform: translate({panX}px, {panY}px) scale({zoom}); cursor: {isDragging ? 'grabbing' : zoom > 1 ? 'grab' : 'default'};"
  >
    <img {src} {alt} draggable="false" />
  </div>

  <div class="image-controls">
    <button type="button" class="ctrl-btn" onclick={zoomOut} title="Alejar (-)">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
    <span class="zoom-level">{Math.round(zoom * 100)}%</span>
    <button type="button" class="ctrl-btn" onclick={zoomIn} title="Acercar (+)">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
    <button type="button" class="ctrl-btn" onclick={resetZoom} title="Restablecer (100%)">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
        <path d="M3 3v5h5" />
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
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-primary, #ffffff);
    background-image:
      linear-gradient(45deg, var(--border-subtle, rgba(0, 0, 0, 0.04)) 25%, transparent 25%),
      linear-gradient(-45deg, var(--border-subtle, rgba(0, 0, 0, 0.04)) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, var(--border-subtle, rgba(0, 0, 0, 0.04)) 75%),
      linear-gradient(-45deg, transparent 75%, var(--border-subtle, rgba(0, 0, 0, 0.04)) 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0px;
    user-select: none;
  }

  .image-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    transform-origin: center center;
    transition: transform 0.05s ease-out;
    max-width: 100%;
    max-height: 100%;
  }

  img {
    max-width: 90vw;
    max-height: 80vh;
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    pointer-events: none;
  }

  .image-controls {
    position: absolute;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-secondary, #f6f8fa);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 20px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 10;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-primary, #24292f);
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .ctrl-btn:hover {
    background-color: var(--bg-hover, rgba(0, 0, 0, 0.08));
  }

  .zoom-level {
    font-size: 12px;
    font-weight: 600;
    min-width: 40px;
    text-align: center;
    color: var(--text-secondary, #57606a);
    font-family: var(--code-font, monospace);
  }
</style>
