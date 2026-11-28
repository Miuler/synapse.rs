<!--
  Detalle de una tarea en un diálogo al estilo de Jira: título, descripción y actividad a la
  izquierda, y el estado y los datos a la derecha. Todo se edita aquí; cambiar el histórico se avisa
  antes, porque el proveedor conserva el original.
-->
<script lang="ts">
  import { ExternalLink, SquareCheckBig, Send, Pencil } from 'lucide-svelte';
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import { t } from '@shared/lib/i18n';
  import type { BoardColumn, BoardComment, BoardTask } from '../model/board.svelte';
  import type { IssueLink } from '../model/issue-sync';
  import {
    awaitsProviderDescription,
    ownDescription,
    type ActivityEntry,
    type EntryRef,
    type TaskFile,
  } from '../model/folder-board';

  interface Props {
    task: BoardTask;
    /** Todas las tareas, para nombrar de la que depende. */
    tasks: BoardTask[];
    columns: BoardColumn[];
    /** La tarea del proveedor enlazada, si la hay. */
    link?: IssueLink;
    /** El archivo de la tarea, en los tableros de carpeta. */
    file?: TaskFile;
    /** Tablero de carpeta: además tiene descripción, tipo, asignado e histórico. */
    folder: boolean;
    /** En modo lectura el detalle solo se mira. */
    readOnly?: boolean;
    errorText: (error: unknown) => string;
    onClose: () => void;
    onAddComment: (taskId: string, text: string) => void;
    onUpdate: (patch: Partial<BoardTask>) => void;
    /** `previous` es la descripción que se estaba editando. */
    onSaveDescription: (previous: string, text: string) => Promise<void>;
    onSaveEntry: (entry: EntryRef, text: string) => Promise<void>;
    /** Cambiar la descripción de aquí por la del proveedor en el próximo «Traer». */
    onTakeProviderDescription: (previous: string) => Promise<void>;
  }

  let {
    task, tasks, columns, link, file, folder, errorText, onClose, onAddComment, onUpdate, onSaveDescription, onSaveEntry,
    onTakeProviderDescription, readOnly = false,
  }: Props = $props();
  const formatHandle = (name?: string) => (name ? (name.startsWith('@') ? name : `@${name}`) : undefined);
  const formatDateTime = (iso?: string) => (iso ? iso.replace('T', ' ').replace(/Z$/, '').slice(0, 19) : undefined);

  let history = $derived(file?.activity.filter((entry) => entry.kind === 'history') ?? []);
  let comments = $derived([...task.comments].reverse());
  let assigneeHandle = $derived(formatHandle(task.assignee));
  let creatorHandle = $derived(formatHandle(task.createdBy));
  let column = $derived(columns.find((c) => c.id === task.status));

  const SIDE_WIDTH_KEY = 'synapse.boardTaskDialog.sideWidth';
  const DEFAULT_SIDE_WIDTH = 340;
  const MIN_SIDE_WIDTH = 240;
  const MIN_MAIN_WIDTH = 260;

  function readSideWidth(): number {
    try {
      const saved = Number(localStorage.getItem(SIDE_WIDTH_KEY));
      if (Number.isFinite(saved) && saved >= MIN_SIDE_WIDTH) return saved;
    } catch { /* sin acceso a localStorage */ }
    return DEFAULT_SIDE_WIDTH;
  }

  let sideWidth = $state(readSideWidth());
  let columnsEl = $state<HTMLDivElement | null>(null);
  let resizingSide = $state(false);
  let dragStart: { x: number; width: number } | null = null;

  function onSideResizeStart(e: PointerEvent) {
    e.preventDefault();
    resizingSide = true;
    dragStart = { x: e.clientX, width: sideWidth };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onSideResizeMove(e: PointerEvent) {
    if (!resizingSide || !dragStart) return;
    const maxWidth = Math.max(MIN_SIDE_WIDTH, (columnsEl?.clientWidth ?? 960) - MIN_MAIN_WIDTH);
    sideWidth = Math.round(Math.min(maxWidth, Math.max(MIN_SIDE_WIDTH, dragStart.width + (dragStart.x - e.clientX))));
  }

  function onSideResizeEnd(e: PointerEvent) {
    if (!resizingSide) return;
    resizingSide = false;
    dragStart = null;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    try {
      localStorage.setItem(SIDE_WIDTH_KEY, String(sideWidth));
    } catch { /* sin acceso a localStorage */ }
  }

  function resetSideWidth() {
    sideWidth = DEFAULT_SIDE_WIDTH;
    try {
      localStorage.removeItem(SIDE_WIDTH_KEY);
    } catch { /* sin acceso a localStorage */ }
  }
  let dateDetails = $derived(
    (
      [
        ['dueDate', task.dueDate],
        ['createdAt', formatDateTime(task.createdAt)],
        ['updatedAt', formatDateTime(link?.updated)],
        ['stateChangedAt', formatDateTime(task.stateChangedAt)],
        ['activatedAt', formatDateTime(task.activatedAt)],
        ['resolvedAt', formatDateTime(task.resolvedAt)],
        ['closedAt', formatDateTime(task.closedAt)],
      ] as const
    ).filter(([, value]) => Boolean(value)),
  );
  let remoteDetails = $derived(
    (
      [
        ['priority', task.priority],
        ['severity', task.severity],
        ['estimate', task.estimate],
        ['iteration', task.iteration],
        ['area', task.area],
        ['parent', task.parent],
        ['reason', task.reason],
        ['boardColumn', task.boardColumn],
        ['boardLane', task.boardLane],
        ['originalEstimate', task.originalEstimate],
        ['remainingWork', task.remainingWork],
        ['completedWork', task.completedWork],
      ] as const
    ).filter(([, value]) => Boolean(value)),
  );
  /** Los valores que ya usa el tablero, como sugerencias. */
  const used = (key: 'type' | 'assignee') => [...new Set(tasks.flatMap((other) => other[key] ?? []))].sort();
  let draft = $state('');

  /** Lo que se está editando: un texto a la vez. */
  let editing = $state<{ key: string; original: string; text: string; save: (text: string) => Promise<void> } | null>(null);
  let saving = $state(false);
  let failed = $state('');
  /** La entrada del histórico a la espera de que se acepte el aviso. */
  let warned = $state.raw<ActivityEntry | null>(null);
  /** Qué se va a descartar, a la espera de confirmarlo. */
  let discarding = $state<'dialog' | 'editor' | null>(null);
  /** La descripción de aquí que se va a cambiar por la del proveedor. */
  let reverting = $state<string | null>(null);
  let unsaved = $derived((!!editing && editing.text !== editing.original) || !!draft.trim());

  function edit(key: string, original: string, save: (text: string) => Promise<void>) {
    failed = '';
    editing = { key, original, text: original, save };
  }

  const commentRef = (c: BoardComment): EntryRef => ({ kind: 'comment', id: c.remote ?? null, at: c.timestamp, text: c.text });
  const commentKey = (c: BoardComment) => `comment ${c.remote ?? ''} ${c.timestamp}`;
  // Enviado sin saber su id: hasta traerlo no se reconoce en el proveedor y cambiarlo lo duplicaría
  const unknownSent = (c: BoardComment) => folder && !c.remote && !!link?.sent?.includes(c.timestamp);

  async function saveEditing() {
    const current = editing;
    if (!current || saving) return;
    const text = current.key === 'description' ? current.text.replace(/\s+$/, '') : current.text.trim();
    // Un comentario no se queda vacío
    if (text === current.original || (!text && current.key !== 'description')) {
      editing = null;
      return;
    }
    saving = true;
    failed = '';
    try {
      await current.save(text);
      editing = null;
    } catch (error) {
      failed = errorText(error);
    } finally {
      saving = false;
    }
  }

  /** Un fotograma después: al abrirse, el botón que lo pidió o el aviso aún se quedan el foco. */
  function grabFocus(node: HTMLTextAreaElement) {
    requestAnimationFrame(() => node.focus());
  }

  function editorKeys(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (editing && editing.text !== editing.original) discarding = 'editor';
      else editing = null;
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void saveEditing();
    }
  }

  /** Un campo se guarda al salir de él o con Enter; Escape lo devuelve a como estaba. */
  function fieldKeys(e: KeyboardEvent & { currentTarget: HTMLInputElement }, saved: string) {
    if (e.key === 'Enter') e.currentTarget.blur();
    else if (e.key === 'Escape' && e.currentTarget.value !== saved) {
      e.preventDefault();
      e.stopPropagation();
      e.currentTarget.value = saved;
    }
  }

  /** El número escrito dentro de sus límites; si no es un número, el campo vuelve a como estaba. */
  function numberField(input: HTMLInputElement, key: 'duration' | 'progress', min: number, max: number) {
    const value = Math.round(input.valueAsNumber);
    if (Number.isNaN(value)) {
      input.value = String(task[key]);
      return;
    }
    const next = Math.min(max, Math.max(min, value));
    input.value = String(next);
    if (next !== task[key]) onUpdate({ [key]: next });
  }

  function textField(input: HTMLInputElement, key: 'type' | 'assignee') {
    const value = input.value.trim();
    if (value !== (task[key] ?? '')) onUpdate({ [key]: value || undefined });
  }

  function submit(e: Event) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    onAddComment(task.id, text);
    draft = '';
  }
</script>

{#snippet editor(label: string, rows: number)}
  {#if editing}
    <div class="editor">
      <textarea aria-label={label} {rows} use:grabFocus bind:value={editing.text} onkeydown={editorKeys}></textarea>
      {#if failed}<p class="editor-error" role="alert">{failed}</p>{/if}
      <div class="editor-actions">
        <button type="button" class="editor-cancel" onclick={() => (editing = null)}>{t('common.cancel')}</button>
        <button type="button" class="editor-save" disabled={saving} onclick={saveEditing}>{t('common.save')}</button>
      </div>
    </div>
  {/if}
{/snippet}

{#snippet pencil(label: string, onclick: () => void, disabledHint?: string)}
  {#if !readOnly}
  <button type="button" class="pencil" aria-label={label} title={disabledHint ?? label} disabled={!!editing || !!disabledHint} {onclick}>
    <Pencil size={12} />
  </button>
  {/if}
{/snippet}

<ResizableDialog
  open
  onClose={() => (unsaved ? (discarding = 'dialog') : onClose())}
  holdClose={unsaved}
  title={task.title}
  sizeKey="synapse.boardTaskDialog.size"
  defaultSize={{ w: 960, h: 560 }}
  minSize={{ w: 520, h: 340 }}
  class="task-dialog"
  bodyPadding="0"
>
  {#snippet icon()}<SquareCheckBig size={20} />{/snippet}
  {#snippet badge()}<code class="task-id">{task.id}</code>{/snippet}
  {#snippet subtitle()}
    {#if assigneeHandle || creatorHandle}
      <div class="task-people">
        {#if assigneeHandle}
          <span class="task-person is-assignee" title={t('board.assignee')}>{assigneeHandle}</span>
        {/if}
        {#if creatorHandle}
          <span class="task-person is-creator" title={t('board.createdBy')}>{t('board.createdBy')}: {creatorHandle}</span>
        {/if}
      </div>
    {/if}
  {/snippet}

  <div class="task-columns" class:is-resizing-side={resizingSide} bind:this={columnsEl}>
    <section class="task-main">
      <input
        class="detail-title"
        disabled={readOnly}
        type="text"
        aria-label={t('board.detail.title')}
        value={task.title}
        onkeydown={(e) => fieldKeys(e, task.title)}
        onchange={(e) => {
          const title = e.currentTarget.value.trim();
          if (title) onUpdate({ title });
          else e.currentTarget.value = task.title;
        }}
      />

      {#if folder && file}
        <div class="section-title">
          <h3>{t('board.detail.description')}</h3>
          {@render pencil(t('board.detail.editDescription'), () => {
            const previous = file.description;
            edit('description', previous, (text) => onSaveDescription(previous, text));
          })}
        </div>
        {#if editing?.key === 'description'}
          {@render editor(t('board.detail.description'), 8)}
        {:else if file.description}
          <div class="description">{file.description}</div>
        {:else}
          <p class="empty">{t('board.detail.noDescription')}</p>
        {/if}
        {#if link && !editing}
          {#if awaitsProviderDescription(file)}
            <p class="detail-note take-pending">{t('board.detail.takeProviderPending')}</p>
          {:else if !readOnly && ownDescription(file)}
            <button type="button" class="take-provider" onclick={() => (reverting = file!.description)}>
              {t('board.detail.takeProvider')}
            </button>
          {/if}
        {/if}
        {#if file.acceptanceCriteria}
          <div class="section-title">
            <h3>{t('board.detail.acceptanceCriteria')}</h3>
          </div>
          <div class="description">{file.acceptanceCriteria}</div>
        {/if}
      {/if}
      {#if failed && !editing}<p class="editor-error" role="alert">{failed}</p>{/if}

      <h3>{t('board.detail.comments')}</h3>
      {#if !readOnly}
      <form class="comment-form" onsubmit={submit}>
        <!-- Con algo escrito, Escape no cierra el diálogo: se perdería -->
        <input
          type="text"
          placeholder={t('board.detail.commentPlaceholder')}
          bind:value={draft}
          onkeydown={(e) => e.key === 'Escape' && draft && e.stopPropagation()}
        />
        <button type="submit" aria-label={t('board.detail.send')}>
          <Send size={14} />
        </button>
      </form>
      {/if}
      <div class="comments-list">
        {#if comments.length === 0}
          <p class="empty">{t('board.detail.noComments')}</p>
        {:else}
          {#each comments as comment, i (task.comments.length - 1 - i)}
            <div class="comment-item">
              <div class="comment-ts">
                <span>
                  {comment.timestamp.replace('T', ' ').slice(0, 19)}{comment.author ? ` · ${comment.author}` : ''}
                  {#if comment.edited}<em class="edited">{t('board.detail.edited')}</em>{/if}
                </span>
                {@render pencil(
                  t('board.detail.editComment'),
                  () => {
                    const ref = commentRef(comment);
                    edit(commentKey(comment), comment.text, (text) => onSaveEntry(ref, text));
                  },
                  unknownSent(comment) ? t('board.detail.pullToEdit') : undefined,
                )}
              </div>
              {#if editing?.key === commentKey(comment)}
                {@render editor(t('board.detail.editComment'), 3)}
              {:else}
                <div class="comment-text">{comment.text}</div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

      {#if history.length}
        <h3>{t('board.detail.history')}</h3>
        <ul class="history">
          {#each history as entry, i (i)}
            <li>
              <div class="comment-ts">
                <span>
                  {entry.at.replace('T', ' ').slice(0, 19)}{entry.author ? ` · ${entry.author}` : ''}
                  {#if entry.edited}<em class="edited">{t('board.detail.edited')}</em>{/if}
                </span>
                {@render pencil(t('board.detail.editHistory'), () => (warned = entry))}
              </div>
              {#if editing?.key === `history ${entry.id} ${entry.at}`}
                {@render editor(t('board.detail.editHistory'), 3)}
              {:else}
                <div class="history-text">{entry.text}</div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="side-resizer"
      class:active={resizingSide}
      role="separator"
      aria-orientation="vertical"
      aria-label={t('board.detail.resizeDetails')}
      title={t('board.resizeColumnHint')}
      onpointerdown={onSideResizeStart}
      onpointermove={onSideResizeMove}
      onpointerup={onSideResizeEnd}
      onpointercancel={onSideResizeEnd}
      ondblclick={resetSideWidth}
    ></div>

    <aside class="task-side" style="width: {sideWidth}px;">
      <!-- El estado es la insignia: al elegir otro, la tarea cambia de columna -->
      <select
        class="detail-status"
        style="--status: {column?.color ?? 'var(--text-muted)'}"
        aria-label={t('board.detail.status')}
        disabled={readOnly}
        value={task.status}
        onchange={(e) => onUpdate({ status: e.currentTarget.value })}
      >
        {#each columns as option (option.id)}
          <option value={option.id}>{option.label}</option>
        {/each}
        {#if !column}<option value={task.status}>{task.status}</option>{/if}
      </select>

      <h3>{t('board.detail.details')}</h3>
      <dl class="details">
        {#if task.key}
          <dt>{t('board.detail.key')}</dt>
          <dd class="detail-key">
            {#if task.url}
              <a class="detail-link-ext" href={task.url} target="_blank" rel="noopener noreferrer" title={t('board.openInProvider')}>
                <span>{task.key}</span>
                <ExternalLink size={11} />
              </a>
            {:else}
              {task.key}
            {/if}
          </dd>
        {/if}
        {#if folder}
          {#each ['type', 'assignee'] as const as key (key)}
            <dt>{t(`board.${key}`)}</dt>
            <dd class="detail-{key}">
              <input
                type="text"
                aria-label={t(`board.${key}`)}
                disabled={readOnly}
                list="task-{key}-values"
                value={task[key] ?? ''}
                onkeydown={(e) => fieldKeys(e, task[key] ?? '')}
                onchange={(e) => textField(e.currentTarget, key)}
              />
              <datalist id="task-{key}-values">
                {#each used(key) as value (value)}<option {value}></option>{/each}
              </datalist>
            </dd>
          {/each}
        {/if}
        {#if task.createdBy}
          <dt>{t('board.createdBy')}</dt>
          <dd class="detail-remote">{task.createdBy}</dd>
        {/if}
        {#if task.changedBy}
          <dt>{t('board.changedBy')}</dt>
          <dd class="detail-remote">{task.changedBy}</dd>
        {/if}
        <dt>{t('board.detail.start')}</dt>
        <dd class="detail-start">
          <input
            type="date"
            aria-label={t('board.detail.start')}
            disabled={readOnly}
            value={task.start}
            onchange={(e) => {
              if (e.currentTarget.value) onUpdate({ start: e.currentTarget.value });
              else e.currentTarget.value = task.start;
            }}
          />
        </dd>
        <dt>{t('board.detail.durationDays')}</dt>
        <dd class="detail-duration">
          <input
            type="number"
            min="1"
            aria-label={t('board.detail.durationDays')}
            disabled={readOnly}
            value={task.duration}
            onchange={(e) => numberField(e.currentTarget, 'duration', 1, 100000)}
          />
        </dd>
        {#each dateDetails as [field, value] (field)}
          <dt>{t(`board.${field}`)}</dt>
          <dd class="detail-remote">{value}</dd>
        {/each}
        <dt>{t('board.detail.dependsOn')}</dt>
        <dd class="detail-depends">
          <select aria-label={t('board.detail.dependsOn')} disabled={readOnly} value={task.dependsOn} onchange={(e) => onUpdate({ dependsOn: e.currentTarget.value })}>
            <option value="-">{t('board.detail.none')}</option>
            {#each tasks.filter((other) => other.id !== task.id) as other (other.id)}
              <option value={other.id}>{other.id} · {other.title}</option>
            {/each}
            {#if task.dependsOn !== '-' && !tasks.some((other) => other.id === task.dependsOn)}
              <option value={task.dependsOn}>{task.dependsOn}</option>
            {/if}
          </select>
        </dd>
        <dt>{t('board.detail.progressPercent')}</dt>
        <dd class="detail-progress">
          <input
            type="number"
            min="0"
            max="100"
            aria-label={t('board.detail.progressPercent')}
            disabled={readOnly}
            value={task.progress}
            onchange={(e) => numberField(e.currentTarget, 'progress', 0, 100)}
          />
          <div class="progress-bar"><div style="width: {task.progress}%"></div></div>
        </dd>
        {#if link}
          <dt>{t('issues.linked')}</dt>
          <dd class="detail-link">
            #{link.id}
            <span class="detail-note">{t('board.detail.syncedState', { state: link.state })}</span>
            {#if link.outside}<span class="detail-note detail-outside">{t('issues.outside')}</span>{/if}
          </dd>
        {/if}
        {#each remoteDetails as [field, value] (field)}
          <dt>{t(`board.${field}`)}</dt>
          <dd class="detail-remote">{value}</dd>
        {/each}
        {#if task.tags?.length}
          <dt>{t('board.tags')}</dt>
          <dd class="detail-tags">
            {#each task.tags as tag (tag)}
              <span class="detail-tag">{tag}</span>
            {/each}
          </dd>
        {/if}
      </dl>
    </aside>
  </div>
</ResizableDialog>

<ConfirmDialog
  open={warned !== null}
  title={t('board.detail.historyWarningTitle')}
  description={t('board.detail.historyWarning')}
  confirmText={t('board.detail.historyWarningConfirm')}
  cancelText={t('common.cancel')}
  onConfirm={() => {
    const entry = warned;
    warned = null;
    if (!entry) return;
    edit(`history ${entry.id} ${entry.at}`, entry.text, (text) => onSaveEntry(entry, text));
  }}
  onCancel={() => (warned = null)}
/>

<ConfirmDialog
  open={discarding !== null}
  title={t('board.detail.discardTitle')}
  description={t('board.detail.discardHint')}
  confirmText={t('board.detail.discard')}
  cancelText={t('common.cancel')}
  variant="danger"
  onConfirm={() => {
    const what = discarding;
    discarding = null;
    if (what === 'editor') editing = null;
    else if (what) onClose();
  }}
  onCancel={() => (discarding = null)}
/>

<ConfirmDialog
  open={reverting !== null}
  title={t('board.detail.takeProviderTitle')}
  description={t('board.detail.takeProviderHint')}
  confirmText={t('board.detail.takeProvider')}
  cancelText={t('common.cancel')}
  variant="danger"
  onConfirm={() => {
    const previous = reverting;
    reverting = null;
    if (previous === null) return;
    failed = '';
    onTakeProviderDescription(previous).catch((error) => (failed = errorText(error)));
  }}
  onCancel={() => (reverting = null)}
/>

<style>
  .task-people {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }

  .task-person {
    display: inline-block;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 500;
  }

  .task-person.is-assignee {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .task-person.is-creator {
    color: var(--text-secondary);
    background: var(--bg-tertiary);
  }

  .task-id {
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .task-columns {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .task-columns.is-resizing-side {
    user-select: none;
    cursor: col-resize;
  }

  .task-main {
    flex: 1;
    min-width: 260px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 20px;
    gap: 10px;
  }

  .side-resizer {
    width: 6px;
    margin-left: -3px;
    margin-right: -3px;
    flex-shrink: 0;
    cursor: col-resize;
    position: relative;
    z-index: 6;
    touch-action: none;
    background: transparent;
    transition: background 0.15s ease;
  }

  .side-resizer:hover,
  .side-resizer.active {
    background: var(--accent);
  }

  /* Lo de la columna principal se desplaza junto: nada se aplasta */
  .task-main > * {
    flex-shrink: 0;
  }

  input:not(.comment-form input),
  select,
  textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 4px 8px;
    color: var(--text-primary);
    font: inherit;
    font-size: 12px;
    outline: none;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: var(--accent);
  }

  .task-main .detail-title {
    font-size: 15px;
    font-weight: 600;
    padding: 6px 8px;
  }

  .section-title,
  .comment-ts {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .pencil {
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .pencil:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .pencil:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .editor textarea {
    resize: vertical;
  }

  .editor-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .editor-actions button {
    padding: 4px 10px;
    font-size: 12px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-primary);
    cursor: pointer;
  }

  .editor-actions .editor-save {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .editor-actions button:disabled {
    opacity: 0.5;
  }

  .editor-error {
    margin: 0;
    font-size: 12px;
    color: var(--danger, #d1242f);
  }

  .edited {
    margin-left: 4px;
    color: var(--warning, #d97706);
  }

  .take-provider {
    align-self: flex-start;
    padding: 3px 8px;
    font-size: 11px;
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .take-pending {
    margin: 0;
    font-size: 11px;
  }



  .task-side {
    flex-shrink: 0;
    max-width: 70%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
    overflow-y: auto;
    padding: 16px 20px;
    border-left: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }

  h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .task-side .detail-status {
    width: auto;
    max-width: 100%;
    flex-shrink: 0;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 5px 10px;
    border-radius: 4px;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--status) 18%, transparent);
    border: 1px solid var(--status);
  }

  .details {
    align-self: stretch;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px 12px;
    margin: 0;
    font-size: 12px;
  }

  .details dt {
    color: var(--text-muted);
  }

  .details dd {
    margin: 0;
    color: var(--text-primary);
    overflow-wrap: anywhere;
    user-select: text;
  }

  .progress-bar {
    height: 4px;
    margin-top: 4px;
    background: var(--bg-tertiary);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-bar div {
    height: 100%;
    background: var(--accent);
  }

  .detail-note {
    display: block;
    color: var(--text-secondary);
  }

  .detail-link-ext {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--accent);
    text-decoration: none;
  }

  .detail-link-ext:hover {
    text-decoration: underline;
  }

  .detail-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .detail-tag {
    font-size: 11px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }

  .description,
  .history {
    margin: 0;
    font-size: 12px;
    color: var(--text-primary);
    user-select: text;
  }

  .description {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .history {
    padding: 0;
    list-style: none;
  }

  .history li {
    padding: 4px 0;
  }

  .history-text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .comments-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .empty {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
  }

  .comment-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 8px 10px;
    user-select: text;
  }

  .comment-ts {
    min-height: 18px;
    font-size: 10px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .comment-text {
    font-size: 12px;
    color: var(--text-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .comment-form {
    display: flex;
    gap: 6px;
  }

  .comment-form input {
    flex: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 8px 10px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .comment-form input:focus {
    border-color: var(--accent);
  }

  .comment-form button {
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
</style>
