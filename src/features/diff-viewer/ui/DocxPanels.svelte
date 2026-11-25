<script lang="ts">
  // Las dos versiones de un documento de Word, dibujadas lado a lado
  import { t } from '@shared/lib/i18n';
  import { renderDocx } from '@shared/lib/docx-render';
  import { untrack } from 'svelte';
  import { docxDiffApi } from '../api/docx-diff.api';
  import type { DiffSides, DiffSource } from '../lib/diff-sides';

  interface Props {
    sides: DiffSides;
    reloadKey?: number;
  }
  let { sides, reloadKey = 0 }: Props = $props();

  type Pane = { body: HTMLElement | null; styles: HTMLElement | null; message: string };
  const panes = $state<Record<'before' | 'after', Pane>>({
    before: { body: null, styles: null, message: '' },
    after: { body: null, styles: null, message: '' },
  });

  let generation = 0;
  async function draw(side: 'before' | 'after', source: DiffSource, gen: number) {
    const pane = panes[side];
    pane.message = t('docxDiff.loading');
    try {
      const data = await docxDiffApi.bytes(source);
      if (gen !== generation || !pane.body || !pane.styles) return;
      if (!data.byteLength) {
        pane.body.replaceChildren();
        pane.message = t('docxDiff.missing');
        return;
      }
      await renderDocx(data, pane.body, pane.styles, `docx-${side}`);
      if (gen === generation) pane.message = '';
    } catch (e) {
      if (gen === generation) pane.message = e instanceof Error ? e.message : String(e);
    }
  }

  const key = $derived(JSON.stringify([sides.before, sides.after, reloadKey]));
  $effect(() => {
    void key;
    if (!panes.before.body || !panes.after.body) return;
    untrack(() => {
      const gen = ++generation;
      void draw('before', sides.before, gen);
      void draw('after', sides.after, gen);
    });
  });
</script>

<div class="docx-panels">
  {#each ['before', 'after'] as const as side (side)}
    <section class="docx-pane">
      <header class="docx-pane-tag">{side === 'before' ? `− ${sides.beforeLabel}` : `+ ${sides.afterLabel}`}</header>
      {#if panes[side].message}<p class="docx-pane-msg">{panes[side].message}</p>{/if}
      <div class="docx-pane-scroll">
        <div bind:this={panes[side].styles}></div>
        <div class="docx-pane-body" bind:this={panes[side].body}></div>
      </div>
    </section>
  {/each}
</div>

<style>
  .docx-panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    height: 100%;
    min-height: 0;
  }
  .docx-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .docx-pane + .docx-pane {
    border-left: 1px solid var(--border-primary);
  }
  .docx-pane-tag {
    padding: 4px 10px;
    font-size: 12px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .docx-pane-msg {
    padding: 8px 12px;
    color: var(--text-muted);
  }
  .docx-pane-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .docx-pane-body {
    zoom: 0.7;
  }
  .docx-pane-body :global([class$='-wrapper']) {
    background: var(--bg-secondary);
    padding: 16px;
    align-items: flex-start;
    width: max-content;
    min-width: 100%;
    box-sizing: border-box;
  }
  .docx-pane-body :global([class$='-wrapper'] > section) {
    margin: 0 auto 16px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
    color: #000; /* fijo: papel, no depende del tema */
  }
</style>
