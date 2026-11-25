<script lang="ts">
  // Diff estructural de un documento de Word: párrafos, títulos, tablas e imágenes
  import { t } from '@shared/lib/i18n';
  import { AlertCircle, CheckCircle2, ChevronDown, ChevronUp, Info, UnfoldVertical } from 'lucide-svelte';
  import { untrack } from 'svelte';
  import { docxDiffApi, type DocxBlock, type DocxBlockChange, type DocxCell, type DocxDiff, type DocxDiffEntry } from '../api/docx-diff.api';
  import type { DiffSides } from '../lib/diff-sides';
  import { countChanges, groupEntries } from '../lib/docx-groups';
  import DocxDiffImage from './DocxDiffImage.svelte';

  interface Props {
    sides: DiffSides;
    /** Cambia para volver a comparar. */
    reloadKey?: number;
  }
  let { sides, reloadKey = 0 }: Props = $props();

  let data = $state.raw<DocxDiff | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let unfolded = $state<number[]>([]);
  let scroller = $state<HTMLDivElement | null>(null);
  let current = $state(0);

  let generation = 0;
  async function load() {
    const gen = ++generation;
    loading = true;
    error = null;
    try {
      const result = await docxDiffApi.diff(sides.before, sides.after);
      if (gen !== generation) return;
      data = result;
      unfolded = [];
      current = 0;
    } catch (e) {
      if (gen === generation) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (gen === generation) loading = false;
    }
  }
  // Por su contenido: `sides` se recalcula aunque no cambie
  const key = $derived(JSON.stringify([sides.before, sides.after, reloadKey]));
  $effect(() => {
    void key;
    untrack(load);
  });

  const groups = $derived(groupEntries(data?.entries ?? []));
  const counts = $derived(countChanges(data?.entries ?? []));
  const total = $derived(counts.added + counts.removed + counts.modified);

  function go(by: number) {
    const all = [...(scroller?.querySelectorAll<HTMLElement>(':scope > .docx-diff-page > .docx-entry.changed') ?? [])];
    if (!all.length) return;
    current = current === 0 && by < 0 ? all.length : ((current - 1 + by + all.length) % all.length) + 1;
    all[current - 1].scrollIntoView({ block: 'center' });
  }

  const marks = { equal: '', added: '+', removed: '−', modified: '~' } as const;
  const labels = {
    equal: () => undefined,
    added: () => t('docxDiff.added'),
    removed: () => t('docxDiff.removed'),
    modified: () => t('docxDiff.modified'),
  } as const;
  const sourceOf = (kind: string) => (kind === 'removed' ? sides.before : sides.after);
</script>

{#snippet textBlock(heading: number | null, listLevel: number | null, content: () => ReturnType<import('svelte').Snippet>)}
  {#if heading}
    <div class="docx-heading h{Math.min(heading, 6)}" role="heading" aria-level={heading}>{@render content()}</div>
  {:else}
    <p class="docx-p" class:list={listLevel !== null} style:margin-left={listLevel === null ? undefined : `${(listLevel + 1) * 20}px`}>
      {@render content()}
    </p>
  {/if}
{/snippet}

{#snippet block(b: DocxBlock, kind: 'equal' | 'added' | 'removed')}
  {#if b.type === 'text'}
    {#snippet content()}{#if kind === 'added'}<ins>{b.text}</ins>{:else if kind === 'removed'}<del>{b.text}</del>{:else}{b.text}{/if}{/snippet}
    {@render textBlock(b.heading, b.listLevel, content)}
  {:else if b.type === 'image'}
    <DocxDiffImage image={b} source={sourceOf(kind)} />
  {:else}
    <table class="docx-table">
      <tbody>
        {#each b.rows as row, r (r)}
          <tr>{#each row as c, i (i)}{@render plainCell(c, kind)}{/each}</tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/snippet}

{#snippet plainCell(c: DocxCell, kind: 'equal' | 'added' | 'removed')}
  <td colspan={c.colSpan} class:merged={c.merged}>
    {#each c.blocks as b, i (i)}{@render block(b, kind)}{/each}
  </td>
{/snippet}

{#snippet change(c: DocxBlockChange)}
  {#if c.type === 'text'}
    {#snippet content()}{#each c.segments as s, i (i)}{#if s.kind === 'added'}<ins>{s.text}</ins>{:else if s.kind === 'removed'}<del>{s.text}</del>{:else}{s.text}{/if}{/each}{/snippet}
    {#if c.oldHeading !== c.newHeading}
      <span class="docx-note">
        {c.oldHeading ? t('docxDiff.heading', { level: c.oldHeading }) : t('docxDiff.paragraph')} →
        {c.newHeading ? t('docxDiff.heading', { level: c.newHeading }) : t('docxDiff.paragraph')}
      </span>
    {/if}
    {@render textBlock(c.newHeading, c.listLevel, content)}
  {:else if c.type === 'image'}
    <div class="docx-images">
      <figure class="removed"><figcaption>− {sides.beforeLabel}</figcaption><DocxDiffImage image={c.old} source={sides.before} /></figure>
      <figure class="added"><figcaption>+ {sides.afterLabel}</figcaption><DocxDiffImage image={c.new} source={sides.after} /></figure>
    </div>
  {:else}
    <table class="docx-table">
      <tbody>
        {#each c.rows as row, r (r)}
          <tr class="row-{row.kind}" title={labels[row.kind]()}>
            {#each row.cells as cell, i (i)}
              <td colspan={cell.colSpan} class="cell-{cell.kind}" class:merged={cell.merged} title={labels[cell.kind]()}>
                {#each cell.entries as e, k (k)}{@render entry(e)}{/each}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/snippet}

{#snippet entry(e: DocxDiffEntry)}
  {#if e.kind === 'modified'}{@render change(e.change)}{:else}{@render block(e.block, e.kind)}{/if}
{/snippet}

{#snippet row(e: DocxDiffEntry)}
  <div class="docx-entry {e.kind}" class:changed={e.kind !== 'equal'}>
    <span class="docx-mark" title={labels[e.kind]()} aria-label={labels[e.kind]()}>{marks[e.kind]}</span>
    <div class="docx-content">{@render entry(e)}</div>
  </div>
{/snippet}

<div class="docx-diff">
  {#if loading && !data}
    <div class="docx-state">{t('docxDiff.loading')}</div>
  {:else if error}
    <div class="docx-state error"><AlertCircle size={28} /><span>{error}</span></div>
  {:else if data}
    <div class="docx-diff-bar">
      <span class="side">− {sides.beforeLabel}</span>
      <span class="side">+ {sides.afterLabel}</span>
      <span class="stats">
        {#if counts.added}<span class="add">+{counts.added}</span>{/if}
        {#if counts.removed}<span class="rem">−{counts.removed}</span>{/if}
        {#if counts.modified}<span class="mod">~{counts.modified}</span>{/if}
      </span>
      {#if total}
        <button type="button" title={t('docxDiff.previous')} aria-label={t('docxDiff.previous')} onclick={() => go(-1)}><ChevronUp size={14} /></button>
        <span class="pos">{current || '–'}/{total}</span>
        <button type="button" title={t('docxDiff.next')} aria-label={t('docxDiff.next')} onclick={() => go(1)}><ChevronDown size={14} /></button>
      {/if}
    </div>
    <div class="docx-diff-info">
      <Info size={13} />
      <span>
        {#if !data.oldExists}{t('docxDiff.onlyAfter')}{:else if !data.newExists}{t('docxDiff.onlyBefore')}{/if}
        {#if data.approximate}{t('docxDiff.approximate')}{/if}
        {t('docxDiff.scope')}
      </span>
    </div>
    {#if !total}
      <div class="docx-state">
        <CheckCircle2 size={28} />
        <span>{data.sameBytes ? t('docxDiff.identical') : t('docxDiff.onlyFormat')}</span>
      </div>
    {:else}
      <div class="docx-diff-scroll" bind:this={scroller}>
        <div class="docx-diff-page">
          {#each groups as g (g.start)}
            {#if g.type === 'fold' && !unfolded.includes(g.start)}
              <button type="button" class="docx-fold" onclick={() => (unfolded = [...unfolded, g.start])}>
                <UnfoldVertical size={13} />
                {t('docxDiff.unchanged', { count: g.entries.length })}
              </button>
            {:else}
              {#each g.entries as e, i (i)}{@render row(e)}{/each}
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .docx-diff {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .docx-diff-bar,
  .docx-diff-info {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 12px;
    font-size: 12px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }
  .docx-diff-info {
    gap: 6px;
    color: var(--text-muted);
    background: transparent;
  }
  .docx-diff-bar .side {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .docx-diff-bar .stats {
    display: inline-flex;
    gap: 6px;
    margin-left: auto;
    font-weight: 600;
  }
  .add {
    color: var(--success);
  }
  .rem {
    color: var(--danger);
  }
  .mod {
    color: var(--warning);
  }
  .docx-diff-bar button {
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .docx-diff-bar button:hover {
    background: var(--bg-hover);
  }
  .docx-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
  }
  .docx-state.error {
    color: var(--error-text);
  }
  .docx-diff-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .docx-diff-page {
    max-width: 900px;
    margin: 0 auto;
    padding: 16px;
    line-height: 1.55;
  }
  .docx-entry {
    display: flex;
    gap: 8px;
    padding: 2px 8px 2px 4px;
    border-left: 3px solid transparent;
  }
  .docx-entry.added {
    border-left-color: var(--success);
    background: var(--success-bg);
  }
  .docx-entry.removed {
    border-left-color: var(--danger);
    background: var(--danger-bg);
  }
  .docx-entry.modified {
    border-left-color: var(--warning);
  }
  .docx-mark {
    width: 14px;
    flex-shrink: 0;
    text-align: center;
    font-weight: 700;
    color: var(--text-muted);
    user-select: none;
  }
  .docx-content {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
  }
  .docx-p,
  .docx-heading {
    margin: 4px 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .docx-p.list {
    display: list-item;
    list-style: disc;
  }
  .docx-heading {
    font-weight: 700;
  }
  .docx-heading.h1 {
    font-size: 1.5em;
  }
  .docx-heading.h2 {
    font-size: 1.3em;
  }
  .docx-heading.h3 {
    font-size: 1.15em;
  }
  ins {
    background: var(--success-bg);
    color: var(--success);
    text-decoration: underline;
  }
  del {
    background: var(--danger-bg);
    color: var(--danger);
    text-decoration: line-through;
  }
  .docx-note {
    font-size: 11px;
    color: var(--text-muted);
  }
  .docx-table {
    margin: 6px 0;
    border-collapse: collapse;
    font-size: 0.95em;
  }
  .docx-table td {
    padding: 3px 8px;
    border: 1px solid var(--border-primary);
    vertical-align: top;
  }
  .docx-table td.merged {
    border-top-style: dotted;
  }
  .row-added td,
  td.cell-added {
    background: var(--success-bg);
    border-color: var(--success-border);
  }
  .row-removed td,
  td.cell-removed {
    background: var(--danger-bg);
    border-color: var(--danger-border);
  }
  td.cell-modified {
    background: var(--warning-bg);
    border-color: var(--warning-border);
  }
  .docx-images {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }
  .docx-images figure {
    margin: 4px 0;
    padding: 8px;
    border: 1px solid var(--border-primary);
    border-radius: 4px;
  }
  .docx-images figure.removed {
    border-color: var(--danger-border);
  }
  .docx-images figure.added {
    border-color: var(--success-border);
  }
  .docx-images figcaption {
    margin-bottom: 6px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .docx-fold {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    margin: 6px 0;
    padding: 4px;
    border: 1px dashed var(--border-primary);
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }
  .docx-fold:hover {
    background: var(--bg-hover);
  }
</style>
