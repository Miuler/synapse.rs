import type { BoardDocument } from './board.svelte';
import { folderDocument, persist, type BoardApi, type BoardFile, type BoardSnapshot, type TaskFile } from './folder-board';

/**
 * Un tablero de carpeta abierto. Las vistas ven cada cambio al momento y se guarda detrás, en orden;
 * si una escritura falla, se vuelve a leer lo que hay en disco.
 */
export class FolderBoard {
  doc = $state.raw<BoardDocument | null>(null);
  problems = $state.raw<BoardSnapshot['problems']>([]);
  error = $state.raw<unknown>(null);
  #file: { board: BoardFile; hash: string } | null = null;
  #queue: Promise<void> = Promise.resolve();
  #pending = 0;

  constructor(
    private api: BoardApi,
    private path: string,
  ) {}

  /** Lee el tablero cuando no queden escrituras pendientes. */
  load(): Promise<void> {
    return this.#enqueue(() => this.#read());
  }

  write(next: BoardDocument): void {
    const before = this.doc;
    if (!before) return;
    this.doc = next;
    this.error = null;
    void this.#enqueue(async () => {
      if (!this.#file) return;
      this.#file = await persist(this.api, this.path, this.#file, before, next);
      // Lo traído del proveedor solo se ve al releer los archivos
      if (next.tasks.some((task) => task.incoming && !before.tasks.includes(task))) await this.#read();
    });
  }

  /** Cambia lo que de una tarea no está en el documento y relee el tablero. */
  async edit(id: string, change: (task: TaskFile) => TaskFile): Promise<void> {
    let failure: { error: unknown } | null = null;
    await this.#enqueue(async () => {
      try {
        const { task, hash } = await this.api.readTask(this.path, id);
        await this.api.writeTask(this.path, id, change(task), hash);
      } catch (error) {
        // Quien edita lo muestra junto a lo que estaba escribiendo
        failure = { error };
      }
      await this.#read();
    });
    if (failure) throw (failure as { error: unknown }).error;
  }

  /** Cuando no quede nada por guardar. */
  settled(): Promise<void> {
    return this.#queue;
  }

  readTask(id: string) {
    return this.#queue.then(() => this.api.readTask(this.path, id));
  }

  async #read() {
    // Con cambios por guardar, lo de disco aún no los tiene: ya se leerá después
    const snapshot = await this.api.read(this.path);
    this.#file = { board: snapshot.board, hash: snapshot.hash };
    this.problems = snapshot.problems;
    if (this.#pending === 1) this.doc = folderDocument(snapshot);
  }

  #enqueue(task: () => Promise<void>): Promise<void> {
    this.#pending++;
    this.#queue = this.#queue.then(async () => {
      try {
        await task();
      } catch (error) {
        this.error = error;
        // Lo que se veía ya no es lo guardado
        this.#pending = 1;
        await this.#read().catch(() => {});
      } finally {
        this.#pending = Math.max(0, this.#pending - 1);
      }
    });
    return this.#queue;
  }
}
