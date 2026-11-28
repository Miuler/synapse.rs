import { describe, expect, it } from 'bun:test';
import { newTaskId, parseBoardMarkdown, placeTask, serializeBoardMarkdown, sortTasks, type BoardTask, type TaskSortKey } from './board.svelte';

const task = (id: string, status: string): BoardTask => ({
  id, title: id, status, start: '2026-01-01', duration: 1, dependsOn: '-', progress: 0, comments: [],
});
const tasks = [task('a', 'todo'), task('b', 'doing'), task('c', 'todo'), task('d', 'doing')];
const order = (list: BoardTask[]) => list.map((t) => `${t.id}:${t.status}`).join(' ');

describe('placeTask', () => {
  it('sube y baja dentro de la columna', () => {
    expect(order(placeTask(tasks, 'c', 'todo', 'a'))).toBe('c:todo a:todo b:doing d:doing');
    expect(order(placeTask(tasks, 'a', 'todo', null))).toBe('b:doing c:todo a:todo d:doing');
  });

  it('cambia de columna en la posición indicada o al final', () => {
    expect(order(placeTask(tasks, 'a', 'doing', 'd'))).toBe('b:doing c:todo a:doing d:doing');
    expect(order(placeTask(tasks, 'a', 'doing', null))).toBe('b:doing c:todo d:doing a:doing');
  });

  it('en una columna vacía conserva su fila', () => {
    expect(order(placeTask(tasks, 'b', 'done', null))).toBe('a:todo b:done c:todo d:doing');
  });

  it('si la columna queda igual no toca ninguna fila', () => {
    // Soltar en su sitio no debe reescribir el archivo, aunque haya filas de otras columnas entre medias
    expect(placeTask(tasks, 'a', 'todo', 'c')).toBe(tasks);
    expect(placeTask(tasks, 'c', 'todo', null)).toBe(tasks);
    expect(placeTask(tasks, 'b', 'doing', 'd')).toBe(tasks);
    expect(placeTask(tasks, 'a', 'todo', 'a')).toBe(tasks);
    expect(placeTask(tasks, 'x', 'todo', null)).toBe(tasks);
  });

  it('el orden se guarda y se vuelve a leer', () => {
    const doc = parseBoardMarkdown(serializeBoardMarkdown({
      defaultView: 'kanban', columns: [{ id: 'todo', label: 'T', color: 'red' }], tasks: placeTask(tasks, 'c', 'todo', 'a'),
    }));
    expect(doc.tasks.map((t) => t.id)).toEqual(['c', 'a', 'b', 'd']);
  });
});

describe('anchos de columna', () => {
  const board = (extra: string) => `---
type: board
default_view: kanban
columns:
  - { id: todo, label: "Mía", color: "red" }
${extra}---

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |
|---|---|---|---|---|---|---|
`;

  it('van y vuelven sin tocar las columnas', () => {
    const text = serializeBoardMarkdown(parseBoardMarkdown(board('column_widths:\n  todo: 400\n')));
    expect(text).toContain('column_widths:\n  todo: 400\n');
    expect(text).toContain('- { id: todo, label: "Mía", color: "red" }');
    expect(parseBoardMarkdown(text).columnWidths).toEqual({ todo: 400 });
  });

  it('sin anchos no se escribe la clave', () => {
    expect(serializeBoardMarkdown(parseBoardMarkdown(board('')))).not.toContain('column_widths');
  });

  it('ignora lo que no es un número y limita lo desmesurado', () => {
    const doc = parseBoardMarkdown(board('column_widths: { todo: 9000, a: ancho, b: -3, c: 20 }\n'));
    expect(doc.columnWidths).toEqual({ todo: 720, c: 180 });
    expect(doc.columns.map((c) => c.label)).toEqual(['Mía']);
  });
});

describe('sortTasks', () => {
  const columns = [
    { id: 'todo', label: 'T', color: 'red' },
    { id: 'done', label: 'A', color: 'red' },
  ];
  const list = [
    { ...task('t10', 'done'), title: 'beta', progress: 5, dependsOn: '-' },
    { ...task('t2', 'todo'), title: 'Árbol', progress: 40, dependsOn: 't10' },
    { ...task('t3', 'todo'), title: 'casa', progress: 100, dependsOn: 't2' },
  ];
  const ids = (key: TaskSortKey, descending = false) => sortTasks(list, { key, descending }, columns).map((t) => t.id);

  it('ordena números, texto con tildes e ids como se leen', () => {
    expect(ids('id')).toEqual(['t2', 't3', 't10']);
    expect(ids('title')).toEqual(['t2', 't10', 't3']);
    expect(ids('progress', true)).toEqual(['t3', 't2', 't10']);
  });

  it('el estado sigue el orden de las columnas y conserva el del tablero en los empates', () => {
    expect(ids('status')).toEqual(['t2', 't3', 't10']);
    expect(ids('status', true)).toEqual(['t10', 't2', 't3']);
  });

  it('sin dependencia va al final en los dos sentidos', () => {
    expect(ids('dependsOn')).toEqual(['t3', 't2', 't10']);
    expect(ids('dependsOn', true)).toEqual(['t2', 't3', 't10']);
  });

  it('no cambia el array del tablero', () => {
    ids('title');
    expect(list.map((t) => t.id)).toEqual(['t10', 't2', 't3']);
    expect(sortTasks(list, null, columns)).toBe(list);
  });
});

describe('tablero de tabla', () => {
  it('no gana columnas por los datos que solo guarda el formato de carpeta', () => {
    const doc = parseBoardMarkdown('| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |\n|---|---|---|---|---|---|---|\n');
    const plain = serializeBoardMarkdown({ ...doc, tasks: [task('a', 'todo')] });
    const synced = serializeBoardMarkdown({
      ...doc,
      tasks: [{ ...task('a', 'todo'), type: 'Bug', assignee: 'Ana', key: 'AB#1', url: 'https://example.test/1' }],
    });
    expect(synced).toBe(plain);
  });
});

describe('newTaskId', () => {
  it('genera un UUIDv7 en Crockford Base32 de 26 caracteres ordenable por fecha', () => {
    const a = newTaskId(1_700_000_000_000);
    const b = newTaskId(1_700_000_000_001);
    expect(a).toMatch(/^[0-9a-hjkmnp-tv-z]{26}$/);
    expect(b).toMatch(/^[0-9a-hjkmnp-tv-z]{26}$/);
    expect(a < b).toBe(true);
  });
});
