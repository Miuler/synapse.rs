<script lang="ts">
  interface Props {
    /** Contenedor flex cuyo ancho se reparte; la barra se coloca entre los dos paneles hijos */
    container: HTMLElement | null;
    /** Límites del primer panel, en porcentaje del ancho del contenedor */
    min?: number;
    max?: number;
    label?: string;
    /** Se invoca mientras se arrastra, con el porcentaje del primer panel */
    onResize?: (percent: number) => void;
    /** Se invoca al soltar la barra, con el porcentaje final */
    onResizeEnd?: (percent: number) => void;
    /** Indica si hay un arrastre en curso */
    active?: boolean;
  }

  let { container, min = 15, max = 85, label = 'Redimensionar paneles', onResize, onResizeEnd, active = $bindable(false) }: Props = $props();

  let lastPercent: number | null = null;

  function handlePointerDown(e: PointerEvent) {
    e.preventDefault();
    active = true;
    lastPercent = null;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!active || !container) return;
    const rect = container.getBoundingClientRect();
    if (rect.width <= 0) return;
    const percent = Math.max(min, Math.min(max, ((e.clientX - rect.left) / rect.width) * 100));
    lastPercent = percent;
    onResize?.(percent);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!active) return;
    active = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    if (lastPercent !== null) onResizeEnd?.(lastPercent);
  }
</script>

<div
  class="split-resizer"
  class:active
  role="separator"
  aria-orientation="vertical"
  aria-label={label}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
></div>

<style>
  .split-resizer {
    width: 5px;
    margin-left: -3px;
    margin-right: -2px;
    flex-shrink: 0;
    cursor: col-resize;
    background: transparent;
    position: relative;
    z-index: 6;
    touch-action: none;
    transition: background 0.15s ease;
  }

  .split-resizer:hover,
  .split-resizer.active {
    background: var(--accent, #0969da);
  }
</style>
