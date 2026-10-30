<script lang="ts">
  import { parseShortcut } from '@shared/lib/shortcuts';

  interface Props {
    shortcut: string;
    /** Muestra solo la combinación principal (por defecto) o todas las alternativas separadas por "o" */
    all?: boolean;
  }

  let { shortcut, all = false }: Props = $props();

  let alternatives = $derived(all ? parseShortcut(shortcut) : parseShortcut(shortcut).slice(0, 1));
</script>

<span class="kbd-group">
  {#each alternatives as keys, i}
    {#if i > 0}<span class="kbd-or">o</span>{/if}
    <span class="kbd-combo">
      {#each keys as key}<kbd class="kbd-key">{key}</kbd>{/each}
    </span>
  {/each}
</span>

<style>
  .kbd-group {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .kbd-combo {
    display: inline-flex;
    gap: 3px;
  }

  .kbd-key {
    font-family: var(--mono, ui-monospace, monospace);
    font-size: 11px;
    line-height: 1;
    min-width: 20px;
    text-align: center;
    padding: 3px 6px;
    border-radius: 4px;
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-primary, #1f2328);
    border: 1px solid var(--border-primary, #d0d7de);
    border-bottom-width: 2px;
  }

  .kbd-or {
    font-size: 11px;
    color: var(--text-secondary, #656d76);
  }
</style>
