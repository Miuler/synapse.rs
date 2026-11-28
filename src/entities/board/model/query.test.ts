import { describe, expect, it } from 'bun:test';
import {
  parseBoardBlock, resolveQuery, resolveTask, taskReference, type BoardQuery, type ReferencedBoard,
} from './query';

type Task = ReferencedBoard['tasks'][number];
const task = (id: string, status: string, extra: Partial<Task> = {}): Task => ({
  id, title: id, status, type: 'Task', assignee: null, key: null, url: null, start: '2026-01-01', duration: 1, progress: 0,
  sync: null, ...extra,
});
const board = (tasks: Task[]): ReferencedBoard => ({
  board: {
    columns: [
      { id: 'todo', label: 'Por hacer', color: '' },
      { id: 'revision', label: 'En revisión', color: '' },
      { id: 'done', label: 'Hecho', color: '' },
    ],
    source: { synced_at: '2026-11-27T10:00:00' },
  },
  tasks,
});
const linked = (state: string, column: string, updated = '') => ({ state, column, updated });
const data = board([
  task('ado-123', 'revision', { type: 'Bug', assignee: 'Ana', key: 'AB#123', sync: linked('In Progress', 'revision', '2026-03-01') }),
  task('ado-124', 'todo', { type: 'Bug', assignee: 'Luis', key: 'AB#124', sync: linked('New', 'todo', '2026-05-01') }),
  task('t1', 'done', { assignee: 'Ana' }),
  task('t2', 'revision', { type: 'Bug', assignee: 'ana' }),
]);

const query = (text: string): BoardQuery => {
  const parsed = parseBoardBlock(`board: Equipo.board\n${text}`);
  if (!parsed.ok) throw new Error(JSON.stringify(parsed.error));
  return parsed.value;
};
const ids = (text: string, from = data) => resolveQuery(from, query(text)).groups.flatMap((g) => g.rows.map((r) => r.id));
const grouped = (text: string) => resolveQuery(data, query(text)).groups.map((g) => [g.label, g.rows.map((r) => r.id)]);
const error = (source: string) => {
  const parsed = parseBoardBlock(source);
  return parsed.ok ? null : parsed.error;
};

describe('bloque `board`', () => {
  it('lee lo mínimo y sus valores por defecto', () => {
    expect(query('')).toEqual({ board: 'Equipo.board', view: 'list', filter: {}, fields: ['key', 'title', 'state'] });
    expect(query('view: table').fields).toEqual(['key', 'title', 'type', 'state', 'assignee']);
  });

  it('lee el ejemplo completo', () => {
    const parsed = query(`view: summary
filter:
  state: [doing, todo]
  type: Bug
  assignee: Ana
  key: [AB#123, AB#124]
group_by: assignee
sort: updated desc
limit: 20
fields: [key, title, state, assignee]`);
    expect(parsed).toMatchObject({
      view: 'summary', groupBy: 'assignee', limit: 20, sort: { field: 'updated', descending: true },
      filter: { state: ['doing', 'todo'], type: ['Bug'], assignee: ['Ana'], key: ['AB#123', 'AB#124'] },
    });
  });

  it('explica lo que está mal y qué se admite', () => {
    expect(error('board: [')?.kind).toBe('yaml');
    expect(error('view: list')?.kind).toBe('missing_board_key');
    expect(error('')?.kind).toBe('missing_board_key');
    expect(error('board: a\nfiltro: {}')).toMatchObject({ kind: 'unknown_key', detail: 'filtro' });
    expect(error('board: a\nfilter: { prioridad: 1 }')).toMatchObject({
      kind: 'unknown_filter', detail: 'prioridad', allowed: ['state', 'type', 'assignee', 'key'],
    });
    for (const wrong of ['view: arbol', 'group_by: title', 'sort: color', 'sort: title up', 'limit: 0', 'limit: dos', 'fields: [nada]', 'fields: []', 'filter: Bug', 'filter: { type: { a: 1 } }']) {
      expect(error(`board: a\n${wrong}`)?.kind).toBe('bad_value');
    }
  });
});

describe('filtros', () => {
  it('por categoría, columna o estado del proveedor', () => {
    expect(ids('filter: { state: doing }')).toEqual(['ado-123', 't2']);
    expect(ids('filter: { state: revision }')).toEqual(['ado-123', 't2']);
    expect(ids('filter: { state: "en REVISIÓN" }')).toEqual(['ado-123', 't2']);
    expect(ids('filter: { state: In Progress }')).toEqual(['ado-123']);
    expect(ids('filter: { state: [done, todo] }')).toEqual(['ado-124', 't1']);
  });

  it('por tipo, asignado y referencia, exigiendo todos a la vez', () => {
    expect(ids('filter: { type: bug }')).toEqual(['ado-123', 'ado-124', 't2']);
    expect(ids('filter: { assignee: Ana }')).toEqual(['ado-123', 't1', 't2']);
    expect(ids('filter: { type: [Bug], state: [doing], assignee: Ana }')).toEqual(['ado-123', 't2']);
    expect(ids('filter: { key: [AB#123, AB#124] }')).toEqual(['ado-123', 'ado-124']);
    expect(ids('filter: { key: t1 }')).toEqual(['t1']);
    expect(ids('filter: { assignee: Nadie }')).toEqual([]);
  });

  it('una tarea movida aquí ya no cuenta con su estado remoto', () => {
    const moved = board([task('a', 'done', { sync: linked('New', 'todo') })]);
    expect(resolveTask(moved, 'a')).toMatchObject({ value: { state: 'Hecho', category: 'done' } });
  });
});

describe('orden, grupos y límite', () => {
  it('sin orden sigue el del tablero; con él, lo vacío va al final', () => {
    expect(ids('')).toEqual(['ado-123', 'ado-124', 't1', 't2']);
    expect(ids('sort: updated desc')).toEqual(['ado-124', 'ado-123', 't1', 't2']);
    expect(ids('sort: updated')).toEqual(['ado-123', 'ado-124', 't1', 't2']);
    expect(ids('sort: state desc')).toEqual(['t1', 'ado-123', 't2', 'ado-124']);
    expect(ids('sort: key\nlimit: 2')).toEqual(['ado-123', 'ado-124']);
  });

  it('agrupa por estado, tipo o asignado', () => {
    expect(grouped('group_by: state')).toEqual([['Por hacer', ['ado-124']], ['En revisión', ['ado-123', 't2']], ['Hecho', ['t1']]]);
    expect(grouped('group_by: type')).toEqual([['Bug', ['ado-123', 'ado-124', 't2']], ['Task', ['t1']]]);
    expect(grouped('group_by: assignee\nfilter: { type: Bug }')).toEqual([['Ana', ['ado-123', 't2']], ['Luis', ['ado-124']]]);
  });

  it('el Kanban enseña todas las columnas, aunque estén vacías', () => {
    expect(grouped('view: kanban\nfilter: { key: t1 }')).toEqual([['Por hacer', []], ['En revisión', []], ['Hecho', ['t1']]]);
  });
});

describe('resumen', () => {
  it('cuenta por categoría y da el avance, sin contar lo cancelado', () => {
    const ten = board([
      ...Array.from({ length: 6 }, (_, i) => task(`h${i}`, 'done')),
      ...Array.from({ length: 4 }, (_, i) => task(`p${i}`, 'todo')),
    ]);
    expect(resolveQuery(ten, query('view: summary\nlimit: 1'))).toMatchObject({
      syncedAt: '2026-11-27T10:00:00',
      summary: { total: 10, progress: 60, counts: { todo: 4, doing: 0, done: 6, cancelled: 0 } },
    });
    const cancelled = board([task('a', 'done'), task('b', 'todo', { sync: linked('Removed', 'todo') })]);
    cancelled.board.columns[0].label = 'Entrada';
    cancelled.board.columns[0].id = 'entrada';
    cancelled.tasks[1] = task('b', 'entrada', { sync: linked('Removed', 'entrada') });
    expect(resolveQuery(cancelled, query('')).summary).toMatchObject({ progress: 100, counts: { cancelled: 1 } });
    expect(resolveQuery(board([]), query('')).summary.progress).toBe(0);
  });
});

describe('referencias', () => {
  it('reconoce el enlace a una tarea', () => {
    expect(taskReference('Equipo.board/ado-123')).toEqual({ board: 'Equipo.board', task: 'ado-123' });
    expect(taskReference('proyectos/Equipo.board/t1.md#Descripción|la tarea')).toEqual({ board: 'proyectos/Equipo.board', task: 't1' });
    for (const other of ['Equipo.board', 'Equipo.board/index', 'Equipo.board/sub/x', 'notas/t1', '.board/t1']) {
      expect(taskReference(other)).toBeNull();
    }
    expect(resolveTask(data, 'ADO-123')).toMatchObject({ ok: true, value: { key: 'AB#123', title: 'ado-123', state: 'In Progress' } });
    expect(resolveTask(data, '123')).toMatchObject({ ok: true, value: { id: 'ado-123', key: 'AB#123' } });
    expect(resolveTask(data, 'AB#123')).toMatchObject({ ok: true, value: { id: 'ado-123', key: 'AB#123' } });
    expect(resolveTask(data, 'nada')).toMatchObject({ ok: false, error: { kind: 'missing_task', detail: 'nada' } });
  });
});
