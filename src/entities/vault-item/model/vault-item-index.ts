import type { VaultItem } from './types';

/**
 * Lista de elementos de la bóveda con búsqueda por ruta en O(1).
 *
 * Es inmutable: cada cambio devuelve una lista nueva, para que la capa reactiva
 * (`vault-item-store.svelte.ts`) solo tenga que reemplazar la referencia.
 */
export class VaultItemIndex {
  readonly items: readonly VaultItem[];
  private readonly byPath: Map<string, VaultItem>;

  constructor(items: readonly VaultItem[] = []) {
    this.items = items;
    this.byPath = new Map(items.map((item) => [item.relative_path, item]));
  }

  get count(): number {
    return this.items.length;
  }

  get(path: string): VaultItem | undefined {
    return this.byPath.get(path);
  }

  has(path: string): boolean {
    return this.byPath.has(path);
  }

  /** Lista con los campos de un elemento cambiados; la misma lista si la ruta no existe. */
  patch(path: string, patch: Partial<VaultItem>): VaultItemIndex {
    const current = this.byPath.get(path);
    if (!current) return this;
    const updated = { ...current, ...patch };
    return new VaultItemIndex(this.items.map((item) => (item === current ? updated : item)));
  }

  /** Lista con un elemento más al final. */
  add(item: VaultItem): VaultItemIndex {
    return new VaultItemIndex([...this.items, item]);
  }
}
