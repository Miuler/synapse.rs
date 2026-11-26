<script lang="ts">
  import { MessageSquare, Plus } from 'lucide-svelte';
  import type { BoardColumn, BoardTask } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    columns: BoardColumn[];
    onUpdateTask: (id: string, patch: Partial<BoardTask>) => void;
    onAddTask: (status?: string) => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, columns, onUpdateTask, onAddTask, onSelectComments }: Props = $props();

  let draggedTaskId = $state<string | null>(null);
  let dragOverCol = $state<string | null>(null);

  function handleDrop(colId: string) {
    if (draggedTaskId) {
      onUpdateTask(draggedTaskId, { status: colId });
    }
    draggedTaskId = null;
    dragOverCol = null;
  }
</script>

<div class="kanban-view">
  {#each columns as col (col.id)}
    {@const colTasks = tasks.filter((t) => t.status === col.id)}
    <div
      class="kanban-col"
      class:drag-over={dragOverCol === col.id}
      role="list"
      ondragover={(e) => {
        e.preventDefault();
        dragOverCol = col.id;
      }}
      ondragleave={() => {
        if (dragOverCol === col.id) dragOverCol = null;
      }}
      ondrop={(e) => {
        e.preventDefault();
        handleDrop(col.id);
      }}
    >
      <div class="col-header">
        <span class="dot" style="background: {col.color}"></span>
        <span class="col-title">{col.label}</span>
        <span class="col-count">{colTasks.length}</span>
      </div>

      <div class="col-cards">
        {#each colTasks as task (task.id)}
          <div
            class="card"
            draggable="true"
            role="listitem"
            ondragstart={() => (draggedTaskId = task.id)}
            ondragend={() => {
              draggedTaskId = null;
              dragOverCol = null;
            }}
          >
            <div class="card-top">
              <span class="card-id">{task.id}</span>
              {#if task.dependsOn !== '-'}
                <span class="dep-badge">↳ {task.dependsOn}</span>
              {/if}
            </div>
            <div class="card-title">{task.title}</div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {task.progress}%"></div>
            </div>
            <div class="card-footer">
              <span class="dates">{task.start} · {task.duration}d</span>
              <button
                type="button"
                class="comment-btn"
                onclick={() => onSelectComments(task.id)}
              >
                <MessageSquare size={12} />
                <span>{task.comments.length}</span>
              </button>
            </div>
          </div>
        {/each}
      </div>

      <button type="button" class="add-card-btn" onclick={() => onAddTask(col.id)}>
        <Plus size={13} /> Añadir
      </button>
    </div>
  {/each}
</div>

<style>
  .kanban-view {
    display: flex;
    gap: 14px;
    padding: 16px;
    height: 100%;
    overflow-x: auto;
    align-items: flex-start;
  }

  .kanban-col {
    min-width: 260px;
    max-width: 280px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 100%;
  }

  .kanban-col.drag-over {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .col-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .col-title {
    flex: 1;
    color: var(--text-primary);
  }

  .col-count {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .col-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }

  .card {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    padding: 10px;
    cursor: grab;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card:hover {
    border-color: var(--accent);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-muted);
  }

  .dep-badge {
    background: var(--bg-tertiary);
    padding: 0 5px;
    border-radius: 4px;
  }

  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .progress-bar {
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--text-muted);
  }

  .comment-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
  }

  .comment-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .add-card-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 6px;
    font-size: 12px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px dashed var(--border-primary);
    border-radius: 6px;
    cursor: pointer;
  }

  .add-card-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
