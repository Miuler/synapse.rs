<script lang="ts">
  // Control de zoom de un visor: −, porcentaje (clic = 100 %), + y, opcional, «Ajustar al ancho».
  import { Minus, Plus, MoveHorizontal } from 'lucide-svelte';
  import { stepViewerZoom } from '@shared/lib/viewer-zoom';

  interface Props {
    zoom: number;
    onChange: (zoom: number) => void;
    /** Si se pasa, muestra «Ajustar al ancho» */
    onFit?: () => void;
  }
  let { zoom, onChange, onFit }: Props = $props();
</script>

<div class="zoom-control" role="group" aria-label="Zoom">
  <button type="button" class="zoom-btn zoom-out" title="Reducir (Ctrl+−, Ctrl+rueda)" onclick={() => onChange(stepViewerZoom(zoom, -1))}>
    <Minus size={13} />
  </button>
  <button type="button" class="zoom-value" title="Volver al 100 % (Ctrl+0)" onclick={() => onChange(1)}>
    {Math.round(zoom * 100)} %
  </button>
  <button type="button" class="zoom-btn zoom-in" title="Ampliar (Ctrl++, Ctrl+rueda)" onclick={() => onChange(stepViewerZoom(zoom, 1))}>
    <Plus size={13} />
  </button>
  {#if onFit}
    <button type="button" class="zoom-btn zoom-fit" title="Ajustar al ancho" onclick={onFit}>
      <MoveHorizontal size={13} />
    </button>
  {/if}
</div>

<style>
  .zoom-control {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .zoom-btn,
  .zoom-value {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 22px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
  }
  .zoom-btn {
    width: 24px;
  }
  .zoom-value {
    min-width: 46px;
    padding: 0 4px;
    border-left: 1px solid var(--border-primary);
    border-right: 1px solid var(--border-primary);
    font-variant-numeric: tabular-nums;
  }
  .zoom-fit {
    border-left: 1px solid var(--border-primary);
  }
  .zoom-btn:hover,
  .zoom-value:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
