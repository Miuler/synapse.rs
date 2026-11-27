<script lang="ts">
  import { untrack } from 'svelte';
  import { MessageSquare, Trash2, Plus, ArrowUp, ArrowDown } from 'lucide-svelte';
  import { t } from '@shared/lib/i18n';
  import { sortTasks, type BoardColumn, type BoardTask, type TaskSort, type TaskSortKey } from '../model/board.svelte';

  interface Props {
    tasks: BoardTask[];
    columns: BoardColumn[];
    /** Ids de las tareas fuera del filtro. */
    outside: Set<string>;
    sort: TaskSort | null;
    onSort: (sort: TaskSort | null) => void;
    onUpdateTask: (id: string, patch: Partial<BoardTask>) => void;
    onDeleteTask: (id: string) => void;
    onAddTask: () => void;
    onSelectComments: (id: string) => void;
  }

  let { tasks, columns, outside, sort, onSort, onUpdateTask, onDeleteTask, onAddTask, onSelectComments }: Props = $props();

  const HEADERS: { key: TaskSortKey; label: string; class?: string }[] = [
    { key: 'id', label: 'ID', class: 'col-id' },
    { key: 'title', label: 'Tarea' },
    { key: 'status', label: 'Estado' },
    { key: 'start', label: 'Inicio' },
    { key: 'duration', label: 'Duración (d)', class: 'col-num' },
    { key: 'dependsOn', label: 'Depende' },
    { key: 'progress', label: 'Progreso %', class: 'col-num' },
  ];

  // Se ordena al elegir columna y al añadir o quitar tareas, no mientras se escribe: la fila no salta
  let ids = $derived(tasks.map((task) => task.id).join('\n'));
  let order = $derived.by(() => {
    const by = sort;
    void ids;
    return untrack(() => sortTasks(tasks, by, columns).map((task) => task.id));
  });
  let rows = $derived(order.map((id) => tasks.find((task) => task.id === id)).filter((task) => task !== undefined));

  /** Ascendente, descendente y de vuelta al orden del tablero. */
  function toggleSort(key: TaskSortKey) {
    if (sort?.key !== key) onSort({ key, descending: false });
    else onSort(sort.descending ? null : { key, descending: true });
  }
</script>

<div class="table-view">
  <table>
    <thead>
      <tr>
        {#each HEADERS as header (header.key)}
          {@const active = sort?.key === header.key ? sort : null}
          <th class={header.class} aria-sort={active ? (active.descending ? 'descending' : 'ascending') : 'none'}>
            <button
              type="button"
              class="sort-btn"
              data-sort={header.key}
              title={t('board.sortHint')}
              onclick={() => toggleSort(header.key)}
            >
              {header.label}
              {#if active?.descending}<ArrowDown size={12} />{:else if active}<ArrowUp size={12} />{/if}
            </button>
          </th>
        {/each}
        <th class="col-actions"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as task (task.id)}
        <!-- Los campos editables conservan su doble clic (seleccionar texto) -->
        <tr
          ondblclick={(e) =>
            !(e.target as HTMLElement).closest('input, select, button') && onSelectComments(task.id)}
        >
          <td class="col-id">
            <code>{task.id}</code>
            {#if outside.has(task.id)}<small class="outside-badge">{t('issues.outside')}</small>{/if}
          </td>
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
  .sort-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    font: inherit;
    color: inherit;
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .sort-btn:hover {
    color: var(--text-primary);
  }

  .outside-badge {
    display: block;
    color: var(--warning, #d97706);
  }

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
