<script lang="ts">
  // Índice de la nota: los encabezados en árbol, contraído por defecto (solo se ven los de primer
  // nivel), con botones para expandir y contraer todo. Abre desplegado hasta el título donde está la
  // vista, que se resalta, y arriba tiene una búsqueda rápida de títulos.
  import { tick } from 'svelte';
  import { ChevronDown, ChevronRight, ChevronsDownUp, ChevronsUpDown, Search } from 'lucide-svelte';
  import { extractHeadingTree, flattenHeadings, headingPathAt, normalizeHeadingText, type HeadingNode } from '../lib/heading-anchor';

  interface Props {
    content: string;
    /** Línea del título donde está la vista (-1 o ausente si ninguno). */
    currentLine?: number;
    onSelect: (heading: HeadingNode) => void;
    onClose: () => void;
  }

  let { content, currentLine = -1, onSelect, onClose }: Props = $props();

  let tree = $derived(extractHeadingTree(content));
  // Abre desplegado hasta el título actual (sus ancestros abiertos)
  let expanded = $state<Record<number, boolean>>(
    Object.fromEntries(
      headingPathAt(extractHeadingTree(content), currentLine)
        .slice(0, -1)
        .map((h) => [h.line, true]),
    ),
  );
  let currentPath = $derived(new Set(headingPathAt(tree, currentLine).map((h) => h.line)));

  // Búsqueda rápida: muestra los títulos que coinciden con sus ancestros, todo desplegado
  let query = $state('');
  let searchEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);

  function filterTree(nodes: HeadingNode[], q: string): HeadingNode[] {
    const out: HeadingNode[] = [];
    for (const n of nodes) {
      const children = filterTree(n.children, q);
      if (children.length > 0 || normalizeHeadingText(n.text).includes(q)) out.push({ ...n, children });
    }
    return out;
  }
  let normalizedQuery = $derived(normalizeHeadingText(query));
  let visibleTree = $derived(normalizedQuery ? filterTree(tree, normalizedQuery) : tree);
  let matches = $derived(
    normalizedQuery ? flattenHeadings(visibleTree).filter((h) => normalizeHeadingText(h.text).includes(normalizedQuery)) : [],
  );
  const isOpen = (node: HeadingNode) => !!normalizedQuery || !!expanded[node.line];

  $effect(() => {
    tick().then(() => {
      searchEl?.focus();
      listEl?.querySelector('.outline-heading.current')?.scrollIntoView({ block: 'nearest' });
    });
  });

  function onSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && matches.length > 0) {
      e.preventDefault();
      onSelect(matches[0]);
    }
  }

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
    <label class="outline-search">
      <Search size={13} />
      <input
        bind:this={searchEl}
        bind:value={query}
        type="text"
        class="outline-search-input"
        placeholder="Buscar título..."
        aria-label="Buscar título"
        onkeydown={onSearchKeydown}
      />
    </label>
    <button type="button" class="outline-btn" aria-label="Expandir todo" title="Expandir todo" onclick={() => setAll(true)}>
      <ChevronsUpDown size={14} />
    </button>
    <button type="button" class="outline-btn" aria-label="Contraer todo" title="Contraer todo" onclick={() => setAll(false)}>
      <ChevronsDownUp size={14} />
    </button>
  </div>
  {#if tree.length === 0}
    <div class="outline-empty">La nota no tiene títulos</div>
  {:else if visibleTree.length === 0}
    <div class="outline-empty">Ningún título coincide</div>
  {:else}
    <ul class="outline-list" bind:this={listEl}>
      {#each visibleTree as node (node.line)}
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
          aria-label={isOpen(node) ? 'Contraer' : 'Expandir'}
          aria-expanded={isOpen(node)}
          onclick={() => (expanded[node.line] = !expanded[node.line])}
        >
          {#if isOpen(node)}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
        </button>
      {:else}
        <span class="outline-toggle-spacer"></span>
      {/if}
      <button
        type="button"
        class="outline-heading level-{node.level}"
        class:in-path={currentPath.has(node.line)}
        class:current={node.line === currentLine}
        aria-current={node.line === currentLine ? 'location' : undefined}
        data-line={node.line}
        onclick={() => onSelect(node)}
      >
        <span class="outline-level">H{node.level}</span>
        <span class="outline-text">{node.text}</span>
      </button>
    </div>
    {#if node.children.length > 0 && isOpen(node)}
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
    box-shadow: var(--popover-shadow);
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
  .outline-search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    color: var(--text-secondary, #656d76);
  }
  .outline-search:focus-within {
    border-color: var(--accent, #0969da);
  }
  .outline-search-input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary, #1f2328);
    font: inherit;
  }
  .outline-heading.in-path {
    color: var(--accent, #0969da);
  }
  .outline-heading.current {
    background: color-mix(in srgb, var(--accent, #0969da) 14%, transparent);
    font-weight: 600;
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
