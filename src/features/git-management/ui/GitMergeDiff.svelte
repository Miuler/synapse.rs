<script lang="ts">
  // Qué pasó con un archivo en conflicto: diff de solo lectura entre el antecesor común y cada lado,
  // o entre los dos. Con un lado borrado, muestra la versión que sobrevive (lo que se borraría).
  import { untrack } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, highlightSpecialChars, keymap, lineNumbers } from '@codemirror/view';
  import { defaultKeymap } from '@codemirror/commands';
  import { MergeView, goToNextChunk, goToPreviousChunk } from '@codemirror/merge';
  import { search, searchKeymap } from '@codemirror/search';
  import { ChevronDown, ChevronUp } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { codeSyntax } from '../lib/code-syntax';

  interface Props {
    /** Cada versión con saltos de línea `\n`; `null` = en esa no existe. */
    base: string | null;
    ours: string | null;
    theirs: string | null;
    path: string;
    oursName: string;
    theirsName: string;
  }

  let { base, ours, theirs, path, oursName, theirsName }: Props = $props();

  type Version = 'base' | 'ours' | 'theirs';
  type Pair = [Version, Version];

  const text = (v: Version) => (v === 'base' ? base : v === 'ours' ? ours : theirs);
  function label(v: Version) {
    if (v === 'base') return t('gitMerge.baseTitle');
    const title = t(v === 'ours' ? 'gitMerge.yoursTitle' : 'gitMerge.theirsTitle', { name: v === 'ours' ? oursName : theirsName });
    return text(v) === null ? `${title} (${t('gitMerge.deleted')})` : title;
  }

  // El diálogo recrea el componente por archivo (`{#key}`)
  const pairs: Pair[] = untrack(() => {
    if (ours !== null && theirs !== null) {
      const both: Pair = ['ours', 'theirs'];
      return base === null ? [both] : [both, ['base', 'ours'], ['base', 'theirs']];
    }
    // Un lado lo borró: interesa lo que cambió en el que queda
    return [['base', theirs !== null ? 'theirs' : 'ours']];
  });
  let chosen = $state(0);
  const pair = $derived(pairs[chosen]);

  /** Qué lado borró el archivo, si alguno. */
  const deletedIn = untrack(() => (ours === null && theirs !== null ? oursName : theirs === null && ours !== null ? theirsName : null));
  const unchanged = $derived((text(pair[0]) ?? '') === (text(pair[1]) ?? ''));

  let container = $state<HTMLDivElement | null>(null);
  let view: MergeView | null = null;

  export function step(direction: 1 | -1) {
    if (view) (direction === 1 ? goToNextChunk : goToPreviousChunk)(view.b);
  }

  $effect(() => {
    if (!container) return;
    const [left, right] = pair;
    const extensions = [
      lineNumbers(),
      highlightSpecialChars(),
      search({ top: true }),
      keymap.of([...searchKeymap, ...defaultKeymap]),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      ...codeSyntax(path),
    ];
    const merge = untrack(
      () =>
        new MergeView({
          parent: container!,
          a: { doc: text(left) ?? '', extensions },
          b: { doc: text(right) ?? '', extensions },
          gutter: true,
        }),
    );
    view = merge;
    // Se empieza en el primer cambio
    const frame = requestAnimationFrame(() => {
      const first = merge.chunks[0];
      if (first) merge.b.dispatch({ effects: EditorView.scrollIntoView(Math.min(first.fromB, merge.b.state.doc.length), { y: 'center' }) });
    });
    return () => {
      cancelAnimationFrame(frame);
      merge.destroy();
      view = null;
    };
  });
</script>

<div class="merge-diff">
  <div class="merge-diff-toolbar">
    <button type="button" data-action="previous-change" onclick={() => step(-1)} title="{t('gitMerge.previous')} (Shift+F7)" aria-label={t('gitMerge.previous')}><ChevronUp size={14} /></button>
    <button type="button" data-action="next-change" onclick={() => step(1)} title="{t('gitMerge.next')} (F7)" aria-label={t('gitMerge.next')}><ChevronDown size={14} /></button>
    {#if pairs.length > 1}
      <select aria-label={t('gitMerge.compare')} bind:value={chosen}>
        {#each pairs as [left, right], i (i)}
          <option value={i}>{label(left)} ↔ {label(right)}</option>
        {/each}
      </select>
    {/if}
    {#if deletedIn}
      <span class="merge-diff-note" data-note="deleted">{t('gitMerge.deletedNote', { name: deletedIn })}</span>
    {/if}
    {#if unchanged}
      <span class="merge-diff-note">{t('gitMerge.noDifference')}</span>
    {/if}
  </div>
  <div class="merge-diff-titles">
    <span>{label(pair[0])}</span>
    <span>{label(pair[1])}</span>
  </div>
  <div class="merge-diff-panes" bind:this={container}></div>
</div>

<style>
  .merge-diff {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .merge-diff-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-diff-toolbar button,
  .merge-diff-toolbar select {
    display: inline-flex;
    padding: 3px 8px;
    font-size: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
  }
  .merge-diff-note {
    color: var(--text-secondary, #656d76);
  }
  .merge-diff-titles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .merge-diff-titles span {
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary, #656d76);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-diff-panes {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .merge-diff-panes :global(.cm-mergeView) {
    height: 100%;
    overflow: auto;
  }
  .merge-diff-panes :global(.cm-editor) {
    font-size: 12px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
  }
  .merge-diff-panes :global(.cm-scroller) {
    font-family: var(--font-mono, monospace);
  }
  .merge-diff-panes :global(.cm-gutters) {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
    border-color: var(--border-primary, #d0d7de);
  }
</style>
