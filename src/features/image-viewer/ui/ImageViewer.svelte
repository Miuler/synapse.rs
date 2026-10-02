<script lang="ts">
  import { Toolbar, Separator } from 'bits-ui';
  import { AlertCircle, Minus, Plus, Maximize2 } from 'lucide-svelte';

  interface Props {
    src: string;
    alt?: string;
    content?: string;
  }

  let { src, alt = 'Imagen', content }: Props = $props();

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

  let hasLoaded = $state(false);
  let hasError = $state(false);

  let isSvg = $derived(
    src.toLowerCase().includes('.svg') ||
    (alt && alt.toLowerCase().endsWith('.svg')) ||
    src.startsWith('data:image/svg') ||
    (content && content.trim().startsWith('<svg'))
  );

  let maxZoom = $derived(isSvg ? 25 : 10);

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
    const nw = img.naturalWidth;
    const nh = img.naturalHeight;

    if (nw > 0 && nh > 0) {
      naturalWidth = nw;
      naturalHeight = nh;
    } else {
      // Para SVGs sin dimensiones intrínsecas fijas o sólo con viewBox
      const fallbackW = img.clientWidth || (containerRef ? Math.max(containerRef.clientWidth - 96, 300) : 800);
      const fallbackH = img.clientHeight || (containerRef ? Math.max(containerRef.clientHeight - 96, 300) : 600);
      naturalWidth = fallbackW > 0 ? fallbackW : 800;
      naturalHeight = fallbackH > 0 ? fallbackH : 600;
    }

    fitZoom = calculateFitZoom();
    zoom = fitZoom;
    panX = 0;
    panY = 0;
    hasLoaded = true;
    hasError = false;
  }

  function handleImageError() {
    hasError = true;
    hasLoaded = false;
  }

  // Reiniciar cuando cambia la imagen
  $effect(() => {
    if (src) {
      naturalWidth = 0;
      naturalHeight = 0;
      zoom = 1;
      panX = 0;
      panY = 0;
      hasLoaded = false;
      hasError = false;
    }
  });

  function zoomIn() {
    zoom = Math.min(zoom * 1.25, maxZoom);
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
      const newZoom = Math.max(0.05, Math.min(zoom * zoomFactor, maxZoom));

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
  {#if hasError}
    <div class="image-error">
      <AlertCircle size={36} strokeWidth={1.7} />
      <p class="error-title">No se pudo cargar la imagen</p>
      <p class="error-path">{alt || src}</p>
    </div>
  {:else}
    <div
      class="image-viewport"
      style="transform: translate({panX}px, {panY}px);"
    >
      <img
        bind:this={imgRef}
        {src}
        {alt}
        onload={handleImageLoad}
        onerror={handleImageError}
        class:is-svg={isSvg}
        style={naturalWidth > 0
          ? `width: ${Math.round(naturalWidth * zoom)}px; height: ${Math.round(naturalHeight * zoom)}px; opacity: 1;`
          : 'max-width: 90vw; max-height: 80vh; opacity: 0;'}
        draggable="false"
      />
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <Toolbar.Root
      class="image-controls"
      aria-label="Controles de zoom de imagen"
      onmousedown={(e) => e.stopPropagation()}
    >
      {#if isSvg}
        <span class="format-badge" title="Formato gráfico vectorial escalable">SVG</span>
        <Separator.Root class="divider" orientation="vertical" />
      {/if}

      <Toolbar.Button class="ctrl-btn" onclick={zoomOut} aria-label="Alejar (Ctrl + Rueda hacia abajo)">
        <Minus size={16} />
      </Toolbar.Button>
      <Toolbar.Button class="zoom-level-btn" onclick={toggleFitOrActual} aria-label="Alternar entre Ajustar y 100%">
        {Math.round(zoom * 100)}%
      </Toolbar.Button>
      <Toolbar.Button class="ctrl-btn" onclick={zoomIn} aria-label="Acercar (Ctrl + Rueda hacia arriba)">
        <Plus size={16} />
      </Toolbar.Button>
      <Separator.Root class="divider" orientation="vertical" />
      <Toolbar.Button class="ctrl-btn text-btn" onclick={setActualSize} aria-label="Tamaño real (1:1 / 100%)">
        1:1
      </Toolbar.Button>
      <Toolbar.Button class="ctrl-btn" onclick={fitToWindow} aria-label="Ajustar a ventana">
        <Maximize2 size={16} />
      </Toolbar.Button>
    </Toolbar.Root>
  {/if}
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
    transition: opacity 0.15s ease;
  }

  img:not(.is-svg) {
    image-rendering: -webkit-optimize-contrast;
  }

  img.is-svg {
    image-rendering: auto;
  }

  .image-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-secondary, #656d76);
    padding: 32px;
    text-align: center;
  }

  .error-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #1f2328);
    margin: 0;
  }

  .error-path {
    font-size: 12px;
    color: var(--text-secondary, #656d76);
    font-family: var(--code-font, monospace);
    margin: 0;
    word-break: break-all;
  }

  :global(.image-controls) {
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

  .format-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    background: var(--accent-bg, rgba(9, 105, 218, 0.1));
    color: var(--accent, #0969da);
    border-radius: 10px;
    border: 1px solid var(--accent-border, rgba(9, 105, 218, 0.25));
  }

  :global(.ctrl-btn) {
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

  :global(.ctrl-btn:hover) {
    background-color: var(--bg-hover, rgba(0, 0, 0, 0.08));
  }

  :global(.text-btn) {
    font-size: 11px;
    font-weight: 700;
    font-family: var(--code-font, monospace);
  }

  :global(.zoom-level-btn) {
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

  :global(.zoom-level-btn:hover) {
    background-color: var(--bg-hover, rgba(0, 0, 0, 0.08));
    color: var(--text-primary, #24292f);
  }

  :global(.image-controls .divider) {
    width: 1px;
    height: 16px;
    background-color: var(--border-primary, #d0d7de);
    margin: 0 2px;
  }
</style>
