<script lang="ts">
  // Elegir a qué archivo lleva un enlace ambiguo (varios archivos con el mismo nombre).
  // ↑/↓ mueven · Enter abre (solo esta vez) · Ctrl+Enter abre y fija el enlace en la nota · Esc cierra.
  // Ratón: clic abre · «Fijar» o Ctrl+clic abre y fija.
  import { tick } from 'svelte';
  import { Pin, FileText } from 'lucide-svelte';

  interface Props {
    /** Destino tal como está escrito en el enlace */
    target: string;
    candidates: string[];
    /** Se puede corregir el enlace en su nota (no en el chat del agente ni en una nota bloqueada) */
    canFix: boolean;
    onPick: (path: string, fix: boolean) => void;
    onClose: () => void;
  }
  let { target, candidates, canFix, onPick, onClose }: Props = $props();

  let index = $state(0);
  let list = $state<HTMLDivElement | null>(null);

  $effect(() => {
    void candidates;
    index = 0;
    tick().then(() => list?.focus());
  });

  function pick(i: number, fix: boolean) {
    const path = candidates[i];
    if (path) onPick(path, fix && canFix);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const d = e.key === 'ArrowDown' ? 1 : -1;
      index = (index + d + candidates.length) % candidates.length;
      list?.querySelectorAll('[role="option"]')[index]?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      pick(index, e.ctrlKey || e.metaKey);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  const folder = (p: string) => (p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '(raíz de la bóveda)');
  const name = (p: string) => p.split('/').pop() ?? p;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="link-chooser-backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="link-chooser" role="dialog" aria-label="Elegir archivo del enlace" onclick={(e) => e.stopPropagation()}>
    <div class="link-chooser-head">
      Hay {candidates.length} archivos llamados <strong>{name(target)}</strong>. ¿A cuál lleva el enlace?
    </div>
    <div class="link-chooser-list" role="listbox" tabindex="-1" bind:this={list} onkeydown={onKeydown}>
      {#each candidates as path, i (path)}
        <div
          role="option"
          tabindex="-1"
          aria-selected={i === index}
          class="link-chooser-item"
          class:active={i === index}
          data-path={path}
          onmouseenter={() => (index = i)}
          onclick={(e) => pick(i, e.ctrlKey || e.metaKey)}
        >
          <FileText size={14} />
          <span class="link-chooser-path"><span class="name">{name(path)}</span><span class="folder">{folder(path)}</span></span>
          {#if canFix}
            <button
              type="button"
              class="link-chooser-pin"
              title="Abrir y fijar: el enlace de la nota pasa a llevar la ruta completa (Ctrl+Enter)"
              onclick={(e) => {
                e.stopPropagation();
                pick(i, true);
              }}
            >
              <Pin size={12} /> Fijar
            </button>
          {/if}
        </div>
      {/each}
    </div>
    <div class="link-chooser-hint">
      {#if canFix}Enter abre · Ctrl+Enter o <Pin size={10} /> Fijar: abre y corrige el enlace en la nota{:else}Enter abre{/if}
    </div>
  </div>
</div>

<style>
  .link-chooser-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(0, 0, 0, 0.25);
  }
  .link-chooser {
    width: min(560px, 92vw);
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    overflow: hidden;
    font-size: 13px;
  }
  .link-chooser-head {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-primary);
  }
  .link-chooser-list {
    overflow: auto;
    outline: none;
    padding: 4px;
  }
  .link-chooser-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-secondary);
  }
  .link-chooser-item.active {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .link-chooser-path {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .link-chooser-path .name {
    color: var(--text-primary);
  }
  .link-chooser-path .folder {
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link-chooser-pin {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 2px 8px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    background: var(--bg-primary);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    visibility: hidden;
  }
  .link-chooser-item.active .link-chooser-pin {
    visibility: visible;
  }
  .link-chooser-pin:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .link-chooser-hint {
    padding: 6px 12px;
    border-top: 1px solid var(--border-primary);
    color: var(--text-secondary);
    font-size: 11px;
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
