// La carpeta de un tablero (`Nombre.board/` con su `Nombre.board.md` al lado) solo la escribe el
// tablero. Rust rechaza cualquier otra escritura; aquí se sabe para mostrar el candado.

const SUFFIX = '.board';

/** La carpeta de tareas de un archivo de tablero (`a/X.board.md` → `a/X.board`). */
export function boardFolder(path: string): string | null {
  const folder = path.endsWith('.md') ? path.slice(0, -3) : '';
  const name = folder.slice(folder.lastIndexOf('/') + 1);
  return name.endsWith(SUFFIX) && name.length > SUFFIX.length ? folder : null;
}

/** Las rutas que están en la carpeta de un tablero presente en la lista. */
export function boardLockedPaths(paths: string[]): Set<string> {
  const boards = new Set(paths.filter(boardFolder));
  const locked = new Set<string>();
  if (!boards.size) return locked;
  for (const path of paths) {
    for (let at = path.indexOf('/'); at !== -1; at = path.indexOf('/', at + 1)) {
      if (boards.has(`${path.slice(0, at)}.md`)) {
        locked.add(path);
        break;
      }
    }
  }
  return locked;
}
