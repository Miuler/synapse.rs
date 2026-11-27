<script lang="ts">
  // Fusión en curso: la lista de archivos en conflicto (como el diálogo «Conflicts» de JetBrains),
  // el editor de tres paneles de cada uno, y confirmar o cancelar la fusión.
  import { AlertCircle, GitMerge, Loader2, CheckCircle2 } from 'lucide-svelte';
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import { t } from '@shared/lib/i18n';
  import { flushPendingWrites } from '@shared/lib/pending-writes';
  import { gitApi, gitDialogState, type GitMergeConflict, type GitMergeState, type GitResolution } from '@entities/git';
  import { fromLf, toLf, usesCrlf } from '../lib/three-way-merge';
  import GitMergeEditor from './GitMergeEditor.svelte';
  import GitMergeDiff from './GitMergeDiff.svelte';

  interface Props {
    open?: boolean;
    /** Tras resolver, confirmar o cancelar: refrescar el estado de git de la app. */
    onRepoChanged?: () => void | Promise<void>;
    onToast?: (message: string) => void;
  }

  let { open = $bindable(false), onRepoChanged, onToast }: Props = $props();

  interface Editing {
    path: string;
    base: string;
    ours: string;
    theirs: string;
    crlf: boolean;
  }

  interface Viewing {
    conflict: GitMergeConflict;
    base: string | null;
    ours: string | null;
    theirs: string | null;
  }

  const repo = $derived(gitDialogState.repo);
  let merge = $state<GitMergeState | null>(null);
  let loading = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let selected = $state(0);
  let editing = $state<Editing | null>(null);
  let editor = $state<ReturnType<typeof GitMergeEditor> | null>(null);
  let message = $state('');
  /** Archivo cuyo diff se está mirando. */
  let viewing = $state<Viewing | null>(null);
  let differ = $state<ReturnType<typeof GitMergeDiff> | null>(null);
  let confirmAbort = $state(false);
  let listEl = $state<HTMLDivElement | null>(null);
  /** Archivos que han estado en conflicto desde que se abrió el diálogo. */
  let total = $state(0);

  const conflicts = $derived(merge?.conflicts ?? []);
  const canMerge = (c: GitMergeConflict) => !c.binary && c.ours && c.theirs;
  const message_of = (err: unknown) => (err instanceof Error ? err.message : String(err));

  async function load() {
    loading = true;
    try {
      const next = await gitApi.gitMergeState(repo);
      if (next && !merge) message = next.message;
      merge = next;
      total = Math.max(total, next?.conflicts.length ?? 0);
      selected = Math.max(0, Math.min(selected, (next?.conflicts.length ?? 1) - 1));
    } catch (err) {
      error = message_of(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (!open) return;
    merge = null;
    editing = null;
    viewing = null;
    error = null;
    confirmAbort = false;
    selected = 0;
    total = 0;
    message = '';
    void load().then(() => queueMicrotask(() => listEl?.focus()));
  });

  /** Ejecuta un paso de la fusión; el estado se vuelve a leer aunque falle. */
  async function step(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      // Lo que esté sin guardar se escribe antes: si no, pisaría después lo resuelto
      await flushPendingWrites();
      await action();
    } catch (err) {
      error = message_of(err);
    } finally {
      busy = false;
    }
  }

  function resolve(path: string, resolution: GitResolution) {
    return step(async () => {
      await gitApi.gitMerge({ kind: 'resolve', path, resolution }, repo);
      editing = null;
      viewing = null;
      await onRepoChanged?.();
      await load();
      queueMicrotask(() => listEl?.focus());
    });
  }

  async function openEditor(conflict: GitMergeConflict) {
    if (!canMerge(conflict) || busy) return;
    await step(async () => {
      const v = await gitApi.gitConflictVersions(conflict.path, repo);
      if (v.binary || v.ours === null || v.theirs === null) return;
      editing = {
        path: conflict.path,
        base: toLf(v.base ?? ''),
        ours: toLf(v.ours),
        theirs: toLf(v.theirs),
        crlf: usesCrlf(v.ours),
      };
    });
  }

  async function openDiff(conflict: GitMergeConflict) {
    if (conflict.binary || busy) return;
    await step(async () => {
      const v = await gitApi.gitConflictVersions(conflict.path, repo);
      if (v.binary) return;
      const lf = (text: string | null) => (text === null ? null : toLf(text));
      viewing = { conflict, base: lf(v.base), ours: lf(v.ours), theirs: lf(v.theirs) };
    });
  }

  function commit() {
    return step(async () => {
      const outcome = await gitApi.gitMerge({ kind: 'continue', message }, repo);
      await onRepoChanged?.();
      onToast?.(t('gitMerge.merged'));
      // El log anuncia el commit y ofrece deshacerlo
      if (outcome.kind === 'merged') gitDialogState.finishedMerge = { repo, point: outcome.undo };
      const target = repo;
      open = false;
      gitDialogState.openRepoLog(target);
    });
  }

  function abort() {
    confirmAbort = false;
    return step(async () => {
      await gitApi.gitMerge({ kind: 'abort' }, repo);
      await onRepoChanged?.();
      onToast?.(t('gitMerge.aborted'));
      open = false;
    });
  }

  function back() {
    editing = null;
    viewing = null;
    queueMicrotask(() => listEl?.focus());
  }

  function handleClose() {
    // Cerrar no cancela la fusión: sigue en el repositorio y el log permite retomarla
    if (editing) editor?.cancel();
    else if (viewing) back();
    else if (confirmAbort) confirmAbort = false;
    else open = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      // El diálogo ya llama a `handleClose`; salvo que un editor la haya usado (cerrar su buscador)
      if (e.defaultPrevented) e.stopPropagation();
      return;
    }
    if (editing) {
      if (e.key === 'F7') {
        e.preventDefault();
        editor?.step(e.shiftKey ? -1 : 1);
      }
      return;
    }
    if (viewing) {
      if (e.key === 'F7') {
        e.preventDefault();
        differ?.step(e.shiftKey ? -1 : 1);
      }
      return;
    }
    if ((e.target as HTMLElement).closest('textarea, button')) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(0, Math.min(conflicts.length - 1, selected + (e.key === 'ArrowDown' ? 1 : -1)));
      queueMicrotask(() => listEl?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }));
    } else if (e.key === 'Enter' && conflicts[selected]) {
      e.preventDefault();
      activate(conflicts[selected]);
    }
  }

  /** Enter o doble clic: el editor si se puede fusionar; si no, el diff. */
  const activate = (c: GitMergeConflict) => void (canMerge(c) ? openEditor(c) : openDiff(c));

  const sideLabel = (c: GitMergeConflict, present: boolean) =>
    !present ? t('gitMerge.deleted') : !c.base ? t('gitMerge.added') : t('gitMerge.modified');
</script>

<ResizableDialog
  {open}
  onClose={handleClose}
  holdClose={editing !== null || viewing !== null || confirmAbort}
  title={editing ? editing.path : viewing ? viewing.conflict.path : merge ? t('gitMerge.title', { theirs: merge.theirs, ours: merge.ours }) : t('gitMerge.conflicts')}
  description={editing ? t('gitMerge.editorDescription') : viewing ? t('gitMerge.diffDescription') : t('gitMerge.description')}
  sizeKey="synapse.gitMergeDialog.size"
  defaultSize={{ w: 1300, h: 780 }}
  minSize={{ w: 720, h: 420 }}
  class="git-merge-dialog"
  bodyPadding="0"
  onkeydown={handleKeyDown}
>
  {#snippet icon()}<GitMerge size={20} />{/snippet}

  <div class="git-merge">
    {#if error}
      <div class="git-merge-error" role="alert"><AlertCircle size={14} /> {error}</div>
    {/if}

    {#if editing && merge}
      {#key editing.path}
        <GitMergeEditor
          bind:this={editor}
          base={editing.base}
          ours={editing.ours}
          theirs={editing.theirs}
          path={editing.path}
          oursName={merge.ours}
          theirsName={merge.theirs}
          {busy}
          onApply={(result) => editing && resolve(editing.path, { kind: 'content', content: fromLf(result, editing.crlf) })}
          onAccept={(side) => editing && resolve(editing.path, { kind: side })}
          onCancel={back}
        />
      {/key}
    {:else if viewing && merge}
      {@const c = viewing.conflict}
      {#key c.path}
        <GitMergeDiff
          bind:this={differ}
          base={viewing.base}
          ours={viewing.ours}
          theirs={viewing.theirs}
          path={c.path}
          oursName={merge.ours}
          theirsName={merge.theirs}
        />
      {/key}
      <div class="git-merge-footer">
        <button type="button" data-action="close-diff" onclick={back}>{t('gitMerge.back')}</button>
        <span class="spacer"></span>
        <button type="button" data-action="take-ours" disabled={busy} onclick={() => resolve(c.path, { kind: 'ours' })}>
          {t('gitMerge.acceptYours')}{#if !c.ours}&nbsp;({t('gitMerge.deleteFile')}){/if}
        </button>
        <button type="button" data-action="take-theirs" disabled={busy} onclick={() => resolve(c.path, { kind: 'theirs' })}>
          {t('gitMerge.acceptTheirs')}{#if !c.theirs}&nbsp;({t('gitMerge.deleteFile')}){/if}
        </button>
        {#if canMerge(c)}
          <button type="button" class="primary" data-action="open-merge" disabled={busy} onclick={() => openEditor(c).then(() => (viewing = null))}>{t('gitMerge.merge')}</button>
        {/if}
      </div>
    {:else if loading && !merge}
      <div class="git-merge-state"><Loader2 size={18} class="spin" /></div>
    {:else if !merge}
      <div class="git-merge-state">{t('gitMerge.none')}</div>
    {:else}
      {#if conflicts.length}
        <div class="git-merge-summary">{t('gitMerge.remaining', { count: conflicts.length, total })}</div>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div class="git-merge-list" role="listbox" tabindex="0" aria-label={t('gitMerge.conflicts')} bind:this={listEl}>
          <div class="git-merge-row head" aria-hidden="true">
            <span>{t('gitMerge.file')}</span>
            <span>{t('gitMerge.yours')}</span>
            <span>{t('gitMerge.theirs')}</span>
            <span></span>
          </div>
          {#each conflicts as c, i (c.path)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="git-merge-row"
              role="option"
              tabindex="-1"
              aria-selected={i === selected}
              data-path={c.path}
              onclick={() => (selected = i)}
              ondblclick={() => activate(c)}
            >
              <span class="path" title={c.path}>{c.path}{#if c.binary}<em>{t('gitMerge.binary')}</em>{/if}</span>
              <span>{sideLabel(c, c.ours)}</span>
              <span>{sideLabel(c, c.theirs)}</span>
              <span class="actions">
                <button type="button" data-action="open-diff" disabled={busy || c.binary} title={c.binary ? t('gitMerge.noBinaryDiff') : undefined} onclick={() => openDiff(c)}>{t('gitMerge.diff')}</button>
                <button type="button" data-action="take-ours" disabled={busy} title={c.ours ? undefined : t('gitMerge.willDelete')} onclick={() => resolve(c.path, { kind: 'ours' })}>{t('gitMerge.acceptYours')}</button>
                <button type="button" data-action="take-theirs" disabled={busy} title={c.theirs ? undefined : t('gitMerge.willDelete')} onclick={() => resolve(c.path, { kind: 'theirs' })}>{t('gitMerge.acceptTheirs')}</button>
                <button type="button" class="primary" data-action="open-merge" disabled={busy || !canMerge(c)} title={canMerge(c) ? undefined : t('gitMerge.wholeOnly')} onclick={() => openEditor(c)}>{t('gitMerge.merge')}</button>
              </span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="git-merge-ready">
          <div class="ready-title"><CheckCircle2 size={18} /> {t('gitMerge.ready')}</div>
          <label>
            {t('gitMerge.message')}
            <textarea rows="6" bind:value={message}></textarea>
          </label>
        </div>
      {/if}

      <div class="git-merge-footer">
        {#if confirmAbort}
          <span role="alert">{t('gitMerge.abortConfirm')}</span>
          <span class="spacer"></span>
          <button type="button" onclick={() => (confirmAbort = false)}>{t('gitMerge.keepMerging')}</button>
          <button type="button" class="danger" data-action="confirm-abort" disabled={busy} onclick={abort}>{t('gitMerge.abort')}</button>
        {:else}
          <button type="button" class="danger" data-action="abort-merge" disabled={busy} onclick={() => (confirmAbort = true)}>{t('gitMerge.abort')}…</button>
          <span class="spacer"></span>
          <button type="button" onclick={() => (open = false)}>{t('gitMerge.later')}</button>
          <button type="button" class="primary" data-action="commit-merge" disabled={busy || conflicts.length > 0} onclick={commit}>{t('gitMerge.commit')}</button>
        {/if}
      </div>
    {/if}
  </div>
</ResizableDialog>

<style>
  .git-merge {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    font-size: 12px;
  }
  .git-merge-error {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    white-space: pre-wrap;
    color: var(--color-danger, #cf222e);
  }
  .git-merge-state {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--text-secondary, #656d76);
  }
  .git-merge-summary {
    padding: 8px 12px;
    color: var(--text-secondary, #656d76);
  }
  /* Las columnas son de la lista: cabecera y filas las comparten (`subgrid`) */
  .git-merge-list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(90px, auto) minmax(90px, auto) auto;
    align-content: start;
    column-gap: 10px;
  }
  .git-merge-row {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: center;
    padding: 4px 12px;
    border-bottom: 1px solid var(--border-primary, #d0d7de);
  }
  .git-merge-row.head {
    font-weight: 600;
    color: var(--text-secondary, #656d76);
  }
  .git-merge-row[aria-selected='true'] {
    background: rgba(9, 105, 218, 0.14);
  }
  .git-merge-row .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, monospace);
  }
  .git-merge-row .path em {
    margin-left: 8px;
    color: var(--text-secondary, #656d76);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .git-merge-ready {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    overflow: auto;
  }
  .ready-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: var(--git-staged, #16a34a);
  }
  .git-merge-ready label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .git-merge-ready textarea {
    padding: 8px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 6px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
  }
  .git-merge-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid var(--border-primary, #d0d7de);
  }
  .spacer {
    margin-left: auto;
  }
  .git-merge-row button,
  .git-merge-footer button {
    padding: 3px 10px;
    font-size: 12px;
    border: 1px solid var(--border-primary, #d0d7de);
    border-radius: 5px;
    background: var(--bg-primary, #fff);
    color: var(--text-primary, #1f2328);
    cursor: pointer;
  }
  .git-merge button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .git-merge button.primary {
    background: var(--accent, #0969da);
    border-color: var(--accent, #0969da);
    color: #fff;
  }
  .git-merge button.danger {
    color: var(--color-danger, #cf222e);
  }
</style>
