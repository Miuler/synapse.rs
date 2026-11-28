import { describe, expect, it } from 'bun:test';
import { boardRefresher, onBoardTaskRequest, requestBoardTask, setBoardRefresher, takeBoardTask } from './actions';

describe('acciones de una referencia', () => {
  it('la tarea pedida espera a que su tablero la tenga y se entrega una vez', () => {
    const asked: string[] = [];
    const stop = onBoardTaskRequest((path) => asked.push(path));
    requestBoardTask('A.board.md', 't1');
    expect(takeBoardTask('A.board.md', [])).toBeNull();
    expect(takeBoardTask('B.board.md', [{ id: 't1' }])).toBeNull();
    expect(takeBoardTask('A.board.md', [{ id: 't1' }])).toBe('t1');
    expect(takeBoardTask('A.board.md', [{ id: 't1' }])).toBeNull();
    stop();
    requestBoardTask('A.board.md', 't2');
    expect(asked).toEqual(['A.board.md']);
  });

  it('actualizar solo existe mientras alguien sabe hacerlo', () => {
    expect(boardRefresher()).toBeNull();
    const first = async () => '';
    const stopFirst = setBoardRefresher(first);
    const second = async () => '';
    const stopSecond = setBoardRefresher(second);
    stopFirst();
    expect(boardRefresher()).toBe(second);
    stopSecond();
    expect(boardRefresher()).toBeNull();
  });
});
