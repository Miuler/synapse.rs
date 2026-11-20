<script lang="ts">
  // Selector pequeño del zoom del contenido (texto del editor, Markdown y OpenAPI). Se abre desde la
  // paleta (Ctrl+P → «Zoom»); ↑/↓ mueven, Enter aplica, Esc cierra. Al moverse ya se ve el cambio.
  import { appSettings } from '@entities/settings';
  import { tick, untrack } from 'svelte';

  interface Props {
    open: boolean;
  }
  let { open = $bindable(false) }: Props = $props();

  const ZOOM_LEVELS = [0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2];

  let list = $state<HTMLUListElement | null>(null);
  let index = $state(3);
  let original = 1;

  function nearest(z: number): number {
    let best = 0;
    for (let i = 1; i < ZOOM_LEVELS.length; i++) if (Math.abs(ZOOM_LEVELS[i] - z) < Math.abs(ZOOM_LEVELS[best] - z)) best = i;
    return best;
  }

  $effect(() => {
    if (!open) return;
    original = untrack(() => appSettings.contentZoom);
    index = nearest(original);
    void tick().then(() => list?.focus());
  });

  function preview(i: number) {
    index = i;
    appSettings.setContentZoom(ZOOM_LEVELS[i]);
    list?.querySelector<HTMLElement>(`[data-index="${i}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function accept(i = index) {
    appSettings.setContentZoom(ZOOM_LEVELS[i]);
    open = false;
  }

  function cancel() {
    appSettings.setContentZoom(original);
    open = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') preview(Math.min(ZOOM_LEVELS.length - 1, index + 1));
    else if (e.key === 'ArrowUp') preview(Math.max(0, index - 1));
    else if (e.key === 'Home') preview(0);
    else if (e.key === 'End') preview(ZOOM_LEVELS.length - 1);
    else if (e.key === 'Enter') accept();
    else if (e.key === 'Escape') cancel();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="zoom-backdrop" onclick={cancel}>
    <div class="zoom-dialog" role="dialog" aria-modal="true" aria-labelledby="zoom-dialog-title" onclick={(e) => e.stopPropagation()}>
      <div class="zoom-title" id="zoom-dialog-title">Zoom</div>
      <ul
        bind:this={list}
        class="zoom-list"
        role="listbox"
        tabindex="0"
        aria-labelledby="zoom-dialog-title"
        aria-activedescendant={`zoom-opt-${index}`}
        onkeydown={onKeydown}
      >
        {#each ZOOM_LEVELS as z, i}
          <li
            id={`zoom-opt-${i}`}
            data-index={i}
            role="option"
            aria-selected={i === index}
            class:active={i === index}
            onmouseenter={() => (index = i)}
            onclick={() => accept(i)}
          >
            {Math.round(z * 100)}%
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .zoom-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
    background: rgba(0, 0, 0, 0.25);
  }
  .zoom-dialog {
    width: 120px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }
  .zoom-title {
    padding: 8px 10px;
    font-size: 13px;
    font-weight: 600;
    text-align: center;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-primary);
  }
  .zoom-list {
    list-style: none;
    margin: 0;
    padding: 4px;
    outline: none;
    max-height: 60vh;
    overflow: auto;
  }
  .zoom-list li {
    padding: 3px 10px;
    font-size: 13px;
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
  }
  .zoom-list li.active {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
