<script lang="ts">
  import { initMerman, renderSvg } from '@mermanjs/web';
  import { onMount, onDestroy, tick } from 'svelte';

  interface Props {
    content: string;
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

  // Modo Puntero Láser con mutación in-place para conservar referencias de array
  let isLaserMode = $state(false);
  let canvasRef = $state<HTMLCanvasElement | null>(null);
  
  type LaserPoint = { x: number; y: number; time: number };
  type LaserStroke = LaserPoint[];
  
  let laserStrokes: LaserStroke[] = [];
  let activeStroke: LaserStroke | null = null;
  let animFrameId: number | null = null;

  let containerRef = $state<HTMLDivElement | null>(null);
  let baseViewBox = $state<{ x: number; y: number; w: number; h: number } | null>(null);

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
        const svgStr = renderSvg(content);
        error = '';
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
          parsedVb = { x: 0, y: 0, w: 800, h: 600 };
        }
        baseViewBox = parsedVb;
        svgContent = svgStr;
      } catch (e) {
        error = 'Error de sintaxis Mermaid:\n' + String(e);
      }
    } else if (isReady && !content) {
      svgContent = '';
      error = '';
      baseViewBox = null;
    }
  });

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

        const rect = containerRef.getBoundingClientRect();
        const factorX = rect.width ? w / rect.width : 1;
        const factorY = rect.height ? h / rect.height : 1;

        const newX = (x + (w - newW) / 2) - (px * factorX);
        const newY = (y + (h - newH) / 2) - (py * factorY);

        svg.setAttribute('viewBox', `${newX} ${newY} ${newW} ${newH}`);
      }
    }
  });

  // Renderizado continuo del láser con mutación in-place
  function drawLaserLoop() {
    if (!canvasRef) return;
    const ctx = canvasRef.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, canvasRef.width, canvasRef.height);

    const now = Date.now();
    const maxAge = 1600; // 1600ms de desvanecimiento (duración más prolongada de la estela)

    // Mientras el puntero esté presionado, mantenemos vivo el último punto del trazo activo
    if (isDragging && activeStroke && activeStroke.length > 0) {
      activeStroke[activeStroke.length - 1].time = now;
    }

    // Purgar puntos obsoletos in-place para no romper la referencia de array
    for (let i = laserStrokes.length - 1; i >= 0; i--) {
      const stroke = laserStrokes[i];
      while (stroke.length > 0 && now - stroke[0].time >= maxAge) {
        stroke.shift();
      }
      if (stroke.length === 0 && stroke !== activeStroke) {
        laserStrokes.splice(i, 1);
      }
    }

    for (const stroke of laserStrokes) {
      if (stroke.length === 0) continue;

      if (stroke.length === 1) {
        const p = stroke[0];
        const alpha = Math.max(0, 1 - (now - p.time) / maxAge);
        ctx.beginPath();
        ctx.arc(p.x, p.y, 7, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(255, 30, 60, ${alpha})`;
        ctx.shadowColor = '#ff0033';
        ctx.shadowBlur = 14 * alpha;
        ctx.fill();
        continue;
      }

      // Dibujar líneas curvas suaves con desvanecimiento por segmento
      for (let i = 0; i < stroke.length - 1; i++) {
        const p1 = stroke[i];
        const p2 = stroke[i + 1];
        const age = now - p2.time;
        const alpha = Math.max(0, 1 - age / maxAge);

        ctx.beginPath();
        if (i === 0) {
          ctx.moveTo(p1.x, p1.y);
          ctx.lineTo((p1.x + p2.x) / 2, (p1.y + p2.y) / 2);
        } else {
          const midX = (p1.x + p2.x) / 2;
          const midY = (p1.y + p2.y) / 2;
          const prevMidX = (stroke[i - 1].x + p1.x) / 2;
          const prevMidY = (stroke[i - 1].y + p1.y) / 2;
          ctx.moveTo(prevMidX, prevMidY);
          ctx.quadraticCurveTo(p1.x, p1.y, midX, midY);
        }
        ctx.strokeStyle = `rgba(255, 30, 60, ${alpha})`;
        ctx.lineWidth = 6 * alpha;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.shadowColor = '#ff0033';
        ctx.shadowBlur = 12 * alpha;
        ctx.stroke();
      }

      // Punto resplandeciente en la punta del láser
      const head = stroke[stroke.length - 1];
      const headAge = now - head.time;
      const headAlpha = Math.max(0, 1 - headAge / maxAge);
      ctx.beginPath();
      ctx.arc(head.x, head.y, 6, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255, 255, 255, ${headAlpha})`;
      ctx.shadowColor = '#ff0033';
      ctx.shadowBlur = 18 * headAlpha;
      ctx.fill();
    }

    if (isLaserMode || laserStrokes.length > 0) {
      animFrameId = requestAnimationFrame(drawLaserLoop);
    } else {
      animFrameId = null;
    }
  }

  $effect(() => {
    if (isLaserMode) {
      if (!animFrameId) {
        animFrameId = requestAnimationFrame(drawLaserLoop);
      }
    } else {
      laserStrokes = [];
      activeStroke = null;
    }
  });

  onDestroy(() => {
    if (animFrameId) cancelAnimationFrame(animFrameId);
  });

  function updateCanvasSize() {
    if (containerRef && canvasRef) {
      const rect = containerRef.getBoundingClientRect();
      if (canvasRef.width !== rect.width || canvasRef.height !== rect.height) {
        canvasRef.width = rect.width;
        canvasRef.height = rect.height;
      }
    }
  }

  function addLaserPoint(clientX: number, clientY: number) {
    if (!containerRef || !activeStroke) return;
    updateCanvasSize();
    const rect = containerRef.getBoundingClientRect();
    activeStroke.push({
      x: clientX - rect.left,
      y: clientY - rect.top,
      time: Date.now(),
    });
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomSensitivity = 0.002;
    const delta = -e.deltaY * zoomSensitivity;
    scale = Math.max(0.1, Math.min(scale * Math.exp(delta), 10));
  }

  function handlePointerDown(e: PointerEvent) {
    isDragging = true;
    if (isLaserMode) {
      activeStroke = [];
      laserStrokes.push(activeStroke);
      addLaserPoint(e.clientX, e.clientY);
    } else {
      startX = e.clientX - panX;
      startY = e.clientY - panY;
    }
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (isDragging) {
      if (isLaserMode) {
        addLaserPoint(e.clientX, e.clientY);
      } else {
        panX = e.clientX - startX;
        panY = e.clientY - startY;
      }
    }
  }

  function handlePointerUp(e: PointerEvent) {
    isDragging = false;
    activeStroke = null;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="merman-container">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div 
    class="preview-pane"
    class:laser-cursor={isLaserMode}
    bind:this={containerRef}
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
      <div class="svg-container">
        {@html svgContent}
      </div>
    {/if}

    <!-- Capa de lienzo transparente para el Puntero Láser -->
    <canvas bind:this={canvasRef} class="laser-canvas"></canvas>
  </div>
  
  <div class="zoom-controls">
    <button 
      class="laser-toggle-btn" 
      class:active={isLaserMode} 
      onclick={() => (isLaserMode = !isLaserMode)} 
      title="Activar / Desactivar Puntero Láser"
    >
      <span class="laser-dot"></span>
      Láser
    </button>
    <div class="separator"></div>
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
    position: relative;
  }
  
  .preview-pane:active {
    cursor: grabbing;
  }

  .preview-pane.laser-cursor,
  .preview-pane.laser-cursor:active {
    cursor: crosshair;
  }

  .laser-canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 5;
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

  .laser-toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px !important;
    border-radius: 6px !important;
    font-size: 13px !important;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .laser-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: #888;
    transition: background-color 0.2s ease;
  }

  .laser-toggle-btn.active {
    background-color: rgba(255, 30, 60, 0.15) !important;
    color: #e0002b !important;
    border: 1px solid rgba(255, 30, 60, 0.3);
  }

  .laser-toggle-btn.active .laser-dot {
    background-color: #ff0033;
    box-shadow: 0 0 8px #ff0033;
  }

  .separator {
    width: 1px;
    height: 16px;
    background-color: var(--border-primary, #d0d7de);
    margin: 0 4px;
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
