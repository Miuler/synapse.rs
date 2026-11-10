<script lang="ts">
  // Índice de la nota: los encabezados en árbol, contraído por defecto (solo se ven los de primer
  // nivel), con botones para expandir y contraer todo.
  import { ChevronDown, ChevronRight, ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte';
  import { extractHeadingTree, type HeadingNode } from '../lib/heading-anchor';

  interface Props {
    content: string;
    onSelect: (heading: HeadingNode) => void;
    onClose: () => void;
  }

  let { content, onSelect, onClose }: Props = $props();

  let tree = $derived(extractHeadingTree(content));
  let expanded = $state<Record<number, boolean>>({});

  function allParents(nodes: HeadingNode[], out: number[] = []): number[] {
    for (const n of nodes) {
      if (n.children.length > 0) {
        out.push(n.line);
        allParents(n.children, out);
      }
    }
    return out;
  }

  function setAll(open: boolean) {
    const next: Record<number, boolean> = {};
    if (open) for (const line of allParents(tree)) next[line] = true;
    expanded = next;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="outline-backdrop" onclick={onClose}></div>
<div class="heading-outline" role="dialog" aria-label="Índice de la nota">
  <div class="outline-header">
    <span class="outline-title">Índice</span>
    <button type="button" class="outline-btn" aria-label="Expandir todo" title="Expandir todo" onclick={() => setAll(true)}>
      <ChevronsUpDown size={14} />
    </button>
    <button type="button" class="outline-btn" aria-label="Contraer todo" title="Contraer todo" onclick={() => setAll(false)}>
      <ChevronsDownUp size={14} />
    </button>
  </div>
  {#if tree.length === 0}
    <div class="outline-empty">La nota no tiene títulos</div>
  {:else}
    <ul class="outline-list">
      {#each tree as node (node.line)}
        {@render item(node, 0)}
      {/each}
    </ul>
  {/if}
</div>

{#snippet item(node: HeadingNode, depth: number)}
  <li>
    <div class="outline-row" style="padding-left: {6 + depth * 14}px">
      {#if node.children.length > 0}
        <button
          type="button"
          class="outline-toggle"
          aria-label={expanded[node.line] ? 'Contraer' : 'Expandir'}
          aria-expanded={!!expanded[node.line]}
          onclick={() => (expanded[node.line] = !expanded[node.line])}
        >
          {#if expanded[node.line]}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
        </button>
      {:else}
        <span class="outline-toggle-spacer"></span>
      {/if}
      <button type="button" class="outline-heading level-{node.level}" data-line={node.line} onclick={() => onSelect(node)}>
        <span class="outline-level">H{node.level}</span>
        <span class="outline-text">{node.text}</span>
      </button>
    </div>
    {#if node.children.length > 0 && expanded[node.line]}
      <ul class="outline-list">
        {#each node.children as child (child.line)}
          {@render item(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .outline-backdrop {
    position: fixed;
    inset: 0;
    z-index: 49;
  }
  .heading-outline {
    position: absolute;
    top: 100%;
    left: 16px;
    z-index: 50;
    width: min(460px, calc(100% - 32px));
    max-height: 60vh;
    overflow: auto;
    background: var(--bg-primary, #fff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
    padding: 6px;
    font-size: 13px;
  }
  .outline-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 6px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    margin-bottom: 4px;
  }
  .outline-title {
    flex: 1;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #656d76);
  }
  .outline-btn,
  .outline-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--text-secondary, #656d76);
    border-radius: 4px;
    cursor: pointer;
    padding: 3px;
  }
  .outline-btn:hover,
  .outline-toggle:hover {
    background: var(--bg-hover, rgba(0, 0, 0, 0.06));
    color: var(--text-primary, #1f2328);
  }
  .outline-toggle-spacer {
    display: inline-block;
    width: 18px;
  }
  .outline-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .outline-row {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .outline-heading {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    border: none;
    background: transparent;
    text-align: left;
    color: var(--text-primary, #1f2328);
    padding: 4px 6px;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
  }
  .outline-heading:hover {
    background: var(--bg-hover, rgba(0, 0, 0, 0.06));
  }
  .outline-heading.level-1 {
    font-weight: 600;
  }
  .outline-level {
    font-size: 10px;
    color: var(--text-secondary, #656d76);
    flex-shrink: 0;
  }
  .outline-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .outline-empty {
    padding: 8px;
    color: var(--text-secondary, #656d76);
  }
</style>
