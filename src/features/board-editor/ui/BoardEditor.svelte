<script lang="ts">
  import { Table, Kanban, GanttChartSquare } from 'lucide-svelte';
  import {
    parseBoardMarkdown,
    serializeBoardMarkdown,
    type BoardTask,
    type BoardViewMode,
  } from '../model/board.svelte';
  import TableView from './TableView.svelte';
  import KanbanView from './KanbanView.svelte';
  import GanttView from './GanttView.svelte';
  import CommentsPanel from './CommentsPanel.svelte';

  interface Props {
    content: string;
    onChange: (nextContent: string) => void;
  }

  let { content, onChange }: Props = $props();

  let doc = $derived(parseBoardMarkdown(content));
  let activeView = $state<BoardViewMode | null>(null);
  let currentView = $derived(activeView ?? doc.defaultView);
  let selectedTaskId = $state<string | null>(null);
  let selectedTask = $derived(doc.tasks.find((t) => t.id === selectedTaskId) ?? null);

  function commit(nextTasks: BoardTask[], nextDefaultView: BoardViewMode = doc.defaultView) {
    onChange(
      serializeBoardMarkdown({
        defaultView: nextDefaultView,
        columns: doc.columns,
        tasks: nextTasks,
      })
    );
  }

  function updateTask(id: string, patch: Partial<BoardTask>) {
    commit(doc.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)));
  }

  function deleteTask(id: string) {
    if (selectedTaskId === id) selectedTaskId = null;
    commit(doc.tasks.filter((t) => t.id !== id));
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
    </div>
  </header>

  <div class="board-body">
    <div class="board-main">
      {#if currentView === 'table'}
        <TableView
          tasks={doc.tasks}
          columns={doc.columns}
          onUpdateTask={updateTask}
          onDeleteTask={deleteTask}
          onAddTask={() => addTask()}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {:else if currentView === 'kanban'}
        <KanbanView
          tasks={doc.tasks}
          columns={doc.columns}
          onUpdateTask={updateTask}
          onAddTask={(status) => addTask(status)}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {:else}
        <GanttView
          tasks={doc.tasks}
          onUpdateTask={updateTask}
          onSelectComments={(id) => (selectedTaskId = id)}
        />
      {/if}
    </div>

    {#if selectedTask}
      <CommentsPanel
        task={selectedTask}
        onClose={() => (selectedTaskId = null)}
        onAddComment={addComment}
      />
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

  .board-main {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
</style>
