<script lang="ts">
  import { MessageSquare, Trash2, Plus } from 'lucide-svelte';
  import type { BoardColumn, BoardTask } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    columns: BoardColumn[];
    onUpdateTask: (id: string, patch: Partial<BoardTask>) => void;
    onDeleteTask: (id: string) => void;
    onAddTask: () => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, columns, onUpdateTask, onDeleteTask, onAddTask, onSelectComments }: Props = $props();
</script>

<div class="table-view">
  <table>
    <thead>
      <tr>
        <th class="col-id">ID</th>
        <th>Tarea</th>
        <th>Estado</th>
        <th>Inicio</th>
        <th class="col-num">Duración (d)</th>
        <th>Depende</th>
        <th class="col-num">Progreso %</th>
        <th class="col-actions"></th>
      </tr>
    </thead>
    <tbody>
      {#each tasks as task (task.id)}
        <!-- Los campos editables conservan su doble clic (seleccionar texto) -->
        <tr
          ondblclick={(e) =>
            !(e.target as HTMLElement).closest('input, select, button') && onSelectComments(task.id)}
        >
          <td class="col-id"><code>{task.id}</code></td>
          <td>
            <input
              type="text"
              value={task.title}
              onchange={(e) => onUpdateTask(task.id, { title: e.currentTarget.value })}
            />
          </td>
          <td>
            <select
              value={task.status}
              onchange={(e) => onUpdateTask(task.id, { status: e.currentTarget.value })}
            >
              {#each columns as col (col.id)}
                <option value={col.id}>{col.label}</option>
              {/each}
            </select>
          </td>
          <td>
            <input
              type="date"
              value={task.start}
              onchange={(e) => onUpdateTask(task.id, { start: e.currentTarget.value })}
            />
          </td>
          <td class="col-num">
            <input
              type="number"
              min="1"
              value={task.duration}
              onchange={(e) =>
                onUpdateTask(task.id, { duration: Math.max(1, parseInt(e.currentTarget.value || '1', 10)) })}
            />
          </td>
          <td>
            <select
              value={task.dependsOn}
              onchange={(e) => onUpdateTask(task.id, { dependsOn: e.currentTarget.value })}
            >
              <option value="-">-</option>
              {#each tasks.filter((t) => t.id !== task.id) as other (other.id)}
                <option value={other.id}>{other.id}</option>
              {/each}
            </select>
          </td>
          <td class="col-num">
            <input
              type="number"
              min="0"
              max="100"
              value={task.progress}
              onchange={(e) =>
                onUpdateTask(task.id, {
                  progress: Math.min(100, Math.max(0, parseInt(e.currentTarget.value || '0', 10))),
                })}
            />
          </td>
          <td class="col-actions">
            <button
              type="button"
              class="icon-btn"
              title="Comentarios ({task.comments.length})"
              onclick={() => onSelectComments(task.id)}
            >
              <MessageSquare size={14} />
              {#if task.comments.length > 0}
                <span class="badge">{task.comments.length}</span>
              {/if}
            </button>
            <button
              type="button"
              class="icon-btn danger"
              title="Eliminar"
              onclick={() => onDeleteTask(task.id)}
            >
              <Trash2 size={14} />
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <button type="button" class="add-row-btn" onclick={onAddTask}>
    <Plus size={14} /> Añadir tarea
  </button>
</div>

<style>
  .table-view {
    padding: 12px 16px;
    overflow: auto;
    height: 100%;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th,
  td {
    border: 1px solid var(--border-primary);
    padding: 4px 6px;
    text-align: left;
  }

  th {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-weight: 600;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .col-id {
    width: 64px;
    color: var(--text-muted);
  }

  .col-num {
    width: 96px;
  }

  .col-actions {
    width: 76px;
    white-space: nowrap;
  }

  input,
  select {
    width: 100%;
    background: transparent;
    color: var(--text-primary);
    border: none;
    outline: none;
    font-size: 13px;
    padding: 2px 4px;
    border-radius: 3px;
  }

  input:focus,
  select:focus {
    background: var(--bg-tertiary);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 3px 5px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .icon-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .icon-btn.danger:hover {
    color: #ef4444;
  }

  .badge {
    font-size: 11px;
    color: var(--accent);
  }

  .add-row-btn {
    margin-top: 10px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    cursor: pointer;
  }

  .add-row-btn:hover {
    background: var(--bg-tertiary);
  }
</style>
