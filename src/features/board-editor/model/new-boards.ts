// Tableros recién creados: al abrirse por primera vez preguntan de dónde salen sus tareas.
const pending = new Set<string>();

export const markNewBoard = (path: string) => void pending.add(path);

/** `true` una sola vez por tablero marcado. */
export const takeNewBoard = (path: string) => pending.delete(path);
