// En la vista en vivo cada referencia se pinta por su cuenta: las que piden a la vez el mismo tablero
// comparten la lectura.

export function sharedReads<T>(read: (path: string) => Promise<T>) {
  const running = new Map<string, Promise<T>>();
  const shared = (path: string): Promise<T> => {
    let current = running.get(path);
    if (!current) {
      current = read(path).finally(() => {
        if (running.get(path) === current) running.delete(path);
      });
      running.set(path, current);
    }
    return current;
  };
  /** Algo cambió: lo que se está leyendo ya no sirve para quien pida a partir de ahora. */
  shared.forget = () => running.clear();
  return shared;
}
