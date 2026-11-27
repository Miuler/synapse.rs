<script lang="ts">
  // Editor de conflictos al estilo de «Merge Revisions» de JetBrains: la versión propia a la
  // izquierda, el resultado (editable, parte del antecesor común) en el centro y la ajena a la
  // derecha. Cada bloque cambiado se aplica o se descarta con los botones junto al resultado.
  import { untrack } from 'svelte';
  import { EditorState, RangeSet, StateEffect, StateField, type ChangeDesc, type Extension, type Range } from '@codemirror/state';
  import { Decoration, EditorView, GutterMarker, gutter, keymap, lineNumbers, highlightSpecialChars, drawSelection } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap, invertedEffects, undo as undoCommand } from '@codemirror/commands';
  import { search, searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { ChevronDown, ChevronUp, ChevronsRight, ChevronsLeft, ArrowRightLeft, Undo2 } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { codeSyntax } from '../lib/code-syntax';
  import { joinBlocks, mapLine, mergeChunks, splitLines, type LineRange, type MergeChunk } from '../lib/three-way-merge';

  interface Props {
    /** Las tres versiones, con saltos de línea `\n`. */
    base: string;
    ours: string;
    theirs: string;
    path: string;
    oursName: string;
    theirsName: string;
    busy?: boolean;
    onApply: (result: string) => void;
    onAccept: (side: Side) => void;
    onCancel: () => void;
  }

  type Side = 'ours' | 'theirs';
  type SideStatus = 'pending' | 'applied' | 'ignored';
  /** Un bloque en el resultado: dónde está y qué se ha hecho con cada lado. */
  interface Block {
    from: number;
    to: number;
    ours: SideStatus;
    theirs: SideStatus;
  }

  let { base, ours, theirs, path, oursName, theirsName, busy = false, onApply, onAccept, onCancel }: Props = $props();

  // El editor se monta una vez por archivo (el diálogo lo recrea con `{#key}`)
  const baseLines = untrack(() => splitLines(base));
  const sideLines = untrack(() => ({ ours: splitLines(ours), theirs: splitLines(theirs) }));
  const chunks: MergeChunk[] = mergeChunks(baseLines, sideLines.ours, sideLines.theirs);
  const involves = (chunk: MergeChunk, side: Side) => chunk.kind === side || chunk.kind === 'both' || chunk.kind === 'conflict';

  function offsets(lines: string[]): number[] {
    const at = [0];
    for (const line of lines) at.push(at[at.length - 1] + line.length);
    return at;
  }

  function initialBlocks(): Block[] {
    const at = offsets(baseLines);
    return chunks.map((chunk) => ({
      from: at[chunk.base[0]],
      to: at[chunk.base[1]],
      ours: involves(chunk, 'ours') ? 'pending' : 'ignored',
      theirs: involves(chunk, 'theirs') ? 'pending' : 'ignored',
    }));
  }

  // Lo que se escribe justo después de un bloque (en la línea siguiente) queda fuera de él
  const moved = (block: Block, changes: ChangeDesc, endsLine = true): Block => ({
    ...block,
    from: changes.mapPos(block.from, -1),
    to: changes.mapPos(block.to, endsLine ? -1 : 1),
  });

  const setBlock = StateEffect.define<{ index: number; block: Block }>({
    map: ({ index, block }, changes) => ({ index, block: moved(block, changes) }),
  });

  const blocksField = StateField.define<Block[]>({
    create: initialBlocks,
    update(blocks, tr) {
      let next = blocks;
      if (tr.docChanged) {
        const doc = tr.startState.doc;
        // Un bloque que acaba sin salto de línea (final del archivo) sí crece por detrás
        next = next.map((b) => moved(b, tr.changes, b.to === b.from || doc.sliceString(b.to - 1, b.to) === '\n'));
      }
      for (const effect of tr.effects) {
        if (effect.is(setBlock)) next = next.with(effect.value.index, effect.value.block);
      }
      return next;
    },
  });

  const isPending = (b: Block) => b.ours === 'pending' || b.theirs === 'pending';

  const blockClass = (chunk: MergeChunk, pending: boolean) =>
    `merge-line merge-${chunk.kind}${pending ? '' : ' merge-done'}`;

  /** Marca las líneas `[first, last]` (desde 1) de un bloque; si está vacío, solo dónde iría. */
  function markLines(state: EditorState, first: number, count: number, cls: string, out: Range<Decoration>[]) {
    const lines = state.doc.lines;
    if (count === 0) {
      out.push(Decoration.line({ class: `${cls} merge-empty` }).range(state.doc.line(Math.min(first, lines)).from));
      return;
    }
    for (let n = first; n < first + count && n <= lines; n++) {
      out.push(Decoration.line({ class: cls }).range(state.doc.line(n).from));
    }
  }

  /** Líneas (desde 0) que ocupa un bloque del resultado. */
  function resultRange(state: EditorState, block: Block): LineRange {
    const first = state.doc.lineAt(block.from).number - 1;
    if (block.to <= block.from) return [first, first];
    return [first, state.doc.lineAt(block.to - 1).number];
  }

  const resultDecorations = EditorView.decorations.compute([blocksField, 'doc'], (state) => {
    const out: Range<Decoration>[] = [];
    state.field(blocksField).forEach((block, i) => {
      const [from, to] = resultRange(state, block);
      markLines(state, from + 1, to - from, blockClass(chunks[i], isPending(block)), out);
    });
    return Decoration.set(out, true);
  });

  // ── Paneles laterales ────────────────────────────────────────────────────
  const setStatuses = StateEffect.define<SideStatus[]>();

  class ActionMarker extends GutterMarker {
    readonly side: Side;
    readonly index: number;
    constructor(side: Side, index: number) {
      super();
      this.side = side;
      this.index = index;
    }
    eq(other: ActionMarker) {
      return other.index === this.index && other.side === this.side;
    }
    toDOM() {
      const box = document.createElement('span');
      box.className = `merge-actions merge-${chunks[this.index].kind}`;
      box.dataset.chunk = String(this.index);
      const button = (action: 'apply' | 'ignore', text: string, title: string) => {
        const el = document.createElement('button');
        el.type = 'button';
        el.dataset.action = action;
        el.textContent = text;
        el.title = title;
        el.setAttribute('aria-label', title);
        el.addEventListener('mousedown', (e) => e.preventDefault());
        el.addEventListener('click', () => (action === 'apply' ? apply(this.index, this.side) : ignore(this.index, this.side)));
        return el;
      };
      const take = button('apply', this.side === 'ours' ? '»' : '«', t('gitMerge.applyChange'));
      const drop = button('ignore', '✕', t('gitMerge.ignoreChange'));
      box.append(...(this.side === 'ours' ? [drop, take] : [take, drop]));
      return box;
    }
  }

  function sideExtensions(side: Side): Extension[] {
    const statuses = StateField.define<SideStatus[]>({
      create: () => initialBlocks().map((b) => b[side]),
      update(value, tr) {
        for (const effect of tr.effects) if (effect.is(setStatuses)) return effect.value;
        return value;
      },
    });
    const decorations = EditorView.decorations.compute([statuses], (state) => {
      const out: Range<Decoration>[] = [];
      chunks.forEach((chunk, i) => {
        if (!involves(chunk, side)) return;
        const [from, to] = chunk[side];
        markLines(state, from + 1, to - from, blockClass(chunk, state.field(statuses)[i] === 'pending'), out);
      });
      return Decoration.set(out, true);
    });
    const actions = gutter({
      class: 'merge-gutter',
      side: side === 'ours' ? 'after' : 'before',
      markers: (view) => {
        const doc = view.state.doc;
        const marks: Range<GutterMarker>[] = [];
        view.state.field(statuses).forEach((status, i) => {
          if (status !== 'pending') return;
          const line = doc.line(Math.min(chunks[i][side][0] + 1, doc.lines));
          marks.push(new ActionMarker(side, i).range(line.from));
        });
        return RangeSet.of(marks, true);
      },
    });
    const readOnly = [EditorState.readOnly.of(true), EditorView.editable.of(false)];
    // El orden de las extensiones decide el de los márgenes: las acciones, pegadas al centro
    return side === 'ours'
      ? [statuses, decorations, lineNumbers(), actions, ...readOnly]
      : [statuses, decorations, actions, lineNumbers(), ...readOnly];
  }

  // ── Montaje ──────────────────────────────────────────────────────────────
  let leftEl = $state<HTMLDivElement | null>(null);
  let centerEl = $state<HTMLDivElement | null>(null);
  let rightEl = $state<HTMLDivElement | null>(null);
  let views: { ours: EditorView; result: EditorView; theirs: EditorView } | null = null;

  let blocks = $state.raw<Block[]>(initialBlocks());
  let edited = $state(false);
  let current = $state(-1);
  /** Pide confirmación antes de guardar con bloques sin decidir, o de salir perdiendo cambios. */
  let confirming = $state<'apply' | 'cancel' | null>(null);

  const pending = $derived(blocks.map((b, i) => ({ b, i })).filter(({ b }) => isPending(b)));
  const conflictsLeft = $derived(pending.filter(({ i }) => chunks[i].kind === 'conflict').length);
  const changesLeft = $derived(pending.length - conflictsLeft);

  /** Panel que mueve el usuario: los otros dos lo siguen. */
  let driver: EditorView | null = null;

  function lineAnchors(from: EditorView, to: EditorView): [number, number][] {
    const rangeIn = (view: EditorView, i: number): LineRange => {
      if (view === views!.result) return resultRange(view.state, view.state.field(blocksField)[i]);
      return chunks[i][view === views!.ours ? 'ours' : 'theirs'];
    };
    return chunks.flatMap((_, i) => {
      const a = rangeIn(from, i);
      const b = rangeIn(to, i);
      return [
        [a[0], b[0]],
        [a[1], b[1]],
      ] as [number, number][];
    });
  }

  function syncScroll(from: EditorView) {
    if (!views || from !== driver) return;
    // Se alinea la línea que está a un tercio del panel
    const reference = from.scrollDOM.clientHeight / 3;
    const y = from.scrollDOM.scrollTop + reference;
    const block = from.lineBlockAtHeight(y);
    const line = from.state.doc.lineAt(block.from).number - 1 + Math.min(1, Math.max(0, (y - block.top) / block.height));
    for (const to of [views.ours, views.result, views.theirs]) {
      if (to === from) continue;
      const target = mapLine(lineAnchors(from, to), line);
      const n = Math.min(to.state.doc.lines - 1, Math.max(0, Math.floor(target)));
      const targetBlock = to.lineBlockAt(to.state.doc.line(n + 1).from);
      to.scrollDOM.scrollTop = targetBlock.top + (target - n) * targetBlock.height - reference;
    }
  }

  $effect(() => {
    if (!leftEl || !centerEl || !rightEl) return;
    return untrack(() => {
      const common = [
        highlightSpecialChars(),
        drawSelection(),
        highlightSelectionMatches(),
        search({ top: true }),
        ...codeSyntax(path),
      ];
      const result = new EditorView({
        parent: centerEl!,
        state: EditorState.create({
          doc: base,
          extensions: [
            lineNumbers(),
            history(),
            blocksField,
            resultDecorations,
            invertedEffects.of((tr) =>
              tr.effects
                .filter((e) => e.is(setBlock))
                .map((e) => setBlock.of({ index: e.value.index, block: tr.startState.field(blocksField)[e.value.index] })),
            ),
            keymap.of([...historyKeymap, ...searchKeymap, ...defaultKeymap]),
            ...common,
            EditorView.updateListener.of((update) => {
              const next = update.state.field(blocksField);
              if (update.docChanged) edited = true;
              if (next === update.startState.field(blocksField)) return;
              blocks = next;
              for (const side of ['ours', 'theirs'] as const) {
                views?.[side].dispatch({ effects: setStatuses.of(next.map((b) => b[side])) });
              }
            }),
          ],
        }),
      });
      const side = (parent: HTMLElement, name: Side) =>
        new EditorView({
          parent,
          state: EditorState.create({ doc: name === 'ours' ? ours : theirs, extensions: [...sideExtensions(name), keymap.of([...searchKeymap, ...defaultKeymap]), ...common] }),
        });
      views = { ours: side(leftEl!, 'ours'), result, theirs: side(rightEl!, 'theirs') };

      const all = [views.ours, views.result, views.theirs];
      const cleanups = all.map((view) => {
        const drive = () => (driver = view);
        const scroll = () => syncScroll(view);
        const events = ['wheel', 'pointerdown', 'pointerenter', 'focusin', 'touchstart'];
        for (const name of events) view.dom.addEventListener(name, drive, { passive: true });
        view.scrollDOM.addEventListener('scroll', scroll, { passive: true });
        return () => {
          for (const name of events) view.dom.removeEventListener(name, drive);
          view.scrollDOM.removeEventListener('scroll', scroll);
        };
      });
      const frame = requestAnimationFrame(() => goTo(conflictIndex() ?? 0));
      return () => {
        cancelAnimationFrame(frame);
        cleanups.forEach((c) => c());
        all.forEach((v) => v.destroy());
        views = null;
      };
    });
  });

  function conflictIndex(): number | null {
    const i = chunks.findIndex((c) => c.kind === 'conflict');
    return i < 0 ? null : i;
  }

  function goTo(index: number) {
    if (!views || !chunks[index]) return;
    current = index;
    driver = null;
    const centerOn = (view: EditorView, pos: number) => view.dispatch({ effects: EditorView.scrollIntoView(pos, { y: 'center' }) });
    for (const side of ['ours', 'theirs'] as const) {
      const doc = views[side].state.doc;
      centerOn(views[side], doc.line(Math.min(chunks[index][side][0] + 1, doc.lines)).from);
    }
    const { from } = views.result.state.field(blocksField)[index];
    views.result.dispatch({ selection: { anchor: from }, effects: EditorView.scrollIntoView(from, { y: 'center' }) });
  }

  export function step(direction: 1 | -1) {
    if (!pending.length) return;
    const ids = pending.map((p) => p.i);
    const next = direction === 1 ? ids.find((i) => i > current) : ids.findLast((i) => i < current);
    goTo(next ?? ids[direction === 1 ? 0 : ids.length - 1]);
  }

  const other = (side: Side): Side => (side === 'ours' ? 'theirs' : 'ours');

  /** El bloque tras aplicar `side`, y lo que hay que escribir en su sitio. */
  function applied(index: number, side: Side, block: Block, doc: string) {
    const chunk = chunks[index];
    const text = sideLines[side].slice(...chunk[side]).join('');
    // En un conflicto, aceptar el segundo lado lo añade detrás del primero
    const insert = chunk.kind === 'conflict' && block[other(side)] === 'applied' ? joinBlocks(doc, text) : text;
    const status: Partial<Block> = chunk.kind === 'both' ? { ours: 'applied', theirs: 'applied' } : { [side]: 'applied' };
    return { insert, status };
  }

  function apply(index: number, side: Side) {
    const view = views?.result;
    if (!view || busy) return;
    const block = view.state.field(blocksField)[index];
    const { insert, status } = applied(index, side, block, view.state.sliceDoc(block.from, block.to));
    view.dispatch({
      changes: { from: block.from, to: block.to, insert },
      effects: setBlock.of({ index, block: { ...block, ...status, to: block.from + insert.length } }),
      userEvent: 'input.merge',
    });
    current = index;
  }

  function ignore(index: number, side: Side) {
    const view = views?.result;
    if (!view || busy) return;
    const block = view.state.field(blocksField)[index];
    const status: Partial<Block> = chunks[index].kind === 'both' ? { ours: 'ignored', theirs: 'ignored' } : { [side]: 'ignored' };
    view.dispatch({ effects: setBlock.of({ index, block: { ...block, ...status } }), userEvent: 'input.merge' });
    edited = true;
    current = index;
  }

  /** Aplica de una vez lo que no está en conflicto, de un lado o de los dos. */
  function applyNonConflicting(which: Side | 'all') {
    const view = views?.result;
    if (!view) return;
    const changes: { from: number; to: number; insert: string }[] = [];
    const effects: StateEffect<{ index: number; block: Block }>[] = [];
    // Lo que se han desplazado los bloques por los cambios anteriores de esta misma operación
    let shift = 0;
    view.state.field(blocksField).forEach((block, index) => {
      const kind = chunks[index].kind;
      if (kind === 'conflict' || !isPending(block) || (which !== 'all' && kind !== which && kind !== 'both')) return;
      const side: Side = kind === 'theirs' ? 'theirs' : 'ours';
      const { insert, status } = applied(index, side, block, '');
      changes.push({ from: block.from, to: block.to, insert });
      const from = block.from + shift;
      effects.push(setBlock.of({ index, block: { ...block, ...status, from, to: from + insert.length } }));
      shift += insert.length - (block.to - block.from);
    });
    if (changes.length) view.dispatch({ changes, effects, userEvent: 'input.merge' });
  }

  function save() {
    if (pending.length && confirming !== 'apply') {
      confirming = 'apply';
      return;
    }
    if (views) onApply(views.result.state.doc.toString());
  }

  export function cancel() {
    if (edited && confirming !== 'cancel') {
      confirming = 'cancel';
      return;
    }
    onCancel();
  }
</script>

<div class="merge-editor">
  <div class="merge-toolbar">
    <button type="button" data-action="previous-change" onclick={() => step(-1)} disabled={!pending.length} title="{t('gitMerge.previous')} (Shift+F7)" aria-label={t('gitMerge.previous')}><ChevronUp size={14} /></button>
    <button type="button" data-action="next-change" onclick={() => step(1)} disabled={!pending.length} title="{t('gitMerge.next')} (F7)" aria-label={t('gitMerge.next')}><ChevronDown size={14} /></button>
    <span class="merge-toolbar-label">{t('gitMerge.applyNonConflicting')}</span>
    <button type="button" data-action="apply-left" onclick={() => applyNonConflicting('ours')} disabled={!changesLeft}><ChevronsRight size={13} /> {t('gitMerge.left')}</button>
    <button type="button" data-action="apply-all" onclick={() => applyNonConflicting('all')} disabled={!changesLeft}><ArrowRightLeft size={13} /> {t('gitMerge.all')}</button>
    <button type="button" data-action="apply-right" onclick={() => applyNonConflicting('theirs')} disabled={!changesLeft}><ChevronsLeft size={13} /> {t('gitMerge.right')}</button>
    <button type="button" data-action="undo" onclick={() => views && undoCommand(views.result)} title="{t('gitLog.undo')} (Ctrl+Z)" aria-label={t('gitLog.undo')}><Undo2 size={13} /></button>
    <span class="merge-counter" aria-live="polite" data-conflicts={conflictsLeft} data-changes={changesLeft}>
      {t('gitMerge.counter', { changes: changesLeft, conflicts: conflictsLeft })}
    </span>
  </div>

  <div class="merge-titles">
    <span title={oursName}>{t('gitMerge.yoursTitle', { name: oursName })}</span>
    <span>{t('gitMerge.result')}</span>
    <span title={theirsName}>{t('gitMerge.theirsTitle', { name: theirsName })}</span>
  </div>
  <div class="merge-panes">
    <div class="merge-pane ours" bind:this={leftEl}></div>
    <div class="merge-pane result" bind:this={centerEl}></div>
    <div class="merge-pane theirs" bind:this={rightEl}></div>
  </div>

  <div class="merge-footer">
    {#if confirming}
      <span class="merge-confirm" role="alert">
        {confirming === 'apply' ? t('gitMerge.unprocessed', { count: pending.length }) : t('gitMerge.discardEdits')}
      </span>
      <button type="button" onclick={() => (confirming = null)}>{t('gitMerge.keepEditing')}</button>
      <button type="button" class="primary" data-action="confirm-merge-editor" disabled={busy} onclick={() => (confirming === 'apply' ? save() : onCancel())}>
        {confirming === 'apply' ? t('gitMerge.applyAnyway') : t('gitMerge.discard')}
      </button>
    {:else}
      <button type="button" data-action="accept-left" disabled={busy} onclick={() => onAccept('ours')}>{t('gitMerge.acceptYours')}</button>
      <button type="button" data-action="accept-right" disabled={busy} onclick={() => onAccept('theirs')}>{t('gitMerge.acceptTheirs')}</button>
      <span class="spacer"></span>
      <button type="button" data-action="cancel-merge-editor" onclick={cancel}>{t('common.cancel')}</button>
      <button type="button" class="primary" data-action="apply-result" disabled={busy} onclick={save}>{t('gitMerge.apply')}</button>
    {/if}
  </div>
</div>

<style>
  .merge-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .merge-toolbar,
  .merge-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    font-size: 12px;
  }
  .merge-toolbar {
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-footer {
    border-top: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-editor button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    font-size: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
  }
  .merge-editor button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .merge-editor button.primary {
    background: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    color: #fff;
  }
  .merge-toolbar-label {
    margin-left: 8px;
    color: var(--text-secondary, #656d76);
  }
  .merge-counter,
  .spacer {
    margin-left: auto;
  }
  .merge-titles,
  .merge-panes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .merge-titles span {
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary, #656d76);
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-panes {
    flex: 1;
    min-height: 0;
  }
  .merge-pane {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .merge-pane.result {
    border-inline: 1px solid var(--border-primary, #d0d7de);
  }
  .merge-pane :global(.cm-editor) {
    height: 100%;
    font-size: 12px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
  }
  .merge-pane :global(.cm-scroller) {
    overflow: auto;
    font-family: var(--font-mono, monospace);
  }
  .merge-pane :global(.cm-gutters) {
    background: var(--bg-secondary, #f6f8fa);
    color: var(--text-secondary, #656d76);
    border-color: var(--border-primary, #d0d7de);
  }
  .merge-pane :global(.cm-cursor) {
    border-left-color: var(--text-primary, #1f2328);
  }
  /* Azul: cambio de un lado (o igual en los dos); rojo: conflicto. Atenuado: ya decidido */
  .merge-pane :global(.merge-line) {
    background: rgba(9, 105, 218, 0.16);
  }
  .merge-pane :global(.merge-line.merge-conflict) {
    background: rgba(220, 38, 38, 0.18);
  }
  .merge-pane :global(.merge-line.merge-done) {
    background: rgba(128, 128, 128, 0.1);
  }
  .merge-pane :global(.merge-line.merge-empty) {
    background: transparent;
    box-shadow: inset 0 2px 0 rgba(9, 105, 218, 0.7);
  }
  .merge-pane :global(.merge-line.merge-empty.merge-conflict) {
    box-shadow: inset 0 2px 0 rgba(220, 38, 38, 0.8);
  }
  .merge-pane :global(.merge-line.merge-empty.merge-done) {
    box-shadow: inset 0 1px 0 rgba(128, 128, 128, 0.5);
  }
  .merge-pane :global(.merge-gutter) {
    width: 40px;
  }
  .merge-pane :global(.merge-actions) {
    display: flex;
    justify-content: center;
    gap: 2px;
  }
  .merge-pane :global(.merge-actions button) {
    padding: 0 3px;
    border: none;
    border-radius: 3px;
    line-height: 1.2;
    background: rgba(9, 105, 218, 0.18);
    color: inherit;
    cursor: pointer;
  }
  .merge-pane :global(.merge-actions.merge-conflict button) {
    background: rgba(220, 38, 38, 0.22);
  }
</style>
