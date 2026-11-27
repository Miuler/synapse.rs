import { describe, expect, it } from 'bun:test';
import { parseBoardMarkdown, placeTask, serializeBoardMarkdown, type BoardTask } from './board.svelte';

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
