<script lang="ts">
  import { initMerman, renderSvg, validate } from '@mermanjs/web';
  import { onMount, onDestroy, tick } from 'svelte';
  import { CodeEditor } from '@shared/ui/code-editor';

  interface Props {
    content: string;
    readOnly?: boolean;
    vimMode?: boolean;
    onChange?: (content: string) => void;
    onSelectionChange?: (info: any) => void;
  }

  let { content = '', readOnly = false, vimMode = false, onChange, onSelectionChange }: Props = $props();

  let svgContent = $state('');
  let isReady = $state(false);
  let error = $state('');
  let codeEditorRef = $state<any>(null);
  
  let scale = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isDragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);

  // Modo Puntero Láser
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
        const valRes = validate(content);
        if (!valRes.valid) {
          error = valRes.error || `Error de sintaxis (${valRes.code_name || 'MERMAN_PARSE_ERROR'})`;
        } else {
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
        }
      } catch (e: any) {
        error = String(e?.message || e);
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
    const maxAge = 1600;

    if (isDragging && activeStroke && activeStroke.length > 0) {
      activeStroke[activeStroke.length - 1].time = now;
    }

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

<div class="merman-container" class:split-mode={!readOnly}>
  {#if !readOnly}
    <div class="editor-pane">
      <div class="editor-header-bar">
        <span>Código Mermaid</span>
        <div class="editor-header-actions">
          {#if error}
            <span class="syntax-error-badge">Error de sintaxis</span>
          {/if}
          <button
            type="button"
            class="editor-header-btn"
            onclick={() => { if (codeEditorRef) codeEditorRef.triggerSearch(); }}
            title="Buscar y Reemplazar (Ctrl+F / Ctrl+H)"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"/>
              <line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <span>Buscar</span>
          </button>

          <button
            type="button"
            class="editor-save-btn"
            onclick={() => { if (onChange) onChange(content); }}
            title="Guardar / Grabar cambios"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
              <polyline points="17 21 17 13 7 13 7 21"/>
              <polyline points="7 3 7 8 15 8"/>
            </svg>
            <span>Grabar</span>
          </button>
        </div>
      </div>

      <div class="editor-body">
        <CodeEditor
          bind:this={codeEditorRef}
          {content}
          {readOnly}
          {vimMode}
          mode="mermaid"
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
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <pre>{error}</pre>
        </div>
      {/if}
    </div>
  {/if}

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
      <div class="error-msg">
        <div class="error-title">Error al renderizar diagrama Mermaid</div>
        <pre>{error}</pre>
      </div>
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
    min-width: 280px;
    height: 100%;
    border-right: 1px solid var(--border-primary, #d0d7de);
    background: var(--bg-secondary, #f6f8fa);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
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
    background: rgba(0, 0, 0, 0.05);
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
    color: #ffffff;
  }

  .syntax-error-badge {
    font-size: 10px;
    font-weight: 600;
    color: #cf222e;
    background: #ffebe9;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(207, 34, 46, 0.3);
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
    background: #ffebe9;
    border-top: 1px solid rgba(207, 34, 46, 0.3);
    color: #cf222e;
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

  .editor-error-footer svg {
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
    padding: 14px;
    border-radius: 8px;
    font-family: var(--code-font, monospace);
    font-size: 13px;
    width: 85%;
    border: 1px solid rgba(207, 34, 46, 0.3);
    z-index: 10;
    box-shadow: 0 4px 12px rgba(207, 34, 46, 0.1);
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
