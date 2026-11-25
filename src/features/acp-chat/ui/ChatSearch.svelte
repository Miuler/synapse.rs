<script lang="ts">
  // Barra de búsqueda de una conversación (Ctrl+F)
  import { t } from '@shared/lib/i18n';
  import { ChevronDown, ChevronUp, Search, Wrench, X } from 'lucide-svelte';
  import { tick, untrack } from 'svelte';
  import { paintMatches, revealRange, searchRanges } from '../lib/chat-search';

  interface Props {
    scroller: HTMLElement;
    /** Cambia cada vez que se pide buscar: vuelve a enfocar la caja. */
    request: number;
    onClose: () => void;
  }
  let { scroller, request, onClose }: Props = $props();

  let query = $state('');
  /** Buscar también en la entrada y el resultado de las herramientas; se recuerda. */
  const TOOLS_KEY = 'synapse.agent.search-tools';
  let tools = $state(load());
  function load() {
    try {
      return localStorage.getItem(TOOLS_KEY) === '1';
    } catch {
      return false;
    }
  }
  function toggleTools() {
    tools = !tools;
    try {
      localStorage.setItem(TOOLS_KEY, tools ? '1' : '0');
    } catch {
      /* sin almacenamiento */
    }
    run(true);
    input?.focus();
  }
  let input = $state<HTMLInputElement | null>(null);
  let matches = $state.raw<Range[]>([]);
  let index = $state(0);
  /** Pliegues abiertos para mostrar una coincidencia: se vuelven a cerrar al pasar a otra. */
  let opened: HTMLDetailsElement[] = [];

  $effect(() => {
    void request;
    tick().then(() => {
      input?.focus();
      input?.select();
    });
  });

  function show(i: number) {
    index = i;
    const range = matches[i] ?? null;
    const keep: HTMLDetailsElement[] = [];
    if (range) {
      // Razonamientos y herramientas plegados: se abre el que contiene la coincidencia
      const inSummary = range.startContainer.parentElement?.closest('summary');
      let d = (inSummary?.parentElement?.parentElement ?? range.startContainer.parentElement)?.closest('details') ?? null;
      while (d && scroller.contains(d)) {
        if (!d.open) {
          d.open = true;
          keep.push(d);
        } else if (opened.includes(d)) keep.push(d);
        d = d.parentElement?.closest('details') ?? null;
      }
    }
    for (const d of opened) if (!keep.includes(d)) d.open = false;
    opened = keep;
    paintMatches(matches, range);
    if (range) revealRange(scroller, range);
  }

  function step(by: number) {
    if (timer) run(true);
    else if (matches.length) show((index + by + matches.length) % matches.length);
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  /** `fresh`: búsqueda nueva, empieza por la más reciente; si no, se conserva la posición. */
  function run(fresh: boolean) {
    clearTimeout(timer);
    timer = undefined;
    matches = searchRanges(scroller, query, tools);
    if (fresh) return show(Math.max(0, matches.length - 1));
    index = Math.min(index, Math.max(0, matches.length - 1));
    paintMatches(matches, matches[index] ?? null);
  }

  $effect(() => {
    void query;
    untrack(() => {
      clearTimeout(timer);
      timer = setTimeout(() => run(true), 120);
    });
  });

  // Mensajes nuevos o que siguen llegando mientras se busca
  $effect(() => {
    let wait: ReturnType<typeof setTimeout> | undefined;
    const observer = new MutationObserver(() => {
      if (wait || !query || timer) return;
      wait = setTimeout(() => {
        wait = undefined;
        if (!timer) run(false);
      }, 400);
    });
    observer.observe(scroller, { childList: true, characterData: true, subtree: true });
    return () => {
      observer.disconnect();
      clearTimeout(wait);
      clearTimeout(timer);
      paintMatches([], null);
    };
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      // Cierra la búsqueda, no el panel
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (e.isComposing) return;
      e.preventDefault();
      step(e.key === 'ArrowUp' || (e.key === 'Enter' && e.shiftKey) ? -1 : 1);
    }
  }
</script>

<div class="acp-search" role="search">
  <Search size={13} />
  <input
    bind:this={input}
    bind:value={query}
    class="acp-search-input"
    type="text"
    placeholder={t('chatSearch.placeholder')}
    aria-label={t('chatSearch.placeholder')}
    onkeydown={onKeydown}
  />
  <span class="acp-search-count" aria-live="polite">
    {#if query.trim()}{matches.length ? `${index + 1}/${matches.length}` : t('chatSearch.none')}{/if}
  </span>
  <button
    type="button"
    class="acp-search-tools"
    class:active={tools}
    aria-pressed={tools}
    title={t('chatSearch.tools')}
    aria-label={t('chatSearch.tools')}
    onclick={toggleTools}><Wrench size={13} /></button
  >
  <button type="button" title={t('chatSearch.previous')} aria-label={t('chatSearch.previous')} disabled={!matches.length} onclick={() => step(-1)}
    ><ChevronUp size={14} /></button
  >
  <button type="button" title={t('chatSearch.next')} aria-label={t('chatSearch.next')} disabled={!matches.length} onclick={() => step(1)}
    ><ChevronDown size={14} /></button
  >
  <button type="button" title={t('chatSearch.close')} aria-label={t('chatSearch.close')} onclick={onClose}><X size={14} /></button>
</div>

<style>
  .acp-search {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-muted);
  }
  .acp-search-input {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    font: inherit;
    font-size: 12px;
    color: var(--text-primary);
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 4px;
    outline: none;
  }
  .acp-search-input:focus {
    border-color: var(--accent);
  }
  .acp-search-count {
    font-size: 11px;
    white-space: nowrap;
  }
  .acp-search button {
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
  }
  .acp-search button.active {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .acp-search button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .acp-search button:hover:not(:disabled) {
    background: var(--bg-primary);
  }
  :global(::highlight(synapse-chat-search)) {
    background-color: rgba(255, 223, 93, 0.45); /* fijo: resaltado de búsqueda, no depende del tema */
  }
  :global(::highlight(synapse-chat-search-active)) {
    background-color: #ff9632; /* fijo: resaltado de búsqueda, no depende del tema */
    color: #000000; /* fijo: texto sobre resaltado naranja */
  }
</style>
