<script lang="ts">
  // Lienzo transparente del puntero láser: ocupa a su contenedor (con `position: relative`) y dibuja
  // trazos que se desvanecen. El visor le pasa los puntos; él no recibe el ratón.
  import { cssVar, onThemeChange, withAlpha } from '@shared/lib/theme-colors';
  import { onDestroy } from 'svelte';

  type LaserPoint = { x: number; y: number; time: number };
  type LaserStroke = LaserPoint[];

  let { active }: { active: boolean } = $props();

  let canvasRef = $state<HTMLCanvasElement | null>(null);
  let laserStrokes: LaserStroke[] = [];
  let activeStroke: LaserStroke | null = null;
  let animFrameId: number | null = null;

  // Color del láser del tema (`--laser`); se relee al cambiar de tema, no en cada frame
  let laserColor = cssVar('--laser', '#ff0033');
  const stopTheme = onThemeChange(() => (laserColor = cssVar('--laser', '#ff0033')));

  /** Empieza un trazo (coordenadas de la ventana). */
  export function start(clientX: number, clientY: number) {
    activeStroke = [];
    laserStrokes.push(activeStroke);
    move(clientX, clientY);
  }

  export function move(clientX: number, clientY: number) {
    if (!canvasRef || !activeStroke) return;
    const rect = canvasRef.getBoundingClientRect();
    const [w, h] = [Math.round(rect.width), Math.round(rect.height)];
    if (canvasRef.width !== w || canvasRef.height !== h) {
      canvasRef.width = w;
      canvasRef.height = h;
    }
    activeStroke.push({ x: clientX - rect.left, y: clientY - rect.top, time: Date.now() });
    animFrameId ??= requestAnimationFrame(draw);
  }

  export function end() {
    activeStroke = null;
  }

  function draw() {
    animFrameId = null;
    const ctx = canvasRef?.getContext('2d');
    if (!canvasRef || !ctx) return;

    ctx.clearRect(0, 0, canvasRef.width, canvasRef.height);

    const now = Date.now();
    const maxAge = 1600;

    // Mientras se mantiene pulsado, la punta no se apaga
    if (activeStroke?.length) activeStroke[activeStroke.length - 1].time = now;

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
        ctx.fillStyle = withAlpha(laserColor, alpha);
        ctx.shadowColor = laserColor;
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
        ctx.strokeStyle = withAlpha(laserColor, alpha);
        ctx.lineWidth = 6 * alpha;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.shadowColor = laserColor;
        ctx.shadowBlur = 12 * alpha;
        ctx.stroke();
      }

      const head = stroke[stroke.length - 1];
      const headAge = now - head.time;
      const headAlpha = Math.max(0, 1 - headAge / maxAge);
      ctx.beginPath();
      ctx.arc(head.x, head.y, 6, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255, 255, 255, ${headAlpha})`; /* fijo: el centro del láser es blanco, como un puntero real */
      ctx.shadowColor = laserColor;
      ctx.shadowBlur = 18 * headAlpha;
      ctx.fill();
    }

    if (laserStrokes.length > 0) animFrameId = requestAnimationFrame(draw);
  }

  $effect(() => {
    if (active) return;
    laserStrokes = [];
    activeStroke = null;
    // Borra lo que quedara dibujado
    animFrameId ??= requestAnimationFrame(draw);
  });

  onDestroy(() => {
    stopTheme();
    if (animFrameId) cancelAnimationFrame(animFrameId);
  });
</script>

<canvas bind:this={canvasRef} class="laser-canvas" aria-hidden="true"></canvas>

<style>
  .laser-canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 5;
  }
</style>
