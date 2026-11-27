<!--
  Detalle de una tarea en un diálogo al estilo de Jira: la actividad (comentarios) a la izquierda y el
  estado y los datos a la derecha.
-->
<script lang="ts">
  import { SquareCheckBig, Send } from 'lucide-svelte';
  import { ResizableDialog } from '@shared/ui/dialog-resize';
  import { t } from '@shared/lib/i18n';
  import type { BoardColumn, BoardTask } from '../model/board.svelte';
  import type { IssueLink } from '../model/issue-sync';
  import type { TaskFile } from '../model/folder-board';

  interface Props {
    task: BoardTask;
    /** Todas las tareas, para nombrar de la que depende. */
    tasks: BoardTask[];
    columns: BoardColumn[];
    /** La tarea del proveedor enlazada, si la hay. */
    link?: IssueLink;
    /** El archivo de la tarea, en los tableros de carpeta. */
    file?: TaskFile;
    onClose: () => void;
    onAddComment: (taskId: string, text: string) => void;
  }

  let { task, tasks, columns, link, file, onClose, onAddComment }: Props = $props();
  let history = $derived(file?.activity.filter((entry) => entry.kind === 'history') ?? []);
  let column = $derived(columns.find((c) => c.id === task.status));
  let dependency = $derived(tasks.find((other) => other.id === task.dependsOn));
  let draft = $state('');

  function submit(e: Event) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    onAddComment(task.id, text);
    draft = '';
  }
</script>

<ResizableDialog
  open
  {onClose}
  title={task.title}
  sizeKey="synapse.boardTaskDialog.size"
  defaultSize={{ w: 880, h: 560 }}
  minSize={{ w: 520, h: 340 }}
  class="task-dialog"
  bodyPadding="0"
>
  {#snippet icon()}<SquareCheckBig size={20} />{/snippet}
  {#snippet badge()}<code class="task-id">{task.id}</code>{/snippet}

  <div class="task-columns">
    <section class="task-main">
      {#if file?.description}
        <h3>{t('board.detail.description')}</h3>
        <div class="description">{file.description}</div>
      {/if}
      <h3>{t('board.detail.comments')}</h3>
      <div class="comments-list">
        {#if task.comments.length === 0}
          <p class="empty">{t('board.detail.noComments')}</p>
        {:else}
          {#each task.comments as comment, i (i)}
            <div class="comment-item">
              <div class="comment-ts">
                {comment.timestamp.replace('T', ' ').slice(0, 19)}{comment.author ? ` · ${comment.author}` : ''}
              </div>
              <div class="comment-text">{comment.text}</div>
            </div>
          {/each}
        {/if}
      </div>

      {#if history.length}
        <h3>{t('board.detail.history')}</h3>
        <ul class="history">
          {#each history as entry, i (i)}
            <li>
              <small>{entry.at.replace('T', ' ').slice(0, 19)}{entry.author ? ` · ${entry.author}` : ''}</small
              >{entry.text}
            </li>
          {/each}
        </ul>
      {/if}

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
    </section>

    <aside class="task-side">
      <span class="detail-status" style="--status: {column?.color ?? 'var(--text-muted)'}">
        {column?.label ?? task.status}
      </span>

      <h3>{t('board.detail.details')}</h3>
      <dl class="details">
        {#if task.key}
          <dt>{t('board.detail.key')}</dt>
          <dd class="detail-key">{task.key}</dd>
        {/if}
        {#if task.type}
          <dt>{t('board.type')}</dt>
          <dd class="detail-type">{task.type}</dd>
        {/if}
        {#if task.assignee}
          <dt>{t('board.assignee')}</dt>
          <dd class="detail-assignee">{task.assignee}</dd>
        {/if}
        <dt>{t('board.detail.start')}</dt>
        <dd class="detail-start">{task.start}</dd>
        <dt>{t('board.detail.duration')}</dt>
        <dd class="detail-duration">{t('board.detail.days', { count: task.duration })}</dd>
        <dt>{t('board.detail.dependsOn')}</dt>
        <dd class="detail-depends">
          {#if task.dependsOn === '-'}
            {t('board.detail.none')}
          {:else}
            {task.dependsOn}{dependency ? ` · ${dependency.title}` : ''}
          {/if}
        </dd>
        <dt>{t('board.detail.progress')}</dt>
        <dd class="detail-progress">
          {task.progress}%
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
      </dl>
    </aside>
  </div>
</ResizableDialog>

<style>
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
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    grid-template-rows: minmax(0, 1fr);
  }

  .task-main {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 16px 20px;
    gap: 10px;
  }

  .task-side {
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

  .detail-status {
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

  .description,
  .history {
    max-height: 30%;
    overflow-y: auto;
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
    white-space: pre-wrap;
  }

  .history small {
    display: block;
    color: var(--text-muted);
  }

  .comments-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
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
