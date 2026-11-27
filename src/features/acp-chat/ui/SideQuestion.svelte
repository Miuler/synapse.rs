<script lang="ts">
  // Ventana flotante de las preguntas aparte (`/btw`) de una conversación: se mueve por su cabecera y
  // cambia de tamaño desde la esquina. El chat de dentro es el mismo del panel (llega en `children`).
  import type { Conversation } from '@entities/acp-agent';
  import { t } from '@shared/lib/i18n';
  import { GitFork, Loader2, Minus, Trash2 } from 'lucide-svelte';
  import { onMount, type Snippet } from 'svelte';
  import { loadWindow, placeWindow, saveWindow, type WindowBox } from '../lib/aside-window';

  interface Props {
    /** Las preguntas aparte de la conversación y la que está a la vista. */
    asides: Conversation[];
    side: Conversation;
    onSelect: (id: string) => void;
    onMinimize: () => void;
    onRemove: (id: string) => void;
    onPromote: (id: string) => void;
    children: Snippet;
  }

  let { asides, side, onSelect, onMinimize, onRemove, onPromote, children }: Props = $props();

  const working = $derived(side.status === 'working');
  const cost = $derived(side.usage?.cost);

  let el = $state<HTMLElement | null>(null);
  /** Lugar y tamaño; `null` hasta saber dónde está el chat del que sale. */
  let box = $state<WindowBox | null>(null);
  const viewport = () => ({ w: window.innerWidth, h: window.innerHeight });
  const chatRect = () => el?.parentElement?.getBoundingClientRect() ?? null;

  export function focus() {
    el?.querySelector('textarea')?.focus();
  }

  const label = (c: Conversation) => {
    const first = c.items.find((i) => i.type === 'user');
    return (first?.type === 'user' ? first.text : c.draft) || t('aside.title');
  };

  onMount(() => {
    box = placeWindow(loadWindow(), chatRect(), viewport());
    // El tamaño que se cambie desde la esquina se recuerda
    const resized = new ResizeObserver(() => {
      // Con el panel oculto mide 0: no es un tamaño elegido
      if (!el?.offsetWidth || !box || (el.offsetWidth === box.w && el.offsetHeight === box.h)) return;
      box = { ...box, w: el.offsetWidth, h: el.offsetHeight };
      saveWindow(box);
    });
    resized.observe(el!);
    const fit = () => box && (box = placeWindow(box, chatRect(), viewport()));
    window.addEventListener('resize', fit);
    return () => {
      resized.disconnect();
      window.removeEventListener('resize', fit);
    };
  });

  function startDrag(e: PointerEvent) {
    if (e.button !== 0 || !box || (e.target as HTMLElement).closest('button')) return;
    e.preventDefault();
    const x0 = e.clientX - box.x;
    const y0 = e.clientY - box.y;
    const move = (ev: PointerEvent) => (box = placeWindow({ ...box!, x: ev.clientX - x0, y: ev.clientY - y0 }, null, viewport()));
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      if (box) saveWindow(box);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="acp-side"
  role="dialog"
  aria-label={t('aside.title')}
  tabindex="-1"
  data-aside={side.id}
  bind:this={el}
  style={box ? `left: ${box.x}px; top: ${box.y}px; width: ${box.w}px; height: ${box.h}px;` : 'visibility: hidden;'}
  onkeydown={(e) => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    onMinimize();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="acp-side-head" title={t('aside.drag')} onpointerdown={startDrag}>
    {#if asides.length > 1}
      <div class="acp-side-tabs" role="tablist">
        {#each asides as a (a.id)}
          <button type="button" role="tab" class="acp-side-tab" aria-selected={a === side} title={label(a)} onclick={() => onSelect(a.id)}>
            {#if a.status === 'working'}<Loader2 size={10} class="acp-spin" />{:else if a.unseen && a !== side}<span class="acp-side-unread" title={t('aside.unread')}></span>{/if}
            <span>{label(a)}</span>
          </button>
        {/each}
      </div>
    {:else}
      <span class="acp-side-title">{t('aside.title')}</span>
    {/if}
    {#if cost}<span class="acp-side-cost">{cost.amount} {cost.currency}</span>{/if}
    <button
      type="button"
      class="acp-side-promote"
      title={t('aside.promoteHint')}
      disabled={working || !side.lastText('agent')}
      onclick={() => onPromote(side.id)}><GitFork size={11} /> {t('aside.promote')}</button
    >
    <button type="button" class="acp-side-minimize" aria-label={t('aside.minimize')} title={t('aside.minimize')} onclick={onMinimize}>
      <Minus size={12} />
    </button>
    <button type="button" class="acp-side-remove" aria-label={t('aside.remove')} title={t('aside.remove')} onclick={() => onRemove(side.id)}>
      <Trash2 size={12} />
    </button>
  </div>
  {@render children()}
</div>

<style>
  .acp-side {
    position: fixed;
    z-index: 950;
    display: flex;
    flex-direction: column;
    min-width: 300px;
    min-height: 260px;
    max-width: 100vw;
    max-height: 100vh;
    resize: both;
    overflow: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--accent);
    border-radius: 8px;
    box-shadow: var(--modal-shadow);
    font-size: 13px;
  }
  .acp-side:focus {
    outline: none;
  }
  .acp-side-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    font-size: 11px;
    color: var(--text-muted);
    cursor: move;
    user-select: none;
    touch-action: none;
  }
  .acp-side-title {
    flex: 1;
  }
  .acp-side-tabs {
    flex: 1;
    display: flex;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .acp-side-tabs::-webkit-scrollbar {
    display: none;
  }
  /* Se estrechan con puntos suspensivos antes de tener que desplazarlas */
  .acp-side-head .acp-side-tab {
    flex: 0 1 140px;
    min-width: 56px;
  }
  .acp-side-tab span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acp-side-head .acp-side-tab[aria-selected='true'] {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
  .acp-side-unread {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .acp-side-head button {
    display: inline-flex;
    white-space: nowrap;
    align-items: center;
    gap: 3px;
    padding: 2px 6px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
  }
  .acp-side-head button:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .acp-side-head button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
