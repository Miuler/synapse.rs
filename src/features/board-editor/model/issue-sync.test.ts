import { describe, expect, it } from 'bun:test';
import { applyPull, buildPushPlan, runPush, type Issue, type IssueLink, type IssueProvider, type IssueWriter } from './issue-sync';
import { parseBoardMarkdown, type BoardDocument, type BoardTask } from './board.svelte';

const provider: IssueProvider = {
  id: 'prueba',
  name: 'Prueba',
  query_language: 'Q',
  default_query: 'todo',
  default_issue_type: 'Task',
  default_states: { todo: ['To Do'], doing: ['In Progress', 'Doing'], done: ['Done'] },
  task_prefix: 'ado',
  scope_example: '',
};

const item = (id: number, title: string, state: string, revision = 1): Issue => ({
  id: String(id),
  type: 'Task',
  title,
  state: { name: state },
  revision: String(revision),
});

const task = (id: string, title: string, status: string): BoardTask => ({
  id,
  title,
  status,
  start: '2026-01-01',
  duration: 1,
  dependsOn: '-',
  progress: 0,
  comments: [],
});

const board = (tasks: BoardTask[], links: Record<string, IssueLink> = {}): BoardDocument => ({
  ...parseBoardMarkdown(''),
  tasks,
  source: { provider: 'prueba', connection: 'prueba', scope: 'https://ejemplo.test/proyecto', links },
});

const link = (id: number, title: string, state: string, column: string): IssueLink => ({ id: String(id), title, state, column });

describe('Traer', () => {
  it('añade lo nuevo y no lo repite', () => {
    const first = applyPull(board([]), provider, [item(1, 'Uno', 'To Do'), item(2, 'Dos', 'Doing')], ['1', '2'], '2026-02-02');
    expect(first.added).toBe(2);
    expect(first.doc.tasks.map((t) => [t.id, t.status])).toEqual([
      ['ado-1', 'todo'],
      ['ado-2', 'doing'],
    ]);
    const again = applyPull(first.doc, provider, [item(1, 'Uno', 'To Do'), item(2, 'Dos', 'Doing')], ['1', '2'], '2026-02-03');
    expect([again.added, again.updated]).toEqual([0, 0]);
  });

  it('actualiza lo que solo cambió allí y respeta lo que solo cambió aquí', () => {
    const doc = board([task('t1', 'Viejo', 'todo'), task('t2', 'Mío', 'done')], {
      t1: link(1, 'Viejo', 'To Do', 'todo'),
      t2: link(2, 'Suyo', 'To Do', 'todo'),
    });
    const result = applyPull(doc, provider, [item(1, 'Nuevo', 'Done'), item(2, 'Suyo', 'To Do')], ['1', '2'], '2026-02-02');
    expect(result.doc.tasks.map((t) => [t.title, t.status])).toEqual([
      ['Nuevo', 'done'],
      ['Mío', 'done'],
    ]);
    expect(result.updated).toBe(1);
    expect(result.conflicts).toEqual([]);
    // Lo local sigue pendiente de enviar
    expect(result.doc.source!.links!.t2).toEqual(link(2, 'Suyo', 'To Do', 'todo'));
  });

  it('si cambió en los dos lados, conserva lo local y avisa', () => {
    const doc = board([task('t1', 'Local', 'doing')], { t1: link(1, 'Base', 'To Do', 'todo') });
    const result = applyPull(doc, provider, [item(1, 'Remoto', 'Done')], ['1'], '2026-02-02');
    expect(result.doc.tasks[0]).toMatchObject({ title: 'Local', status: 'doing' });
    expect(result.conflicts).toEqual(['t1']);
    expect(result.doc.source!.links!.t1).toEqual(link(1, 'Base', 'To Do', 'todo'));
  });

  it('pone cada estado en la columna de su categoría aunque su nombre no se conozca', () => {
    const waiting = (id: number): Issue => ({ ...item(id, 'A', 'En espera'), state: { name: 'En espera', category: 'doing' } });
    // t1 se trajo antes de saberlo y nadie la movió; t2 sí se movió aquí; la 3 es nueva
    const doc = board([task('t1', 'A', 'todo'), task('t2', 'A', 'done')], {
      t1: link(1, 'A', 'En espera', 'todo'),
      t2: link(2, 'A', 'En espera', 'todo'),
    });
    const result = applyPull(doc, provider, [waiting(1), waiting(2), waiting(3)], ['1', '2', '3'], '2026-02-02');
    expect(result.doc.tasks.map((t) => [t.id, t.status])).toEqual([
      ['t1', 'doing'],
      ['t2', 'done'],
      ['ado-3', 'doing'],
    ]);
    expect([result.updated, result.conflicts]).toEqual([1, []]);
    expect(result.doc.source!.links!.t1.column).toBe('doing');
    // Y moverla a la columna que ya le corresponde no pide cambiar nada
    const moved = { ...doc, tasks: [task('t2', 'A', 'doing')] };
    expect(buildPushPlan(moved, provider, [waiting(2)]).changes).toEqual([]);
  });

  it('los estados configurados en el tablero mandan sobre la categoría', () => {
    const doc = board([], {});
    doc.source!.states = { done: ['En espera'] };
    const waiting: Issue = { ...item(1, 'A', 'En espera'), state: { name: 'En espera', category: 'doing' } };
    expect(applyPull(doc, provider, [waiting], ['1'], '2026-02-02').doc.tasks[0].status).toBe('done');
  });

  it('desvincula lo que ya no existe sin borrar la tarea, y no toca vínculos que no se pidieron', () => {
    const doc = board([task('t1', 'A', 'todo'), task('t2', 'B', 'todo')], {
      t1: link(1, 'A', 'To Do', 'todo'),
      t2: link(2, 'B', 'To Do', 'todo'),
    });
    const result = applyPull(doc, provider, [], ['1'], '2026-02-02');
    expect(result.missing).toEqual(['1']);
    expect(result.doc.tasks).toHaveLength(2);
    expect(Object.keys(result.doc.source!.links!)).toEqual(['t2']);
  });
});

describe('Enviar', () => {
  it('solo propone lo que cambió aquí y los comentarios sin enviar', () => {
    const doc = board([task('t1', 'Igual', 'todo'), task('t2', 'Cambiado', 'done'), task('t3', 'Suelta', 'todo')], {
      t1: link(1, 'Igual', 'To Do', 'todo'),
      t2: link(2, 'Antes', 'To Do', 'todo'),
    });
    doc.tasks[1].comments = [{ timestamp: '2026-01-01T10:00:00', text: 'hola' }];
    const plan = buildPushPlan(doc, provider, [item(1, 'Cambiado allí', 'Done'), item(2, 'Antes', 'To Do')]);
    expect(plan.changes).toHaveLength(1);
    expect(plan.changes[0]).toMatchObject({ taskId: 't2', title: 'Cambiado', state: 'Done', conflict: false });
    expect(plan.changes[0].comments).toHaveLength(1);
    expect(plan.unlinked.map((t) => t.id)).toEqual(['t3']);
  });

  it('marca como conflicto lo que cambió en los dos lados', () => {
    const doc = board([task('t1', 'Local', 'todo')], { t1: link(1, 'Base', 'To Do', 'todo') });
    expect(buildPushPlan(doc, provider, [item(1, 'Remoto', 'To Do')]).changes[0].conflict).toBe(true);
  });

  function fake() {
    const calls: string[] = [];
    const writer: IssueWriter = {
      async update(id, revision, changes) {
        calls.push(`update ${id}@${revision} ${JSON.stringify(changes)}`);
        return item(Number(id), changes.title ?? 'Antes', changes.state ?? 'To Do', Number(revision) + 1);
      },
      async create(type, title) {
        calls.push(`create ${type} ${title}`);
        return item(99, title, 'To Do');
      },
      async comment(id, text) {
        if (text === 'falla') throw new Error('sin red');
        calls.push(`comment ${id} ${text}`);
      },
    };
    return { calls, writer };
  }

  it('no repite lo enviado aunque falle a medias, y solo crea lo marcado', async () => {
    let doc = board([task('t1', 'Antes', 'todo'), task('t2', 'Nueva', 'doing'), task('t3', 'Privada', 'todo')], {
      t1: link(1, 'Antes', 'To Do', 'todo'),
    });
    doc.tasks[0].comments = [
      { timestamp: 'a', text: 'bien' },
      { timestamp: 'b', text: 'falla' },
    ];
    const save = (taskId: string, saved: IssueLink) => {
      doc = { ...doc, source: { ...doc.source!, links: { ...doc.source!.links, [taskId]: saved } } };
    };
    const { calls, writer } = fake();
    const plan = buildPushPlan(doc, provider, [item(1, 'Antes', 'To Do')]);
    const result = await runPush(provider, writer, () => doc, save, plan.changes, ['t2']);

    expect(calls).toEqual(['comment 1 bien', 'create Task Nueva', 'update 99@1 {"state":"In Progress"}']);
    expect(result).toMatchObject({ created: 1, comments: 1 });
    expect(result.errors.map((e) => e.taskId)).toEqual(['t1']);
    expect(doc.source!.links!.t1.sent).toEqual(['a']);
    expect(doc.source!.links!.t2).toEqual(link(99, 'Nueva', 'In Progress', 'doing'));
    expect(doc.source!.links!.t3).toBeUndefined();

    const next = buildPushPlan(doc, provider, [item(1, 'Antes', 'To Do'), item(99, 'Nueva', 'In Progress', 2)]);
    expect(next.changes.map((c) => [c.taskId, c.comments.map((x) => x.timestamp)])).toEqual([['t1', ['b']]]);
  });
});
