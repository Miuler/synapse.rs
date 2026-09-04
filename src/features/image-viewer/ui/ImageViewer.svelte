<script lang="ts">
  interface Props {
    src: string;
    alt?: string;
  }

  let { src, alt = 'Imagen' }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let imgRef = $state<HTMLImageElement | null>(null);

  let naturalWidth = $state(0);
  let naturalHeight = $state(0);

  let zoom = $state(1);
  let fitZoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;

  function calculateFitZoom() {
    if (!containerRef || !naturalWidth || !naturalHeight) return 1;
    const padding = 64;
    const availW = Math.max(containerRef.clientWidth - padding, 100);
    const availH = Math.max(containerRef.clientHeight - padding, 100);
    const scaleW = availW / naturalWidth;
    const scaleH = availH / naturalHeight;
    return Math.min(scaleW, scaleH, 1);
  }

  function handleImageLoad(e: Event) {
    const img = e.target as HTMLImageElement;
    naturalWidth = img.naturalWidth;
    naturalHeight = img.naturalHeight;
    fitZoom = calculateFitZoom();
    zoom = fitZoom;
    panX = 0;
    panY = 0;
  }

  // Reiniciar cuando cambia la imagen
  $effect(() => {
    if (src) {
      naturalWidth = 0;
      naturalHeight = 0;
      zoom = 1;
      panX = 0;
      panY = 0;
    }
  });

  function zoomIn() {
    zoom = Math.min(zoom * 1.25, 10);
  }

  function zoomOut() {
    zoom = Math.max(zoom / 1.25, 0.05);
  }

  function toggleFitOrActual() {
    if (Math.abs(zoom - fitZoom) < 0.01) {
      zoom = 1;
    } else {
      zoom = fitZoom;
      panX = 0;
      panY = 0;
    }
  }

  function setActualSize() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function fitToWindow() {
    fitZoom = calculateFitZoom();
    zoom = fitZoom;
    panX = 0;
    panY = 0;
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const zoomFactor = e.deltaY < 0 ? 1.2 : 0.833;
      const newZoom = Math.max(0.05, Math.min(zoom * zoomFactor, 10));

      if (containerRef) {
        const rect = containerRef.getBoundingClientRect();
        const mouseX = e.clientX - (rect.left + rect.width / 2);
        const mouseY = e.clientY - (rect.top + rect.height / 2);

        const ratio = newZoom / zoom;
        panX = mouseX - (mouseX - panX) * ratio;
        panY = mouseY - (mouseY - panY) * ratio;
      }

      zoom = newZoom;
    } else {
      if (e.shiftKey) {
        panX -= e.deltaY;
      } else {
        panY -= e.deltaY;
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
  bind:this={containerRef}
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  role="region"
  aria-label="Visor de imagen"
  style="cursor: {isDragging ? 'grabbing' : 'grab'};"
>
  <div
    class="image-viewport"
    style="transform: translate({panX}px, {panY}px);"
  >
    <img
      bind:this={imgRef}
      {src}
      {alt}
      onload={handleImageLoad}
      style={naturalWidth > 0
        ? `width: ${Math.round(naturalWidth * zoom)}px; height: ${Math.round(naturalHeight * zoom)}px; opacity: 1;`
        : 'max-width: 90vw; max-height: 80vh; opacity: 0;'}
      draggable="false"
    />
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="image-controls" onmousedown={(e) => e.stopPropagation()}>
    <button type="button" class="ctrl-btn" onclick={zoomOut} title="Alejar (Ctrl + Rueda hacia abajo)">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
    <button type="button" class="zoom-level-btn" onclick={toggleFitOrActual} title="Alternar entre Ajustar y 100%">
      {Math.round(zoom * 100)}%
    </button>
    <button type="button" class="ctrl-btn" onclick={zoomIn} title="Acercar (Ctrl + Rueda hacia arriba)">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
    <div class="divider"></div>
    <button type="button" class="ctrl-btn text-btn" onclick={setActualSize} title="Tamaño real (1:1 / 100%)">
      1:1
    </button>
    <button type="button" class="ctrl-btn" onclick={fitToWindow} title="Ajustar a ventana">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
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

  .image-viewport {
    display: flex;
    align-items: center;
    justify-content: center;
    will-change: transform;
    pointer-events: none;
    transform-origin: center center;
  }

  img {
    display: block;
    max-width: none;
    max-height: none;
    border-radius: 4px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
    pointer-events: none;
    image-rendering: auto;
    image-rendering: -webkit-optimize-contrast;
    transition: opacity 0.15s ease;
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
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.15);
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

  .text-btn {
    font-size: 11px;
    font-weight: 700;
    font-family: var(--code-font, monospace);
  }

  .zoom-level-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    min-width: 44px;
    text-align: center;
    color: var(--text-secondary, #57606a);
    font-family: var(--code-font, monospace);
    padding: 2px 6px;
    border-radius: 4px;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .zoom-level-btn:hover {
    background-color: var(--bg-hover, rgba(0, 0, 0, 0.08));
    color: var(--text-primary, #24292f);
  }

  .divider {
    width: 1px;
    height: 16px;
    background-color: var(--border-primary, #d0d7de);
    margin: 0 2px;
  }
</style>
