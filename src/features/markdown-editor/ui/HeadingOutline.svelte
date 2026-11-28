<script lang="ts">
  // Índice de la nota: los encabezados en árbol, contraído por defecto (solo se ven los de primer
  // nivel), con botones para expandir y contraer todo. Abre desplegado hasta el título donde está la
  // vista, que se resalta, y arriba tiene una búsqueda rápida de títulos.
  import { tick, untrack } from 'svelte';
  import { ChevronDown, ChevronRight, ChevronsDownUp, ChevronsUpDown, Search } from 'lucide-svelte';
  import { DialogResizeHandles, persistedDialogSize } from '@shared/ui/dialog-resize';
  import { t } from '@shared/lib/i18n';
  import { extractHeadingTree, flattenHeadings, headingPathAt, normalizeHeadingText, type HeadingNode } from '../lib/heading-anchor';

  interface Props {
    content: string;
    /** Árbol ya calculado (p. ej. las claves de un YAML); si falta, los títulos de `content`. */
    nodes?: HeadingNode[];
    /** Textos para otros tipos de archivo. */
    label?: string;
    emptyText?: string;
    /** Mostrar el nivel (`H1`, `H2`…): solo tiene sentido con títulos de Markdown. */
    showLevel?: boolean;
    searchPlaceholder?: string;
    /** Dentro de un diálogo: sin fondo para cerrar ni posición flotante (lo maneja el diálogo). */
    embedded?: boolean;
    /** A qué borde está anclado el popover para decidir hacia qué lado se redimensiona. */
    align?: 'left' | 'right';
    /** Línea del título donde está la vista (-1 o ausente si ninguno). */
    currentLine?: number;
    onSelect: (heading: HeadingNode) => void;
    onClose: () => void;
  }

  let { content, nodes, label, emptyText, showLevel = true, searchPlaceholder, embedded = false, align = 'left', currentLine = -1, onSelect, onClose }: Props = $props();

  const popoverSize = persistedDialogSize(
    'synapse.outlinePopover.size',
    { w: 460, h: 420 },
    { w: 280, h: 180 },
    { centered: false, maxHeightMargin: 120 },
  );

  let tree = $derived(nodes ?? extractHeadingTree(content));
  // Abre desplegado hasta el título actual (sus ancestros abiertos)
  // Solo el valor inicial: después el usuario despliega y pliega a mano
  let expanded = $state<Record<number, boolean>>(
    untrack(() =>
      Object.fromEntries(
        headingPathAt(nodes ?? extractHeadingTree(content), currentLine)
          .slice(0, -1)
          .map((h) => [h.line, true]),
      ),
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

  // Navegación con teclado: ↑/↓ recorren las entradas visibles, →/← despliegan y pliegan, Enter elige
  function visibleRows(nodes: HeadingNode[], out: HeadingNode[] = []): HeadingNode[] {
    for (const n of nodes) {
      out.push(n);
      if (n.children.length > 0 && isOpen(n)) visibleRows(n.children, out);
    }
    return out;
  }
  let rows = $derived(visibleRows(visibleTree));
  let activeLine = $state<number>(untrack(() => currentLine));
  // Al buscar, la entrada activa pasa a la primera coincidencia
  $effect(() => {
    const first = matches[0];
    if (normalizedQuery) activeLine = first ? first.line : -1;
  });

  function moveActive(delta: number) {
    if (rows.length === 0) return;
    const i = rows.findIndex((r) => r.line === activeLine);
    const next = i < 0 ? (delta > 0 ? 0 : rows.length - 1) : Math.max(0, Math.min(rows.length - 1, i + delta));
    activeLine = rows[next].line;
    tick().then(() => listEl?.querySelector(`.outline-heading[data-line="${activeLine}"]`)?.scrollIntoView({ block: 'nearest' }));
  }

  function onSearchKeydown(e: KeyboardEvent) {
    const active = rows.find((r) => r.line === activeLine);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      moveActive(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'PageDown' || e.key === 'PageUp') {
      e.preventDefault();
      moveActive(e.key === 'PageDown' ? 10 : -10);
    } else if (e.key === 'Enter') {
      const target = active ?? matches[0];
      if (target) {
        e.preventDefault();
        onSelect(target);
      }
    } else if (!normalizedQuery && active && active.children.length > 0 && (e.key === 'ArrowRight' || e.key === 'ArrowLeft')) {
      // Solo con la búsqueda vacía, para no quitarle las flechas al cursor del texto
      e.preventDefault();
      expanded[active.line] = e.key === 'ArrowRight';
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
    if (e.key === 'Escape' && !embedded) {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
{#if !embedded}<div class="outline-backdrop" onclick={onClose}></div>{/if}
<div
  class="heading-outline"
  class:embedded
  style={embedded ? undefined : popoverSize.style}
  role={embedded ? undefined : 'dialog'}
  aria-label={label ?? t('headingOutline.title')}
>
  {#if !embedded}
    <DialogResizeHandles
      onStart={popoverSize.startResize}
      edges={align === 'right' ? ['w', 's', 'sw'] : ['e', 's', 'se']}
    />
  {/if}
  <div class="outline-header">
    <label class="outline-search">
      <Search size={13} />
      <input
        bind:this={searchEl}
        bind:value={query}
        type="text"
        class="outline-search-input"
        placeholder={searchPlaceholder ?? t('headingOutline.searchPlaceholder')}
        aria-label={t('headingOutline.searchAria')}
        onkeydown={onSearchKeydown}
      />
    </label>
    <button type="button" class="outline-btn" aria-label={t('headingOutline.expandAll')} title={t('headingOutline.expandAll')} onclick={() => setAll(true)}>
      <ChevronsUpDown size={14} />
    </button>
    <button type="button" class="outline-btn" aria-label={t('headingOutline.collapseAll')} title={t('headingOutline.collapseAll')} onclick={() => setAll(false)}>
      <ChevronsDownUp size={14} />
    </button>
  </div>
  {#if tree.length === 0}
    <div class="outline-empty">{emptyText ?? t('headingOutline.emptyNote')}</div>
  {:else if visibleTree.length === 0}
    <div class="outline-empty">{t('headingOutline.noMatches')}</div>
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
        class:active={node.line === activeLine}
        aria-current={node.line === currentLine ? 'location' : undefined}
        data-line={node.line}
        onclick={() => onSelect(node)}
      >
        {#if showLevel}<span class="outline-level">H{node.level}</span>{/if}
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
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-sizing: border-box;
    background: var(--bg-primary, #fff);
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 8px;
    box-shadow: var(--popover-shadow);
    padding: 6px;
    font-size: 13px;
  }
  /* En un diálogo: ocupa todo su cuerpo */
  .heading-outline.embedded {
    position: static;
    width: 100%;
    height: 100%;
    max-height: none;
    border: none;
    border-radius: 0;
    box-shadow: none;
    box-sizing: border-box;
    padding: 8px 12px;
  }
  .outline-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 6px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
    margin-bottom: 4px;
  }
  .heading-outline > .outline-list {
    flex: 1;
    min-height: 0;
    overflow: auto;
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
  .outline-heading.active {
    outline: 1px solid var(--accent, #0969da);
    outline-offset: -1px;
    background: color-mix(in srgb, var(--accent, #0969da) 22%, transparent);
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
