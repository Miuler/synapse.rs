<script lang="ts">
  // Barra de búsqueda (Ctrl+F): la caja, el contador y los botones. Quién busca y qué resalta, aparte.
  import { t } from '@shared/lib/i18n';
  import { ChevronDown, ChevronUp, Search, X } from 'lucide-svelte';
  import { tick, type Snippet } from 'svelte';

  interface Props {
    query: string;
    count: number;
    /** Coincidencia que se está viendo (desde 0). */
    index: number;
    /** Hay más coincidencias de las que se cuentan. */
    more?: boolean;
    /** Cambia cada vez que se pide buscar: vuelve a enfocar la caja. */
    request: number;
    placeholder?: string;
    onStep: (by: number) => void;
    onClose: () => void;
    /** Botones propios de quien la usa. */
    extra?: Snippet;
  }
  let { query = $bindable(), count, index, more = false, request, placeholder, onStep, onClose, extra }: Props = $props();

  let input = $state<HTMLInputElement | null>(null);
  const label = $derived(placeholder ?? t('find.placeholder'));

  export function focus() {
    input?.focus();
  }

  $effect(() => {
    void request;
    tick().then(() => {
      input?.focus();
      input?.select();
    });
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      // Cierra la búsqueda, no lo que la contiene
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (e.isComposing) return;
      e.preventDefault();
      e.stopPropagation();
      onStep(e.key === 'ArrowUp' || (e.key === 'Enter' && e.shiftKey) ? -1 : 1);
    }
  }
</script>

<div class="find-bar" role="search">
  <Search size={13} />
  <input
    bind:this={input}
    bind:value={query}
    class="find-input"
    type="text"
    placeholder={label}
    aria-label={label}
    onkeydown={onKeydown}
  />
  <span class="find-count" aria-live="polite">
    {#if query.trim()}{count ? `${index + 1}/${count}${more ? '+' : ''}` : t('find.none')}{/if}
  </span>
  {@render extra?.()}
  <button type="button" title={t('find.previous')} aria-label={t('find.previous')} disabled={!count} onclick={() => onStep(-1)}
    ><ChevronUp size={14} /></button
  >
  <button type="button" title={t('find.next')} aria-label={t('find.next')} disabled={!count} onclick={() => onStep(1)}
    ><ChevronDown size={14} /></button
  >
  <button type="button" title={t('find.close')} aria-label={t('find.close')} onclick={onClose}><X size={14} /></button>
</div>

<style>
  .find-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    color: var(--text-muted);
  }
  .find-input {
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
  .find-input:focus {
    border-color: var(--accent);
  }
  .find-count {
    font-size: 11px;
    white-space: nowrap;
  }
  .find-bar :global(button) {
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
  }
  .find-bar :global(button.active) {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .find-bar :global(button:disabled) {
    opacity: 0.4;
    cursor: default;
  }
  .find-bar :global(button:hover:not(:disabled)) {
    background: var(--bg-primary);
  }
</style>
