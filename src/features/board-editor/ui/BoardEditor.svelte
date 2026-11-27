<script lang="ts">
  import { Table, Kanban, GanttChartSquare, FileCode } from 'lucide-svelte';
  import { untrack } from 'svelte';
  import { t } from '@shared/lib/i18n';
  import { CodeEditor } from '@shared/ui/code-editor';
  import {
    parseBoardMarkdown,
    placeTask,
    serializeBoardMarkdown,
    type BoardTask,
    type BoardViewMode,
    type TaskSort,
  } from '../model/board.svelte';
  import { takeNewBoard } from '../model/new-boards';
  import TableView from './TableView.svelte';
  import KanbanView from './KanbanView.svelte';
  import GanttView from './GanttView.svelte';
  import TaskDialog from './TaskDialog.svelte';
  import IssueSync from './IssueSync.svelte';

  interface Props {
    content: string;
    /** Ruta del tablero en la bóveda. */
    path?: string;
    vimMode?: boolean;
    onChange: (nextContent: string) => void;
  }

  let { content, path = '', vimMode = false, onChange }: Props = $props();

  const isNew = untrack(() => takeNewBoard(path));

  let doc = $derived(parseBoardMarkdown(content));
  // «raw» solo se mira: no se guarda como vista del tablero
  let activeView = $state<BoardViewMode | 'raw' | null>(null);
  let currentView = $derived(activeView ?? doc.defaultView);
  // El orden de la tabla es solo para mirar: dura lo que la pestaña
  let tableSort = $state<TaskSort | null>(null);
  let selectedTaskId = $state<string | null>(null);
  /** Las tareas vinculadas que la consulta del tablero ya no devuelve. */
  let outside = $derived(
    new Set(Object.entries(doc.source?.links ?? {}).flatMap(([id, link]) => (link.outside ? [id] : []))),
  );
  let selectedTask = $derived(doc.tasks.find((t) => t.id === selectedTaskId) ?? null);

  function commit(nextTasks: BoardTask[], nextDefaultView: BoardViewMode = doc.defaultView) {
    onChange(serializeBoardMarkdown({ ...doc, defaultView: nextDefaultView, tasks: nextTasks }));
  }

  function updateTask(id: string, patch: Partial<BoardTask>) {
    commit(doc.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)));
  }

  function moveTask(id: string, status: string, before: string | null) {
    const tasks = placeTask(doc.tasks, id, status, before);
    if (tasks !== doc.tasks) commit(tasks);
  }

  /** `null` devuelve la columna a su ancho normal. */
  function resizeColumn(id: string, width: number | null) {
    const { [id]: previous, ...others } = doc.columnWidths ?? {};
    if ((previous ?? null) === width) return;
    write({ ...doc, columnWidths: width === null ? others : { ...others, [id]: width } });
  }

  function write(next: typeof doc) {
    onChange(serializeBoardMarkdown(next));
  }

  function deleteTask(id: string) {
    if (selectedTaskId === id) selectedTaskId = null;
    const tasks = doc.tasks.filter((t) => t.id !== id);
    if (!doc.source?.links?.[id]) return commit(tasks);
    // Sin su vínculo, una tarea nueva con el mismo id no hereda la del proveedor
    const { [id]: _, ...links } = doc.source.links;
    write({ ...doc, tasks, source: { ...doc.source, links } });
  }

  function addTask(status?: string) {
    const nextNum = doc.tasks.length + 1;
    let id = `t${nextNum}`;
    while (doc.tasks.some((t) => t.id === id)) {
      id = `t${nextNum}_${Math.floor(Math.random() * 100)}`;
    }
    const newTask: BoardTask = {
      id,
      title: 'Nueva tarea',
      status: status ?? doc.columns[0]?.id ?? 'todo',
      start: new Date().toISOString().slice(0, 10),
      duration: 2,
      dependsOn: '-',
      progress: 0,
      comments: [],
    };
    commit([...doc.tasks, newTask]);
  }

  function addComment(taskId: string, text: string) {
    const timestamp = new Date().toISOString().slice(0, 19);
    commit(
      doc.tasks.map((t) =>
        t.id === taskId ? { ...t, comments: [...t.comments, { timestamp, text }] } : t
      )
    );
  }
</script>

<div class="board-editor">
  <header class="board-toolbar">
    <div class="view-switcher">
      <button
        type="button"
        class:active={currentView === 'table'}
        onclick={() => (activeView = 'table')}
      >
        <Table size={14} /> Tabla
      </button>
      <button
        type="button"
        class:active={currentView === 'kanban'}
        onclick={() => (activeView = 'kanban')}
      >
        <Kanban size={14} /> Kanban
      </button>
      <button
        type="button"
        class:active={currentView === 'gantt'}
        onclick={() => (activeView = 'gantt')}
      >
        <GanttChartSquare size={14} /> Gantt
      </button>
      <button
        type="button"
        class="view-raw"
        class:active={currentView === 'raw'}
        title={t('board.rawHint')}
        onclick={() => (activeView = 'raw')}
      >
        <FileCode size={14} /> Raw
      </button>
    </div>

    <IssueSync read={() => doc} {write} {isNew} />
  </header>

  <div class="board-body">
    <div class="board-main">
      {#if currentView === 'table'}
        <TableView
          tasks={doc.tasks}
          {outside}
          sort={tableSort}
          onSort={(sort) => (tableSort = sort)}
          columns={doc.columns}
          onUpdateTask={updateTask}
          onDeleteTask={deleteTask}
          onAddTask={() => addTask()}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {:else if currentView === 'kanban'}
        <KanbanView
          tasks={doc.tasks}
          {outside}
          columns={doc.columns}
          widths={doc.columnWidths ?? {}}
          onResizeColumn={resizeColumn}
          onMoveTask={moveTask}
          onOpenTask={(id) => (selectedTaskId = id)}
          onAddTask={(status) => addTask(status)}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {:else if currentView === 'raw'}
        <div class="board-raw">
          <CodeEditor {content} {vimMode} readOnly isActive />
        </div>
      {:else}
        <GanttView
          tasks={doc.tasks}
          onUpdateTask={updateTask}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {/if}
    </div>

    {#if selectedTask}
      <!-- Otra tarea, otro diálogo: el comentario a medio escribir no cambia de tarea -->
      {#key selectedTask.id}
        <TaskDialog
          task={selectedTask}
          tasks={doc.tasks}
          columns={doc.columns}
          link={doc.source?.links?.[selectedTask.id]}
          onClose={() => (selectedTaskId = null)}
          onAddComment={addComment}
        />
      {/key}
    {/if}
  </div>
</div>

<style>
  .board-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .board-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 12px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }

  .view-switcher {
    display: inline-flex;
    gap: 4px;
    background: var(--bg-primary);
    padding: 3px;
    border-radius: 6px;
    border: 1px solid var(--border-primary);
  }

  .view-switcher button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font-size: 12px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 4px;
    cursor: pointer;
  }

  .view-switcher button:hover {
    color: var(--text-primary);
  }

  .view-switcher button.active {
    background: var(--accent);
    color: white;
  }

  .board-body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .board-raw {
    height: 100%;
    overflow: hidden;
  }

  .board-main {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
</style>
