<!-- Bordes y esquinas para cambiar el tamaño de un diálogo; el padre debe tener posición fija o relativa. -->
<script lang="ts">
  interface Props {
    onStart: (e: PointerEvent, dx: -1 | 0 | 1, dy: -1 | 0 | 1) => void;
  }

  let { onStart }: Props = $props();

  const HANDLES: { cls: string; dx: -1 | 0 | 1; dy: -1 | 0 | 1 }[] = [
    { cls: 'n', dx: 0, dy: -1 },
    { cls: 's', dx: 0, dy: 1 },
    { cls: 'e', dx: 1, dy: 0 },
    { cls: 'w', dx: -1, dy: 0 },
    { cls: 'ne', dx: 1, dy: -1 },
    { cls: 'nw', dx: -1, dy: -1 },
    { cls: 'se', dx: 1, dy: 1 },
    { cls: 'sw', dx: -1, dy: 1 },
  ];
</script>

{#each HANDLES as h (h.cls)}
  <div class="dialog-resize {h.cls}" aria-hidden="true" onpointerdown={(e) => onStart(e, h.dx, h.dy)}></div>
{/each}

<style>
  .dialog-resize {
    position: absolute;
    z-index: 5;
  }
  .n,
  .s {
    left: 8px;
    right: 8px;
    height: 6px;
    cursor: ns-resize;
  }
  .n { top: 0; }
  .s { bottom: 0; }
  .e,
  .w {
    top: 8px;
    bottom: 8px;
    width: 6px;
    cursor: ew-resize;
  }
  .e { right: 0; }
  .w { left: 0; }
  .ne,
  .nw,
  .se,
  .sw {
    width: 12px;
    height: 12px;
  }
  .ne { top: 0; right: 0; cursor: nesw-resize; }
  .sw { bottom: 0; left: 0; cursor: nesw-resize; }
  .nw { top: 0; left: 0; cursor: nwse-resize; }
  .se { bottom: 0; right: 0; cursor: nwse-resize; }
</style>
