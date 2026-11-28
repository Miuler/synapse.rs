<script lang="ts">
  import { Table, Kanban, GanttChartSquare, FileCode } from 'lucide-svelte';
  import { untrack } from 'svelte';
  import { t } from '@shared/lib/i18n';
  import { CodeEditor } from '@shared/ui/code-editor';
  import { ConfirmDialog } from '@shared/ui/confirm-dialog';
  import { vaultApi } from '@entities/vault-item';
  import { onBoardTaskRequest, takeBoardTask } from '@entities/board';
  import {
    parseBoardMarkdown,
    placeTask,
    serializeBoardMarkdown,
    type BoardTask,
    type BoardViewMode,
    type TaskSort,
  } from '../model/board.svelte';
  import { takeNewBoard } from '../model/new-boards';
  import { boardsApi, type BoardError } from '../api/boards.api';
  import {
    editDescription,
    editEntry,
    isFolderBoard,
    takeProviderDescription,
    type EntryRef,
    type TaskFile,
  } from '../model/folder-board';
  import { FolderBoard } from '../model/folder-board-store.svelte';
  import { DEFAULT_COLUMNS, type BoardDocument } from '../model/board.svelte';
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
    /** Guarda lo pendiente y devuelve lo que quedó en disco; `null` si el archivo está bloqueado. */
    beforeConvert?: () => Promise<string | null>;
    /** Vuelve a leer la pestaña tras convertir el tablero. */
    onConverted?: () => Promise<void>;
  }

  let { content, path = '', vimMode = false, onChange, beforeConvert, onConverted }: Props = $props();

  const isNew = untrack(() => takeNewBoard(path));

  // Un tablero de carpeta (`format: 2`) tiene sus tareas en archivos; los demás, en la tabla del contenido
  let folder = $derived(isFolderBoard(content));
  let store = $state.raw<FolderBoard | null>(null);
  const loading: BoardDocument = { defaultView: 'kanban', columns: DEFAULT_COLUMNS, tasks: [] };
  let doc = $derived(folder ? (store?.doc ?? loading) : parseBoardMarkdown(content));

  $effect(() => {
    if (!folder || !path) return;
    const opened = new FolderBoard(boardsApi, path);
    store = opened;
    void opened.load();
    const tasksFolder = `${path.slice(0, -'.md'.length)}/`;
    const mine = (changed: string) => changed === path || changed.startsWith(tasksFolder);
    const unlisten = vaultApi.onFilesChanged(({ paths, deleted }) => {
      if ([...paths, ...deleted].some(mine)) void opened.load();
    });
    return () => {
      store = null;
      void unlisten.then((stop) => stop());
    };
  });
  // El archivo del tablero cambió en la pestaña (se recargó del disco)
  $effect(() => {
    void content;
    void untrack(() => store)?.load();
  });

  function errorText(error: unknown): string {
    const e = error as Partial<BoardError>;
    return e.kind ? t(`board.errors.${e.kind}`, { detail: e.detail ?? '' }) : String(error);
  }
  let storeError = $derived(store?.error ? errorText(store.error) : '');

  // Un tablero con tabla ofrece pasar a un archivo por tarea
  let confirmingConversion = $state(false);
  let converting = $state(false);
  let conversionError = $state('');
  let fileName = $derived(path.split('/').pop() ?? '');

  async function convert() {
    confirmingConversion = false;
    if (!beforeConvert || !onConverted || converting) return;
    converting = true;
    conversionError = '';
    try {
      const saved = await beforeConvert();
      if (saved === null) conversionError = t('board.convert.unsaved');
      else {
        await boardsApi.convert(path, saved);
        selectedTaskId = null;
        await onConverted();
      }
    } catch (error) {
      conversionError = errorText(error);
    } finally {
      converting = false;
    }
  }
  // «raw» solo se mira: no se guarda como vista del tablero
  let activeView = $state<BoardViewMode | 'raw' | null>(null);
  let currentView = $derived(activeView ?? doc.defaultView);
  // El orden de la tabla es solo para mirar: dura lo que la pestaña
  let tableSort = $state<TaskSort | null>(null);
  let selectedTaskId = $state<string | null>(null);
  // Una nota puede pedir abrir el tablero en una de sus tareas
  let taskRequests = $state(0);
  $effect(() => onBoardTaskRequest((requested) => requested === path && taskRequests++));
  $effect(() => {
    void taskRequests;
    const wanted = takeBoardTask(path, doc.tasks);
    if (wanted) selectedTaskId = wanted;
  });
  /** Las tareas vinculadas que la consulta del tablero ya no devuelve. */
  let outside = $derived(
    new Set(Object.entries(doc.source?.links ?? {}).flatMap(([id, link]) => (link.outside ? [id] : []))),
  );
  let selectedTask = $derived(doc.tasks.find((t) => t.id === selectedTaskId) ?? null);

  function commit(nextTasks: BoardTask[], nextDefaultView: BoardViewMode = doc.defaultView) {
    write({ ...doc, defaultView: nextDefaultView, tasks: nextTasks });
  }

  function updateTask(id: string, patch: Partial<BoardTask>) {
    const tasks = doc.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t));
    const task = doc.tasks.find((t) => t.id === id);
    const link = doc.source?.links?.[id];
    // Al cambiar el tipo o el asignado de una vinculada se anota el anterior: traer no lo pisará
    const edited = (['type', 'assignee'] as const).filter(
      (field) => field in patch && link && link[field] === undefined && (task?.[field] ?? '') !== (patch[field] ?? ''),
    );
    if (!folder || !task || !link || edited.length === 0) return commit(tasks);
    const base = Object.fromEntries(edited.map((field) => [field, task[field] ?? '']));
    write({ ...doc, tasks, source: { ...doc.source!, links: { ...doc.source!.links, [id]: { ...link, ...base } } } });
  }

  /** Guarda un comentario o una entrada del histórico con otro texto. */
  async function saveEntry(id: string, entry: EntryRef, text: string) {
    if (folder) return store?.edit(id, (file) => editEntry(file, entry, text));
    // En la tabla cada comentario es una línea
    const line = text.replace(/\s*\n\s*/g, ' ');
    const task = doc.tasks.find((t) => t.id === id);
    if (!task) return;
    const at = task.comments.findIndex((c) => c.timestamp === entry.at && c.text === entry.text);
    if (at >= 0) updateTask(id, { comments: task.comments.with(at, { ...task.comments[at], text: line }) });
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

  function write(next: BoardDocument) {
    // Mientras se convierte, nada puede volver a escribir la tabla encima
    if (converting) return;
    if (folder) store?.write(next);
    else onChange(serializeBoardMarkdown(next));
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

  /** Lo que solo está en el archivo de la tarea (descripción e histórico). */
  let details = $state.raw<{ id: string; task: TaskFile } | null>(null);
  $effect(() => {
    const id = selectedTaskId;
    const opened = store;
    void doc;
    if (!id || !opened) {
      details = null;
      return;
    }
    let stale = false;
    opened
      .readTask(id)
      .then(({ task }) => !stale && (details = { id, task }))
      .catch(() => !stale && (details = null));
    return () => (stale = true);
  });

  /** Qué datos del proveedor se ven en las tarjetas y en la tabla. */
  const KNOWN_FIELDS = ['key', 'type', 'assignee'] as const;
  let cardFields = $derived(doc.fields?.card?.length ? doc.fields.card : [...KNOWN_FIELDS]);
  let tableFields = $derived(
    doc.fields?.table?.length ? doc.fields.table : KNOWN_FIELDS.filter((field) => doc.tasks.some((task) => task[field])),
  );

  function addComment(taskId: string, text: string) {
    const timestamp = new Date().toISOString().slice(0, 19);
    commit(
      doc.tasks.map((t) =>
        t.id === taskId ? { ...t, comments: [...t.comments, { timestamp, text }] } : t
      )
    );
  }
</script>

<div class="board-editor" inert={converting}>
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

    <IssueSync read={() => doc} {write} {isNew} readTask={folder && store ? (id) => store!.readTask(id) : undefined} />
  </header>

  {#if !folder && path && beforeConvert}
    <div class="board-convert">
      <span>{t('board.convert.banner')}</span>
      <button type="button" disabled={converting} onclick={() => (confirmingConversion = true)}>
        {t('board.convert.action')}
      </button>
    </div>
  {/if}
  {#if conversionError}<p class="board-problem" role="alert">{conversionError}</p>{/if}
  {#if storeError}<p class="board-problem" role="alert">{storeError}</p>{/if}
  {#if folder && store?.problems.length}
    <p class="board-problem" role="status">
      {t('board.unreadable', { files: store.problems.map((p) => `${p.file} (${p.detail})`).join(', ') })}
    </p>
  {/if}

  <div class="board-body">
    <div class="board-main">
      {#if currentView === 'table'}
        <TableView
          tasks={doc.tasks}
          fields={tableFields}
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
          fields={cardFields}
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
          file={details?.id === selectedTask.id ? details.task : undefined}
          {folder}
          {errorText}
          onClose={() => (selectedTaskId = null)}
          onAddComment={addComment}
          onUpdate={(patch) => updateTask(selectedTask!.id, patch)}
          onSaveDescription={(previous, text) =>
            store!.edit(selectedTask!.id, (file) => editDescription(file, previous, text))}
          onSaveEntry={(entry, text) => saveEntry(selectedTask!.id, entry, text)}
          onTakeProviderDescription={(previous) =>
            store!.edit(selectedTask!.id, (file) => takeProviderDescription(file, previous))}
        />
      {/key}
    {/if}
  </div>
</div>

<ConfirmDialog
  open={confirmingConversion}
  title={t('board.convert.title')}
  description={t('board.convert.hint', {
    folder: fileName.slice(0, -'.md'.length),
    backup: `${fileName}.bak`,
    count: doc.tasks.length,
  })}
  confirmText={t('board.convert.confirm')}
  cancelText={t('common.cancel')}
  onConfirm={convert}
  onCancel={() => (confirmingConversion = false)}
/>

<style>
  .board-convert {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 6px 12px;
    padding: 6px 16px;
    font-size: 12px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-primary);
  }

  .board-convert button {
    padding: 4px 10px;
    font-size: 12px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
  }

  .board-problem {
    margin: 0;
    padding: 6px 16px;
    font-size: 12px;
    color: var(--warning, #d97706);
    border-bottom: 1px solid var(--border-primary);
    overflow-wrap: anywhere;
    user-select: text;
  }

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
