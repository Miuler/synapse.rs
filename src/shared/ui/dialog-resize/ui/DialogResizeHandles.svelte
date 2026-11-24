<!-- Bordes y esquinas para cambiar el tamaño de un diálogo; el padre debe tener posición fija o relativa. -->
<script lang="ts">
  import type { ResizeEdge } from '../model/types';
  interface Props {
    onStart: (e: PointerEvent, dx: -1 | 0 | 1, dy: -1 | 0 | 1) => void;
    edges?: ResizeEdge[];
    showCornerGrip?: boolean;
  }

  let { onStart, edges, showCornerGrip = false }: Props = $props();

  const HANDLES: { cls: ResizeEdge; dx: -1 | 0 | 1; dy: -1 | 0 | 1 }[] = [
    { cls: 'n', dx: 0, dy: -1 },
    { cls: 's', dx: 0, dy: 1 },
    { cls: 'e', dx: 1, dy: 0 },
    { cls: 'w', dx: -1, dy: 0 },
    { cls: 'ne', dx: 1, dy: -1 },
    { cls: 'nw', dx: -1, dy: -1 },
    { cls: 'se', dx: 1, dy: 1 },
    { cls: 'sw', dx: -1, dy: 1 },
  ];

  let visibleHandles = $derived(edges ? HANDLES.filter((h) => edges.includes(h.cls)) : HANDLES);
</script>

{#each visibleHandles as h (h.cls)}
  <div class="dialog-resize {h.cls}" aria-hidden="true" onpointerdown={(e) => onStart(e, h.dx, h.dy)}>
    {#if showCornerGrip && h.cls === 'se'}
      <svg class="corner-grip-icon" viewBox="0 0 10 10" width="10" height="10">
        <path d="M9 1L1 9 M9 5L5 9 M9 9L9 9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      </svg>
    {/if}
  </div>
{/each}

<style>
  .dialog-resize {
    position: absolute;
    z-index: 20;
  }
  .n,
  .s {
    left: 10px;
    right: 10px;
    height: 8px;
    cursor: ns-resize;
  }
  .n { top: 0; }
  .s { bottom: 0; }
  .e,
  .w {
    top: 10px;
    bottom: 10px;
    width: 8px;
    cursor: ew-resize;
  }
  .e { right: 0; }
  .w { left: 0; }
  .ne,
  .nw,
  .se,
  .sw {
    width: 16px;
    height: 16px;
  }
  .ne { top: 0; right: 0; cursor: nesw-resize; }
  .sw { bottom: 0; left: 0; cursor: nesw-resize; }
  .nw { top: 0; left: 0; cursor: nwse-resize; }
  .se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 3px;
  }
  .corner-grip-icon {
    color: var(--text-secondary, #656d76);
    opacity: 0.45;
    transition: opacity 0.15s ease;
    pointer-events: none;
  }
  .se:hover .corner-grip-icon {
    opacity: 0.9;
  }
</style>
