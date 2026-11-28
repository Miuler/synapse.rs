import { describe, expect, it } from 'bun:test';
import { applyTask, boardFile, folderDocument, isFolderBoard, persist, taskChanges, type BoardApi, type BoardSnapshot, type TaskCard, type TaskFile } from './folder-board';
import { placeTask } from './board.svelte';
import { buildPushPlan, type IssueProvider } from './issue-sync';

const card = (id: string, title: string, status: string, extra: Partial<TaskCard> = {}): TaskCard => ({
  id, hash: 'h', title, status, type: null, assignee: null, key: null, url: null, tags: [],
  priority: null, severity: null, iteration: null, area: null, parent: null, estimate: null, dueDate: null,
  reason: null, createdBy: null, createdAt: null, changedBy: null, originalEstimate: null,
  remainingWork: null, completedWork: null, boardColumn: null, boardLane: null,
  stateChangedAt: null, activatedAt: null, resolvedAt: null, closedAt: null,
  start: '2026-01-01', duration: 1, dependsOn: '-', progress: 0, sync: null, extra: [], comments: [], ...extra,
});

const snapshot = (): BoardSnapshot => ({
  hash: 'b1',
  problems: [],
  board: {
    defaultView: 'table', columns: [], columnWidths: {}, fields: { card: ['key'] }, extraFrontmatter: 'tags: [a]', body: 'apuntes', order: ['a', 'b'],
    source: { provider: 'p', connection: '', scope: 's', issue_type: 'Bug' },
  },
  tasks: [
    card('a', 'A', 'todo', { type: 'Bug', key: 'AB#1', sync: { id: '1', rev: '3', title: 'A', state: 'New', column: 'todo' } }),
    card('b', 'B', '', { comments: [{ at: 't', kind: 'comment', author: 'x', id: '9', edited: false, text: 'hola' }] }),
  ],
});

describe('formato de carpeta', () => {
  it('se reconoce por su marca', () => {
    expect(isFolderBoard('---\ntype: board\nformat: 2\n---\n')).toBe(true);
    expect(isFolderBoard('---\ntype: board\n---\nformat: 2\n')).toBe(false);
  });

  it('pasa a documento con sus vínculos y vuelve sin perder lo ajeno', () => {
    const doc = folderDocument(snapshot());
    expect(doc.tasks.map((t) => [t.id, t.status, t.type])).toEqual([['a', 'todo', 'Bug'], ['b', 'todo', undefined]]);
    expect(doc.source!.links).toEqual({ a: { id: '1', rev: '3', title: 'A', state: 'New', column: 'todo' } });
    const file = boardFile(snapshot().board, doc);
    expect(file.source).toEqual({ provider: 'p', connection: '', scope: 's', issue_type: 'Bug' });
    expect([file.extraFrontmatter, file.body, file.order]).toEqual(['tags: [a]', 'apuntes', ['a', 'b']]);
  });

  it('solo cambia en el archivo lo que cambió en Synapse', () => {
    const doc = folderDocument(snapshot());
    const edited: TaskFile = { ...applyTask(null, null, { task: doc.tasks[0], link: doc.source!.links!.a }), title: 'Editado fuera', description: 'larga' };
    edited.sync = { ...edited.sync!, rev: '3' };
    const moved = { ...doc.tasks[0], status: 'doing', comments: [{ timestamp: 'n', text: 'nuevo' }] };
    const result = applyTask(edited, { task: doc.tasks[0], link: doc.source!.links!.a }, { task: moved, link: doc.source!.links!.a });
    expect([result.title, result.status, result.description, result.sync!.rev]).toEqual(['Editado fuera', 'doing', 'larga', '3']);
    expect(result.activity.map((a) => a.text)).toEqual(['nuevo']);
  });

  it('guarda solo lo que cambió, las tareas antes que el tablero', async () => {
    const calls: string[] = [];
    const stored = () => ({ task: applyTask(null, null, { task: folderDocument(snapshot()).tasks[0] }), hash: 'h' });
    const api: BoardApi = {
      read: async () => snapshot(),
      write: async (_, board) => (calls.push(`tablero ${board.order.join()}`), 'b2'),
      readTask: async () => stored(),
      writeTask: async (_, id, task, hash) => (calls.push(`tarea ${id} ${task.status} ${hash}`), 'h2'),
      deleteTask: async (_, id) => void calls.push(`borrar ${id}`),
    };
    const file = { board: snapshot().board, hash: 'b1' };
    const doc = folderDocument(snapshot());

    // Reordenar solo toca el tablero
    const reordered = { ...doc, tasks: placeTask(doc.tasks, 'b', 'todo', 'a') };
    expect((await persist(api, 'p', file, doc, reordered)).hash).toBe('b2');
    expect(calls.splice(0)).toEqual(['tablero b,a']);

    // Mover de columna solo la tarea
    const moved = { ...doc, tasks: [{ ...doc.tasks[0], status: 'done' }, doc.tasks[1]] };
    expect(await persist(api, 'p', file, doc, moved)).toBe(file);
    expect(calls.splice(0)).toEqual(['tarea a done h']);

    const replaced = { ...doc, tasks: [doc.tasks[0], { ...doc.tasks[1], id: 'c' }] };
    await persist(api, 'p', file, doc, replaced);
    expect(calls).toEqual(['tarea c todo null', 'borrar b', 'tablero a,c']);
    expect(taskChanges(doc, doc)).toEqual({ created: [], updated: [], removed: [] });
  });

  it('al enviar no repite los comentarios que la tarea ya había enviado', () => {
    const read = snapshot();
    const comment = (at: string) => ({ at, kind: 'comment' as const, author: null, id: null, edited: false, text: at });
    read.tasks = [card('a', 'A', 'todo', {
      comments: [comment('t0'), comment('t1')],
      sync: { id: '1', title: 'A', state: 'New', column: 'todo', sent: ['t0'] },
    })];
    const provider = { id: 'p', default_states: {} } as unknown as IssueProvider;
    const item = { id: '1', type: 'Bug', title: 'A', state: { name: 'New' }, revision: null };
    const plan = buildPushPlan(folderDocument(read), provider, [item]);
    expect(plan.changes.map((c) => c.comments.map((m) => m.timestamp))).toEqual([['t1']]);
  });
});
