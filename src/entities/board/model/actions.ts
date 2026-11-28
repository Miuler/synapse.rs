// Lo que una referencia puede pedir a quien sabe hacerlo: actualizar un tablero y abrirlo en una
// tarea. La nota no conoce el editor del tablero ni la sincronización.

type Refresher = (path: string) => Promise<string>;

let refresher: Refresher | null = null;

/** Quien sincroniza tableros se apunta aquí; devuelve cómo borrarse. */
export function setBoardRefresher(next: Refresher): () => void {
  refresher = next;
  return () => {
    if (refresher === next) refresher = null;
  };
}

export const boardRefresher = () => refresher;

const requested = new Map<string, string>();
const listeners = new Set<(path: string) => void>();

/** Al abrirse (o si ya lo está), ese tablero muestra esa tarea. */
export function requestBoardTask(path: string, task: string): void {
  requested.set(path, task);
  for (const listener of listeners) listener(path);
}

/** La tarea pedida para un tablero, una sola vez: cuando ya la tiene entre las suyas. */
export function takeBoardTask(path: string, tasks: { id: string }[]): string | null {
  const id = requested.get(path);
  if (!id || !tasks.some((task) => task.id === id)) return null;
  requested.delete(path);
  return id;
}

export function onBoardTaskRequest(listener: (path: string) => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}
