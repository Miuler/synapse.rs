<script lang="ts">
  import type { Conversation } from '@entities/acp-agent';
  import { AlertCircle, GitFork, Loader2, X } from 'lucide-svelte';
  import { renderAgentMarkdown } from '../lib/markdown';

  interface Props {
    /** Pregunta aparte (`/btw`) de la conversación. */
    side: Conversation;
    zoom: number;
    onLink: (e: MouseEvent) => void;
    onClose: () => void;
    onPromote: () => void;
  }

  let { side, zoom, onLink, onClose, onPromote }: Props = $props();

  const working = $derived(side.status === 'working');
  const cost = $derived(side.usage?.cost);
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="acp-side"
  role="dialog"
  aria-label="Pregunta aparte"
  tabindex="-1"
  onkeydown={(e) => {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    onClose();
  }}
>
  <div class="acp-side-head">
    <span class="acp-side-title">Pregunta aparte</span>
    {#if cost}<span class="acp-side-cost">{cost.amount} {cost.currency}</span>{/if}
    <button
      type="button"
      class="acp-side-promote"
      title="Seguir en una conversación propia, con el contexto de esta"
      disabled={side.connection !== 'ready' || !side.lastText('agent')}
      onclick={onPromote}><GitFork size={11} /> Convertir en conversación</button
    >
    <button type="button" aria-label="Cerrar la pregunta aparte" title="Cerrar (Esc): no queda guardada" onclick={onClose}>
      <X size={12} />
    </button>
  </div>
  <div class="acp-side-body" style={`zoom:${zoom}`}>
    {#each side.items as item (item.id)}
      {#if item.type === 'user'}
        <div class="acp-side-question">{item.text}</div>
      {:else if item.type === 'agent'}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="acp-side-answer" onclick={onLink}>{@html renderAgentMarkdown(item.text)}</div>
      {:else if item.type === 'info'}
        <div class="acp-side-info">{item.text}</div>
      {:else if item.type === 'error'}
        <div class="acp-side-info error"><AlertCircle size={12} /> {item.text}</div>
      {/if}
    {/each}
    {#if working}<div class="acp-side-info"><Loader2 size={12} class="acp-spin" /> Respondiendo…</div>{/if}
  </div>
</div>

<style>
  .acp-side {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 100%;
    margin-bottom: 4px;
    display: flex;
    flex-direction: column;
    max-height: 45vh;
    background: var(--bg-primary);
    border: 1px solid var(--accent);
    border-radius: 8px;
    box-shadow: var(--popover-shadow);
    font-size: 13px;
    z-index: 4;
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
    font-size: 11px;
    color: var(--text-muted);
  }
  .acp-side-title {
    flex: 1;
  }
  .acp-side-head button {
    display: inline-flex;
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
  .acp-side-body {
    min-height: 0;
    overflow-y: auto;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-wrap: anywhere;
    line-height: 1.45;
  }
  .acp-side-question {
    color: var(--text-secondary);
    font-weight: 600;
    white-space: pre-wrap;
  }
  .acp-side-answer :global(p) {
    margin: 0 0 0.6em;
  }
  .acp-side-answer :global(:last-child) {
    margin-bottom: 0;
  }
  .acp-side-answer :global(ul),
  .acp-side-answer :global(ol) {
    margin: 0 0 0.6em;
    padding-left: 1.4em;
  }
  .acp-side-answer :global(pre) {
    margin: 0 0 0.6em;
    padding: 8px;
    border-radius: 6px;
    overflow-x: auto;
    background: var(--bg-tertiary);
  }
  .acp-side-answer :global(code) {
    font-family: var(--font-mono, monospace);
    font-size: 0.9em;
  }
  .acp-side-answer :global(a) {
    color: var(--accent);
    cursor: pointer;
  }
  .acp-side-info {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
    color: var(--text-muted);
  }
  .acp-side-info.error {
    color: var(--danger);
  }
</style>
